use crate::ast::{Expression, Instruction, Level};
use crate::register::{
    RegBase, RegKind, ensure_helix_fits, ensure_number_fits, ensure_same_level,
    ensure_supported_level, modrm_imm, modrm_mov_reg_rm, modrm_reg_reg, parse_general_register,
    parse_register, reg_code, seg_code,
};
use std::collections::HashMap;
const STAGE_ONE: isize = 0x7C00;
const STAGE_TWO: isize = 0x7E00;
const NOUN_BASE: u16 = 0x8000;
const KERNEL_CURSOR_ADDR: u32 = 0x9000;
const KERNEL_CUR_PLAN_ADDR: u32 = 0x9004;
const HAPI_BITMAP_ADDR: u32 = 0x9008;
const HAPI_PAGES_ADDR: u32 = 0x900C;
const HAPI_HEAP_ADDR: u32 = 0x9010;
const HAPI_OWNER_ADDR: u32 = 0x9014;
const CAS_DIR_ADDR: u32 = 0x9018;
const CAS_DIR_CAP_ADDR: u32 = 0x901C;
const NOUN_HEADER_SIZE: u16 = 0x30;
const NOUN_TYPE_DATA: u32 = 1;
const NOUN_PERM_RO: u32 = 1;
const STACK_TOP: u32 = 0x0009_FC00;

pub struct Emitter {
    instructions: Vec<Instruction>,
    kbd_layout: String,
    in_kernel: bool,
    pmode_enabled: bool,
    segment_noun: Vec<u8>,
    variables: HashMap<String, u16>,
    dictionnaire_cas: HashMap<blake3::Hash, u16>,
    jump: Vec<JumpPatch>,
    cursor_noun: u16,
    labels: HashMap<String, isize>,
}

#[derive(Clone)]
struct JumpPatch {
    offset: usize,
    cible: Expression,
    kernel: bool,
    size: usize,
}

impl Emitter {
    // On charge l'Émetteur avec l'Arbre Syntaxique (AST)
    pub fn new() -> Self {
        Emitter {
            instructions: Vec::new(),
            kbd_layout: String::new(),
            in_kernel: false,
            pmode_enabled: false,
            segment_noun: Vec::new(),
            variables: HashMap::new(),
            dictionnaire_cas: HashMap::new(),
            jump: Vec::new(),
            cursor_noun: NOUN_BASE,
            labels: HashMap::new(),
        }
    }
    pub fn add_instruction(&mut self, instruction: Vec<Instruction>) -> &mut Self {
        self.instructions.extend(instruction);
        self
    }

    fn emit_mov_reg_reg(&self, code: &mut Vec<u8>, dest: RegBase, src: RegBase) {
        if dest == src {
            return;
        }
        self.emit_op32_prefix(code);
        code.push(0x8B); // MOV reg, r/m
        code.push(modrm_mov_reg_rm(dest, src));
    }

    fn emit_mov_reg_imm32(&self, code: &mut Vec<u8>, dest: RegBase, imm: u32) {
        self.emit_op32_prefix(code);
        code.push(0xB8 + reg_code(dest)); // MOV r32, imm32
        code.extend_from_slice(&imm.to_le_bytes());
    }

    fn emit_rep_movsd_4(&self, code: &mut Vec<u8>) {
        // MOV ECX, 4 ; CLD ; REP MOVSD
        if self.pmode_enabled {
            code.extend_from_slice(&[0xB9, 0x04, 0x00, 0x00, 0x00]);
            code.push(0xFC); // CLD
            code.extend_from_slice(&[0xF3, 0xA5]);
        } else {
            code.extend_from_slice(&[0x66, 0xB9, 0x04, 0x00, 0x00, 0x00]);
            code.push(0xFC); // CLD
            code.extend_from_slice(&[0x66, 0x67, 0xF3, 0xA5]);
        }
    }

    fn emit_op32_prefix(&self, code: &mut Vec<u8>) {
        if !self.pmode_enabled {
            code.push(0x66);
        }
    }

    fn emit_rel16_prefix(&self, code: &mut Vec<u8>) {
        // No-op: rel size is selected by patch size (16-bit in real, 32-bit in pmode)
        let _ = code;
    }

    fn record_jump(&mut self, code: &mut Vec<u8>, cible: &Expression) {
        let size = if self.pmode_enabled { 4 } else { 2 };
        self.jump.push(JumpPatch {
            offset: code.len(),
            cible: cible.clone(),
            kernel: self.in_kernel,
            size,
        });
        if size == 4 {
            code.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        } else {
            code.extend_from_slice(&[0x00, 0x00]);
        }
    }

    fn emit_pmode_prologue(&self, base_addr: isize) -> (Vec<u8>, usize, usize, usize) {
        const CODE_SEL: u16 = 0x08;
        const DATA_SEL: u16 = 0x10;
        const VGA_SEL: u16 = 0x18;

        let mut code = Vec::new();

        // --- Real mode: prepare protected mode ---
        code.push(0xFA); // CLI
        code.extend_from_slice(&[0x31, 0xC0]); // XOR AX, AX
        code.extend_from_slice(&[0x8E, 0xD8]); // MOV DS, AX
        // Enable A20 (fast gate)
        code.extend_from_slice(&[0xE4, 0x92]); // IN AL, 0x92
        code.extend_from_slice(&[0x0C, 0x02]); // OR AL, 0x02
        code.extend_from_slice(&[0xE6, 0x92]); // OUT 0x92, AL

        // LGDT [disp16] (patch later)
        code.extend_from_slice(&[0x0F, 0x01, 0x16, 0x00, 0x00]);
        let lgdt_off = code.len() - 2;

        // CR0.PE = 1
        code.extend_from_slice(&[0x66, 0x0F, 0x20, 0xC0]); // MOV EAX, CR0
        code.extend_from_slice(&[0x66, 0x83, 0xC8, 0x01]); // OR EAX, 1
        code.extend_from_slice(&[0x66, 0x0F, 0x22, 0xC0]); // MOV CR0, EAX

        // Far jump to protected mode entry (patch now, offset is within prologue)
        let far_pos = code.len();
        code.extend_from_slice(&[0x66, 0xEA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let pmode_entry_off = code.len();
        let pmode_entry_addr = base_addr as u32 + pmode_entry_off as u32;
        let offset_bytes = pmode_entry_addr.to_le_bytes();
        code[far_pos + 2..far_pos + 6].copy_from_slice(&offset_bytes);
        code[far_pos + 6..far_pos + 8].copy_from_slice(&CODE_SEL.to_le_bytes());

        // --- Protected mode entry (32-bit code, flat 32-bit data) ---
        code.extend_from_slice(&[0x66, 0xB8, (DATA_SEL & 0xFF) as u8, (DATA_SEL >> 8) as u8]); // MOV AX, DATA_SEL
        code.extend_from_slice(&[0x8E, 0xD8]); // MOV DS, AX
        code.extend_from_slice(&[0x8E, 0xC0]); // MOV ES, AX
        code.extend_from_slice(&[0x8E, 0xD0]); // MOV SS, AX
        code.extend_from_slice(&[0x8E, 0xE0]); // MOV FS, AX
        code.extend_from_slice(&[0x66, 0xB8, (VGA_SEL & 0xFF) as u8, (VGA_SEL >> 8) as u8]); // MOV AX, VGA_SEL
        code.extend_from_slice(&[0x8E, 0xE8]); // MOV GS, AX

        code.push(0xBC); // MOV ESP, imm32
        code.extend_from_slice(&STACK_TOP.to_le_bytes());
        code.push(0xFC); // CLD

        // cursor = 0 (dword)
        code.extend_from_slice(&[0xC7, 0x05]);
        code.extend_from_slice(&KERNEL_CURSOR_ADDR.to_le_bytes());
        code.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // kernel vars init (current plan, hapi bitmap base, hapi pages, hapi heap base)
        for addr in [
            KERNEL_CUR_PLAN_ADDR,
            HAPI_BITMAP_ADDR,
            HAPI_PAGES_ADDR,
            HAPI_HEAP_ADDR,
            HAPI_OWNER_ADDR,
            CAS_DIR_ADDR,
            CAS_DIR_CAP_ADDR,
        ] {
            code.extend_from_slice(&[0xC7, 0x05]);
            code.extend_from_slice(&addr.to_le_bytes());
            code.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        }

        // LIDT [disp16] (patch later)
        code.extend_from_slice(&[0x67, 0x0F, 0x01, 0x1E, 0x00, 0x00]);
        let lidt_off = code.len() - 2;

        (code, lgdt_off, lidt_off, pmode_entry_off)
    }

    fn alloc_helix_literal(&mut self, level: Level, ra: u16, apophis: u16) -> u16 {
        if level != Level::Extreme {
            panic!("Helix literal storage is only supported for Extreme (128) right now.");
        }
        let mut block = vec![0u8; level.bytes() as usize];
        let ra64 = (ra as u64).to_le_bytes();
        let ap64 = (apophis as u64).to_le_bytes();
        block[0..8].copy_from_slice(&ra64);
        block[8..16].copy_from_slice(&ap64);
        self.alloc_noun_object(NOUN_TYPE_DATA, &block, 0)
    }

    fn alloc_xenith_literal(&mut self, ra: u16, apophis: u16) -> u16 {
        let mut block = vec![0u8; Level::Xenith.bytes() as usize];
        let ra64 = (ra as u64).to_le_bytes();
        let ap64 = (apophis as u64).to_le_bytes();
        block[0..8].copy_from_slice(&ra64);
        block[8..16].copy_from_slice(&ap64);
        // Reste du bloc (16..32) = 0
        self.alloc_noun_object(NOUN_TYPE_DATA, &block, 0)
    }

    fn alloc_noun_object(&mut self, obj_type: u32, payload: &[u8], entrypoint: u32) -> u16 {
        let hash = blake3::hash(payload);
        if let Some(addr) = self.dictionnaire_cas.get(&hash) {
            return *addr;
        }
        while self.cursor_noun % 4 != 0 {
            self.segment_noun.push(0);
            self.cursor_noun += 1;
        }
        let header_addr = self.cursor_noun;
        let payload_addr = header_addr + NOUN_HEADER_SIZE;
        let mut header = Vec::with_capacity(NOUN_HEADER_SIZE as usize);
        header.extend_from_slice(&obj_type.to_le_bytes());
        header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        header.extend_from_slice(&NOUN_PERM_RO.to_le_bytes());
        header.extend_from_slice(&entrypoint.to_le_bytes());
        header.extend_from_slice(hash.as_bytes());
        debug_assert_eq!(header.len(), NOUN_HEADER_SIZE as usize);
        self.segment_noun.extend_from_slice(&header);
        self.segment_noun.extend_from_slice(payload);
        self.dictionnaire_cas.insert(hash, payload_addr);
        self.cursor_noun = payload_addr + payload.len() as u16;
        payload_addr
    }
    pub fn kherankh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JLE (Jump if Less or Equal)
        self.emit_rel16_prefix(actual_code);
        actual_code.push(0x0F);
        actual_code.push(0x8E);

        // On enregistre l'emplacement pour le patcher plus tard
        self.record_jump(actual_code, cible);
    }
    pub fn herankh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JGE (Jump if Greater or Equal)
        self.emit_rel16_prefix(actual_code);
        actual_code.push(0x0F);
        actual_code.push(0x8D);

        // On enregistre l'emplacement pour le patcher plus tard
        self.record_jump(actual_code, cible);
    }
    pub fn ankh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JE near (Sauter si Égal, relatif 16-bit)
        self.emit_rel16_prefix(actual_code);
        actual_code.push(0x0F);
        actual_code.push(0x84);

        // On enregistre l'endroit à patcher (comme pour neheh)
        self.record_jump(actual_code, cible);
    }
    pub fn per(&mut self, actual_code: &mut Vec<u8>, message: &Expression) {
        match message {
            Expression::StringLiteral(s) => {
                // 1. On enregistre le texte dans le Noun (payload immuable)
                let mut payload = s.as_bytes().to_vec();
                payload.push(0); // Signe du Silence
                let addr = self.alloc_noun_object(NOUN_TYPE_DATA, &payload, 0);

                // 2. On génère le code machine pour l'afficher
                if self.pmode_enabled {
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                } else {
                    actual_code.push(0xBE); // MOV SI, adresse_du_texte
                    actual_code.extend_from_slice(&addr.to_le_bytes());
                }

                self.emit_rel16_prefix(actual_code);
                actual_code.push(0xE8); // CALL std_print
                let cible = Expression::Identifier("std_print".to_string());
                self.record_jump(actual_code, &cible);
            }
            _ => { /* Gestion des registres si besoin */ }
        }
    }
    pub fn mer(&mut self, actual_code: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level <= Level::High {
            self.emit_op32_prefix(actual_code);
            ensure_supported_level("mer", destination, dest_spec.level);
            match value {
                Expression::Number(n) => {
                    actual_code.push(0x81);
                    actual_code.push(modrm_imm(dest_base, 1));
                    ensure_number_fits("mer", destination, dest_spec.level, *n);
                    actual_code.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Helix { ra, apophis } => {
                    ensure_helix_fits(
                        "mer",
                        destination,
                        dest_spec.level,
                        *ra as u128,
                        *apophis as u128,
                    );
                    let n = ((*ra as i32) << 16) | (*apophis as i32);
                    actual_code.push(0x81);
                    actual_code.push(modrm_imm(dest_base, 1));
                    actual_code.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("mer", destination, dest_spec.level, src, src_spec.level);
                    ensure_supported_level("mer", src, src_spec.level);
                    actual_code.push(0x09); // OR r/m32, r32
                    actual_code.push(modrm_reg_reg(dest_base, src_base));
                }
                _ => panic!("Mer only supports numbers, Helix literals, or registers."),
            }
        } else if dest_spec.level == Level::Extreme {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("mer", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(actual_code, RegBase::Si, src_base);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__helix_or128".to_string());
                    self.record_jump(actual_code, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__helix_or128".to_string());
                    self.record_jump(actual_code, &cible);
                }
                _ => panic!("Mer only supports Helix literals or registers for 128-bit."),
            }
        } else {
            panic!(
                "Mer does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        }
    }
    pub fn henet(&mut self, actual_code: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level <= Level::High {
            self.emit_op32_prefix(actual_code); // Stabilisation 32 bits
            ensure_supported_level("henet", destination, dest_spec.level);
            match value {
                Expression::Number(n) => {
                    actual_code.push(0x81); // Opcode groupe logique
                    actual_code.push(modrm_imm(dest_base, 4));
                    ensure_number_fits("henet", destination, dest_spec.level, *n);
                    actual_code.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Helix { ra, apophis } => {
                    ensure_helix_fits(
                        "henet",
                        destination,
                        dest_spec.level,
                        *ra as u128,
                        *apophis as u128,
                    );
                    let n = ((*ra as i32) << 16) | (*apophis as i32);
                    actual_code.push(0x81);
                    actual_code.push(modrm_imm(dest_base, 4));
                    actual_code.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("henet", destination, dest_spec.level, src, src_spec.level);
                    ensure_supported_level("henet", src, src_spec.level);
                    actual_code.push(0x21); // AND r/m32, r32
                    actual_code.push(modrm_reg_reg(dest_base, src_base));
                }
                _ => panic!("Henet only supports numbers, Helix literals, or registers."),
            }
        } else if dest_spec.level == Level::Extreme {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("henet", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(actual_code, RegBase::Si, src_base);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__helix_and128".to_string());
                    self.record_jump(actual_code, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__helix_and128".to_string());
                    self.record_jump(actual_code, &cible);
                }
                _ => panic!("Henet only supports Helix literals or registers for 128-bit."),
            }
        } else {
            panic!(
                "Henet does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        }
    }
    pub fn kheb(&mut self, actual_code: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level <= Level::High {
            self.emit_op32_prefix(actual_code); // Stabilisation 32 bits
            ensure_supported_level("kheb", destination, dest_spec.level);
            match value {
                Expression::Number(n) => {
                    actual_code.push(0x81); // SUB r/m32, imm32
                    actual_code.push(modrm_imm(dest_base, 5));
                    ensure_number_fits("kheb", destination, dest_spec.level, *n);
                    actual_code.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Helix { ra, apophis } => {
                    ensure_helix_fits(
                        "kheb",
                        destination,
                        dest_spec.level,
                        *ra as u128,
                        *apophis as u128,
                    );
                    let n = ((*ra as i32) << 16) | (*apophis as i32);
                    actual_code.push(0x81);
                    actual_code.push(modrm_imm(dest_base, 5));
                    actual_code.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Register(src) => {
                    // MODE 2 : Soustraire un Registre (SUB r/m32, reg32)
                    actual_code.push(0x29); // OpCode SUB registre à registre

                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("kheb", destination, dest_spec.level, src, src_spec.level);
                    ensure_supported_level("kheb", src, src_spec.level);

                    let modrm = modrm_reg_reg(dest_base, src_base);
                    actual_code.push(modrm);
                }
                _ => panic!("Kheb only supports numbers, Helix literals, or registers."),
            }
        } else if dest_spec.level == Level::Extreme {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("kheb", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(actual_code, RegBase::Si, src_base);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__helix_sub128".to_string());
                    self.record_jump(actual_code, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__helix_sub128".to_string());
                    self.record_jump(actual_code, &cible);
                }
                _ => panic!("Kheb only supports Helix literals or registers for 128-bit."),
            }
        } else if dest_spec.level == Level::Xenith {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("kheb", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(actual_code, RegBase::Si, src_base);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__xenith_sub256".to_string());
                    self.record_jump(actual_code, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_xenith_literal(*ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(actual_code);
                    actual_code.push(0xE8);
                    let cible = Expression::Identifier("__xenith_sub256".to_string());
                    self.record_jump(actual_code, &cible);
                }
                _ => panic!("Kheb only supports Helix literals or registers for 256-bit."),
            }
        } else {
            panic!(
                "Kheb does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        }
    }
    pub fn duat(&mut self, actual_code: &mut Vec<u8>, phrase: &String, address: &u16) {
        for (i, c) in phrase.chars().enumerate() {
            // Opcode 0xC6 0x06 = MOV [imm16], imm8
            actual_code.push(0xC6);
            actual_code.push(0x06);
            let addr_actuelle = address + i as u16;
            actual_code.extend_from_slice(&addr_actuelle.to_le_bytes());
            actual_code.push(c as u8);
        }
        // AJOUT AUTOMATIQUE DU ZÉRO DE FIN
        actual_code.push(0xC6);
        actual_code.push(0x06);
        let addr_zero = address + phrase.len() as u16;
        actual_code.extend_from_slice(&addr_zero.to_le_bytes());
        actual_code.push(0x00);
    }
    pub fn setjem(&mut self, actual_code: &mut Vec<u8>, destination: &String) {
        let dest_spec = parse_general_register(destination);
        ensure_supported_level("sedjem", destination, dest_spec.level);
        if let RegKind::General(RegBase::Ka) = dest_spec.kind {
            if dest_spec.level != Level::Base {
                panic!("Sedjem only supports %ka (Base).");
            }
            // Le Scribe du BIOS :
            // Le BIOS met le CPU en pause, lit les impulsions électriques,
            // les traduit en vrai code ASCII (A, B, C...) et place le résultat dans AL.
            actual_code.extend_from_slice(&[0xB4, 0x00]); // MOV AH, 0x00 (Attendre une touche)
            actual_code.extend_from_slice(&[0xCD, 0x16]); // INT 0x16 (Appel BIOS Clavier)
        }
    }
    pub fn set_kbd_layout(&mut self, layout: String) -> &mut Self {
        self.kbd_layout = layout;
        self
    }
    pub fn set_in_kernel(&mut self, in_kernel: bool) -> &mut Self {
        self.in_kernel = in_kernel;
        self
    }
    pub fn neheh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        self.emit_rel16_prefix(actual_code);
        actual_code.push(0xE9);
        self.record_jump(actual_code, cible);
    }
    pub fn jena(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        actual_code.push(0xE8);
        self.record_jump(actual_code, cible);
    }
    pub fn io_in(&mut self, actual_code: &mut Vec<u8>, port: &Expression) {
        // Lecture matérielle (toujours vers AL - 8 bits)
        match port {
            Expression::Helix { ra, .. } => {
                // IN imm8 ou OUT imm8
                actual_code.push(0xE4); // ou 0xE6 pour le Out
                actual_code.push(*ra as u8); // On ne prend que 8 bits de Ra
            }
            Expression::Register(r) => {
                let reg_spec = parse_general_register(r);
                ensure_supported_level("in", r, reg_spec.level);
                if let RegKind::General(RegBase::Da) = reg_spec.kind {
                    if reg_spec.level != Level::Base {
                        panic!("The IN port must be %da (Base).");
                    }
                    // IN AL, DX (Lit le port contenu dans %da)
                    actual_code.push(0xEC);
                } else {
                    panic!("The IN port must be a number or register %da");
                }
            }
            _ => panic!("The IN port must be a number or register %da"),
        }
    }
    pub fn wab(&mut self, actual_code: &mut Vec<u8>) {
        actual_code.extend_from_slice(&[0xB8, 0x03, 0x00, 0xCD, 0x10]);
    }

    pub fn isfet(&mut self, code_actual: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JNE near (Sauter si Différent, relatif 16-bit)
        self.emit_rel16_prefix(code_actual);
        code_actual.push(0x0F);
        code_actual.push(0x85);

        // On enregistre l'endroit à patcher EXACTEMENT comme pour ankh
        self.record_jump(code_actual, cible);

        // Placeholders (les zéros temporels)&
        code_actual.push(0x00);
        code_actual.push(0x00);
    }
    pub fn kheper(&mut self, code_actual: &mut Vec<u8>, source: &String, adresse: &Expression) {
        // 1. On identifie le code du registre source
        let source_spec = parse_general_register(source);
        let source_base = match source_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };

        if source_spec.level == Level::Extreme {
            // Copier 16 octets depuis l'adresse pointée par le registre source vers la RAM
            self.emit_mov_reg_reg(code_actual, RegBase::Si, source_base);
            match adresse {
                Expression::Helix { ra, .. } => {
                    self.emit_mov_reg_imm32(code_actual, RegBase::Di, *ra as u32);
                }
                Expression::Number(n) => {
                    self.emit_mov_reg_imm32(code_actual, RegBase::Di, *n as u32);
                }
                Expression::Identifier(nom) => {
                    let addr = self
                        .variables
                        .get(nom)
                        .expect(&format!("Variable '{}' introuvable", nom));
                    self.emit_mov_reg_imm32(code_actual, RegBase::Di, *addr as u32);
                }
                Expression::Register(r) => {
                    let ptr_spec = parse_general_register(r);
                    ensure_supported_level("kheper", r, ptr_spec.level);
                    if let RegKind::General(RegBase::Ba) = ptr_spec.kind {
                        self.emit_mov_reg_reg(code_actual, RegBase::Di, RegBase::Ba);
                    } else {
                        panic!("The write address is invalid for kheper.");
                    }
                }
                _ => panic!("L'adresse de destination est invalide pour kheper."),
            }
            self.emit_rep_movsd_4(code_actual);
            return;
        }

        ensure_supported_level("kheper", source, source_spec.level);
        let reg_code: u8 = reg_code(source_base);
        if self.pmode_enabled {
            match adresse {
                Expression::Helix { ra, .. } => {
                    code_actual.push(0x89);
                    code_actual.push(0x05 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*ra as u32).to_le_bytes());
                }
                Expression::Number(n) => {
                    code_actual.push(0x89);
                    code_actual.push(0x05 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*n as u32).to_le_bytes());
                }
                Expression::Identifier(nom) => {
                    let addr = self
                        .variables
                        .get(nom)
                        .expect(&format!("Variable '{}' introuvable", nom));
                    code_actual.push(0x89);
                    code_actual.push(0x05 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*addr as u32).to_le_bytes());
                }
                _ => panic!("L'adresse de destination est invalide pour kheper."),
            }
        } else {
            // Préfixes pour le Mode Réel : 32 bits data, 16 bits addr
            code_actual.push(0x66);
            code_actual.push(0x67);
            match adresse {
                Expression::Helix { ra, .. } => {
                    code_actual.push(0x89); // ou 0x8B
                    code_actual.push(0x06 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*ra as u16).to_le_bytes()); // Seulement 16 bits
                }
                Expression::Identifier(nom) => {
                    // On récupère l'adresse de la variable résolue par Thot
                    let addr = self
                        .variables
                        .get(nom)
                        .expect(&format!("Variable '{}' introuvable", nom));
                    code_actual.push(0x89);
                    code_actual.push(0x06 | (reg_code << 3));
                    code_actual.extend_from_slice(addr.to_le_bytes().as_slice());
                }
                _ => panic!("L'adresse de destination est invalide pour kheper."),
            }
        }
    }
    pub fn push(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        match cible {
            Expression::Register(r) => {
                let reg_spec = parse_general_register(r);
                let reg_base = match reg_spec.kind {
                    RegKind::General(base) => base,
                    _ => unreachable!(),
                };
                if reg_spec.level == Level::Extreme {
                    // SUB ESP, 16
                    if self.pmode_enabled {
                        actual_code.extend_from_slice(&[0x83, 0xEC, 0x10]);
                    } else {
                        actual_code.extend_from_slice(&[0x66, 0x83, 0xEC, 0x10]);
                    }
                    // ESI = source pointer
                    self.emit_mov_reg_reg(actual_code, RegBase::Si, reg_base);
                    // EDI = ESP
                    if self.pmode_enabled {
                        actual_code.extend_from_slice(&[0x8B, 0xFC]);
                    } else {
                        actual_code.extend_from_slice(&[0x66, 0x8B, 0xFC]);
                    }
                    self.emit_rep_movsd_4(actual_code);
                } else if reg_spec.level == Level::Xenith {
                    panic!(
                        "Push does not yet support registers beyond Extreme: %{} ({})",
                        r, reg_spec.level
                    );
                } else {
                    self.emit_op32_prefix(actual_code); // Protection 32-bit
                    // L'OpCode PUSH registre commence à 0x50
                    let opcode = 0x50 + reg_code(reg_base);
                    actual_code.push(opcode);
                }
            }
            Expression::Number(n) => {
                self.emit_op32_prefix(actual_code); // Protection 32-bit
                actual_code.push(0x68); // OpCode PUSH imm32
                actual_code.extend_from_slice(&n.to_le_bytes());
            }
            _ => panic!("Push ne supporte que les registres et les nombres."),
        }
    }
    pub fn sena(&mut self, code_actual: &mut Vec<u8>, destination: &String, adresse: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level == Level::Extreme {
            // Copier 16 octets depuis la RAM vers l'adresse pointée par le registre destination
            self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
            match adresse {
                Expression::Helix { ra, .. } => {
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, *ra as u32);
                }
                Expression::Number(n) => {
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, *n as u32);
                }
                Expression::Identifier(nom) => {
                    let addr = self
                        .variables
                        .get(nom)
                        .expect(&format!("Variable '{}' introuvable", nom));
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, *addr as u32);
                }
                Expression::Register(r) => {
                    let ptr_spec = parse_general_register(r);
                    ensure_supported_level("sena", r, ptr_spec.level);
                    if let RegKind::General(RegBase::Ba) = ptr_spec.kind {
                        self.emit_mov_reg_reg(code_actual, RegBase::Si, RegBase::Ba);
                    } else {
                        panic!("The read address is invalid for Thoth.");
                    }
                }
                _ => panic!("The read address is invalid for Thoth."),
            }
            self.emit_rep_movsd_4(code_actual);
            return;
        }

        ensure_supported_level("sena", destination, dest_spec.level);
        // 1. On identifie le code du registre cible
        let reg_code: u8 = reg_code(dest_base);
        if self.pmode_enabled {
            match adresse {
                Expression::Helix { ra, .. } => {
                    code_actual.push(0x8B);
                    code_actual.push(0x05 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*ra as u32).to_le_bytes());
                }
                Expression::Number(n) => {
                    code_actual.push(0x8B);
                    code_actual.push(0x05 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*n as u32).to_le_bytes());
                }
                Expression::Identifier(nom) => {
                    // On récupère l'adresse de la variable (SLS ou NAMA)
                    let addr = self
                        .variables
                        .get(nom)
                        .expect(&format!("Variable '{}' introuvable", nom));
                    code_actual.push(0x8B);
                    code_actual.push(0x05 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*addr as u32).to_le_bytes());
                }
                Expression::Register(r) => {
                    let ptr_spec = parse_general_register(r);
                    ensure_supported_level("sena", r, ptr_spec.level);
                    if let RegKind::General(RegBase::Ba) = ptr_spec.kind {
                        // Cas particulier : sena %reg, [%ba]
                        code_actual.push(0x8B);
                        code_actual.push(0x07 | (reg_code << 3));
                    } else {
                        panic!("The read address is invalid for Thoth.");
                    }
                }
                _ => panic!("The read address is invalid for Thoth."),
            }
        } else {
            // On prépare le terrain : Mode 32 bits et Adressage 16 bits
            code_actual.push(0x66);
            code_actual.push(0x67);
            match adresse {
                Expression::Helix { ra, .. } => {
                    code_actual.push(0x89); // ou 0x8B
                    code_actual.push(0x06 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*ra as u16).to_le_bytes()); // Seulement 16 bits
                }
                Expression::Identifier(nom) => {
                    // On récupère l'adresse de la variable (SLS ou NAMA)
                    let addr = self
                        .variables
                        .get(nom)
                        .expect(&format!("Variable '{nom}' not found"));
                    code_actual.push(0x8B);
                    code_actual.push(0x06 | (reg_code << 3));
                    code_actual.extend_from_slice(&(*addr as u16).to_le_bytes());
                }
                Expression::Register(r) => {
                    let ptr_spec = parse_general_register(r);
                    ensure_supported_level("sena", r, ptr_spec.level);
                    if let RegKind::General(RegBase::Ba) = ptr_spec.kind {
                        // Cas particulier : sena %reg, [%ba]
                        code_actual.push(0x8B);
                        code_actual.push(0x07 | (reg_code << 3));
                    } else {
                        panic!("The read address is invalid for Thoth.");
                    }
                }
                _ => panic!("The read address is invalid for Thoth."),
            }
        }
    }
    pub fn kherp(&mut self, code_actual: &mut Vec<u8>) {
        let setup_disque = vec![
            0xB8, 0x08, 0x02, // AH=02 (Lecture), AL=08 (On lit 8 secteurs d'un coup !)
            0xBB, 0x00, 0x7E, // Destination en RAM : 0x7E00
            0xB9, 0x02, 0x00, // Commencer au Secteur n°2 du disque
            0xBA, 0x80, 0x00, // Disque dur n°0
            0xCD, 0x13, // Appel BIOS
        ];
        code_actual.extend_from_slice(&setup_disque);
    }
    pub fn pop(&mut self, actual_code: &mut Vec<u8>, destination: &String) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level == Level::Extreme {
            // ESI = ESP
            if self.pmode_enabled {
                actual_code.extend_from_slice(&[0x8B, 0xF4]);
            } else {
                actual_code.extend_from_slice(&[0x66, 0x8B, 0xF4]);
            }
            // EDI = destination pointer
            self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
            self.emit_rep_movsd_4(actual_code);
            // ADD ESP, 16
            if self.pmode_enabled {
                actual_code.extend_from_slice(&[0x83, 0xC4, 0x10]);
            } else {
                actual_code.extend_from_slice(&[0x66, 0x83, 0xC4, 0x10]);
            }
        } else if dest_spec.level == Level::Xenith {
            panic!(
                "Pop does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        } else {
            self.emit_op32_prefix(actual_code); // Protection 32-bit
            // L'OpCode POP registre commence à 0x58
            let opcode = 0x58 + reg_code(dest_base);
            actual_code.push(opcode);
        }
    }
    pub fn io_out(&mut self, actual_code: &mut Vec<u8>, port: &Expression) {
        // Écriture matérielle (toujours depuis AL - 8 bits)
        match port {
            Expression::Helix { ra, .. } => {
                // IN imm8 ou OUT imm8
                actual_code.push(0xE4); // ou 0xE6 pour le Out
                actual_code.push(*ra as u8); // On ne prend que 8 bits de Ra
            }
            Expression::Register(r) => {
                let reg_spec = parse_general_register(r);
                ensure_supported_level("out", r, reg_spec.level);
                if let RegKind::General(RegBase::Da) = reg_spec.kind {
                    if reg_spec.level != Level::Base {
                        panic!("The OUT port must be %da (Base).");
                    }
                    // OUT DX, AL (Écrit vers le port contenu dans %da)
                    actual_code.push(0xEE);
                } else {
                    panic!("The OUT port must be a number or register %da");
                }
            }
            _ => panic!("The OUT port must be a number or register %da"),
        }
    }
    pub fn her(&mut self, code_actual: &mut Vec<u8>, cible: &Expression) {
        self.emit_rel16_prefix(code_actual);
        code_actual.push(0x0F);
        code_actual.push(0x8F); // OpCode pour JG (Saut si plus grand)
        self.record_jump(code_actual, cible);
    }
    pub fn kher(&mut self, code_actual: &mut Vec<u8>, cible: &Expression) {
        self.emit_rel16_prefix(code_actual);
        code_actual.push(0x0F);
        code_actual.push(0x8C); // OpCode pour JL (Saut si plus petit)
        self.record_jump(code_actual, cible);
    }
    fn nama(&mut self, name: &String, value: &Expression) {
        let contenu_brut = match value {
            Expression::Helix { ra, apophis } => {
                let n = ((*ra as i32) << 16) | (*apophis as i32);
                n.to_le_bytes().to_vec()
            }
            Expression::StringLiteral(s) => {
                let mut b = s.as_bytes().to_vec();
                b.push(0); // Signe du Silence
                b
            }
            _ => panic!("Type not supported in the Noun."),
        };
        let adresse = self.alloc_noun_object(NOUN_TYPE_DATA, &contenu_brut, 0);
        self.variables.insert(name.to_string(), adresse);
    }
    fn henek(&mut self, code: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_register(destination);
        match dest_spec.kind {
            RegKind::Segment(seg) => {
                if let Expression::Register(src) = value {
                    let src_spec = parse_general_register(src);
                    if src_spec.level != Level::Base {
                        panic!("Segment moves require base registers: %{src}");
                    }
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_supported_level("henek", src, src_spec.level);
                    code.push(0x8E); // MOV sreg, r/m16
                    code.push(0xC0 | (seg_code(seg) << 3) | reg_code(src_base));
                } else {
                    panic!("Sreg exige a registry.");
                }
            }
            RegKind::General(dest_base) => {
                if dest_spec.level <= Level::High {
                    // Ton code Henek existant pour ka, ib, ba...
                    self.emit_op32_prefix(code);
                    ensure_supported_level("henek", destination, dest_spec.level);
                    match value {
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "henek",
                                destination,
                                dest_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                            // 1. Fusion des deux forces en un seul bloc de 32 bits
                            let n = ((*ra as i32) << 16) | (*apophis as i32);
                            // 2. Le reste ne change pas !
                            code.push(0xB8 + reg_code(dest_base));
                            code.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(src_name) => {
                            let src_spec = parse_general_register(src_name);
                            let src_base = match src_spec.kind {
                                RegKind::General(base) => base,
                                _ => unreachable!(),
                            };
                            ensure_same_level(
                                "henek",
                                destination,
                                dest_spec.level,
                                src_name,
                                src_spec.level,
                            );
                            ensure_supported_level("henek", src_name, src_spec.level);
                            code.push(0x8B);
                            code.push(modrm_reg_reg(dest_base, src_base));
                        }
                        _ => { /* ... identifiant ... */ }
                    }
                } else if dest_spec.level == Level::Extreme {
                    match value {
                        Expression::Register(src_name) => {
                            let src_spec = parse_general_register(src_name);
                            let src_base = match src_spec.kind {
                                RegKind::General(base) => base,
                                _ => unreachable!(),
                            };
                            ensure_same_level(
                                "henek",
                                destination,
                                dest_spec.level,
                                src_name,
                                src_spec.level,
                            );
                            self.emit_mov_reg_reg(code, dest_base, src_base);
                        }
                        Expression::Helix { ra, apophis } => {
                            let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                            self.emit_mov_reg_imm32(code, dest_base, addr as u32);
                        }
                        _ => panic!(
                            "Henek only supports Helix literals or registers for 128-bit registers."
                        ),
                    }
                } else if dest_spec.level == Level::Xenith {
                    match value {
                        Expression::Register(src_name) => {
                            let src_spec = parse_general_register(src_name);
                            let src_base = match src_spec.kind {
                                RegKind::General(base) => base,
                                _ => unreachable!(),
                            };
                            ensure_same_level(
                                "henek",
                                destination,
                                dest_spec.level,
                                src_name,
                                src_spec.level,
                            );
                            self.emit_mov_reg_reg(code, dest_base, src_base);
                        }
                        Expression::Helix { ra, apophis } => {
                            let addr = self.alloc_xenith_literal(*ra, *apophis);
                            self.emit_mov_reg_imm32(code, dest_base, addr as u32);
                        }
                        _ => panic!(
                            "Henek only supports Helix literals or registers for 256-bit registers."
                        ),
                    }
                } else {
                    panic!(
                        "Henek does not yet support registers beyond Extreme: %{} ({})",
                        destination, dest_spec.level
                    );
                }
            }
        }
    }

    pub fn sema(&mut self, code_actual: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level <= Level::High {
            self.emit_op32_prefix(code_actual); // Stabilisation 32 bits
            ensure_supported_level("sema", destination, dest_spec.level);
            match value {
                Expression::Number(n) => {
                    // MODE 1 : Additionner un Nombre (ADD r/m32, imm32)
                    code_actual.push(0x81); // Opcode ADD
                    code_actual.push(modrm_imm(dest_base, 0));
                    ensure_number_fits("sema", destination, dest_spec.level, *n);
                    code_actual.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Helix { ra, apophis } => {
                    ensure_helix_fits(
                        "sema",
                        destination,
                        dest_spec.level,
                        *ra as u128,
                        *apophis as u128,
                    );
                    let n = ((*ra as i32) << 16) | (*apophis as i32);
                    code_actual.push(0x81); // Opcode ADD
                    code_actual.push(modrm_imm(dest_base, 0));
                    code_actual.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Register(src) => {
                    // MODE 2 : Additionner un Registre (ADD r/m32, reg32)
                    code_actual.push(0x01); // Opcode ADD registre à registre

                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("sema", destination, dest_spec.level, src, src_spec.level);
                    ensure_supported_level("sema", src, src_spec.level);

                    // Formule x86 magique (ModR/M) : 0xC0 (11000000 en binaire) + (source * 8) + destination
                    let modrm = modrm_reg_reg(dest_base, src_base);
                    code_actual.push(modrm);
                }
                _ => panic!("Sema ne supporte que les nombres ou les registres."),
            }
        } else if dest_spec.level == Level::Extreme {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("sema", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(code_actual, RegBase::Si, src_base);
                    self.emit_rel16_prefix(code_actual);
                    code_actual.push(0xE8);
                    let cible = Expression::Identifier("__helix_add128".to_string());
                    self.record_jump(code_actual, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(code_actual);
                    code_actual.push(0xE8);
                    let cible = Expression::Identifier("__helix_add128".to_string());
                    self.record_jump(code_actual, &cible);
                }
                _ => panic!("Sema only supports Helix literals or registers for 128-bit."),
            }
        } else if dest_spec.level == Level::Xenith {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("sema", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(code_actual, RegBase::Si, src_base);
                    self.emit_rel16_prefix(code_actual);
                    code_actual.push(0xE8);
                    let cible = Expression::Identifier("__xenith_add256".to_string());
                    self.record_jump(code_actual, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_xenith_literal(*ra, *apophis);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(code_actual);
                    code_actual.push(0xE8);
                    let cible = Expression::Identifier("__xenith_add256".to_string());
                    self.record_jump(code_actual, &cible);
                }
                _ => panic!("Sema only supports Helix literals or registers for 256-bit."),
            }
        } else {
            panic!(
                "Sema does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        }
    }

    pub fn shesa(&mut self, code_actual: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level <= Level::High {
            self.emit_op32_prefix(code_actual); // Stabilisation 32 bits
            ensure_supported_level("shesa", destination, dest_spec.level);
            match value {
                Expression::Number(n) => {
                    let modrm = 0xC0 | (reg_code(dest_base) << 3) | reg_code(dest_base);
                    ensure_number_fits("shesa", destination, dest_spec.level, *n);
                    code_actual.push(0x69); // IMUL r32, r/m32, imm32
                    code_actual.push(modrm);
                    code_actual.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Helix { ra, apophis } => {
                    let modrm = 0xC0 | (reg_code(dest_base) << 3) | reg_code(dest_base);
                    ensure_helix_fits(
                        "shesa",
                        destination,
                        dest_spec.level,
                        *ra as u128,
                        *apophis as u128,
                    );
                    let n = ((*ra as i32) << 16) | (*apophis as i32);
                    code_actual.push(0x69);
                    code_actual.push(modrm);
                    code_actual.extend_from_slice(&n.to_le_bytes());
                }
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("shesa", destination, dest_spec.level, src, src_spec.level);
                    ensure_supported_level("shesa", src, src_spec.level);
                    let modrm = 0xC0 | (reg_code(dest_base) << 3) | reg_code(src_base);
                    code_actual.push(0x0F); // IMUL r32, r/m32
                    code_actual.push(0xAF);
                    code_actual.push(modrm);
                }
                _ => panic!("Shesa only supports numbers, Helix literals, or registers."),
            }
        } else if dest_spec.level == Level::Extreme {
            match value {
                Expression::Register(src) => {
                    let src_spec = parse_general_register(src);
                    let src_base = match src_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    ensure_same_level("shesa", destination, dest_spec.level, src, src_spec.level);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_reg(code_actual, RegBase::Si, src_base);
                    self.emit_rel16_prefix(code_actual);
                    code_actual.push(0xE8);
                    let cible = Expression::Identifier("__helix_mul128".to_string());
                    self.record_jump(code_actual, &cible);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, addr as u32);
                    self.emit_rel16_prefix(code_actual);
                    code_actual.push(0xE8);
                    let cible = Expression::Identifier("__helix_mul128".to_string());
                    self.record_jump(code_actual, &cible);
                }
                _ => panic!("Shesa only supports Helix literals or registers for 128-bit."),
            }
        } else {
            panic!(
                "Shesa does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        }
    }

    // Le grand convertisseur: AST -> Code Machine (Binaire)
    pub fn generer_binaire(&mut self, is_bootloader: bool) -> Vec<u8> {
        let mut stage1_code: Vec<u8> = Vec::new();
        let mut stage2_code: Vec<u8> = Vec::new();
        let mut dans_noyau = false; // Le basculement vers l'infini
        let mut pmode_inserted = false;
        let mut pmode_lgdt_patch: Option<usize> = None;
        let mut pmode_lidt_patch: Option<usize> = None;

        // Le Stage 1 est à 0x7C00, le Stage 2 commence à 0x7E00 (juste après 512 octets)
        let base_stage1 = STAGE_ONE;
        let base_stage2 = STAGE_TWO;
        let instructions = self.instructions.clone();
        for instruction in instructions {
            if let Instruction::Label(ref nom) = instruction {
                if nom == "kernel" || nom == "noyau" {
                    self.set_in_kernel(true);
                    dans_noyau = true;
                }
            }
            let mut actual_code = if dans_noyau {
                &mut stage2_code
            } else {
                &mut stage1_code
            };
            let base_actuelle = if dans_noyau { base_stage2 } else { base_stage1 };
            match instruction {
                Instruction::Neheh { cible } => {
                    self.neheh(&mut actual_code, &cible);
                }
                Instruction::Jena { cible } => {
                    self.jena(&mut actual_code, &cible);
                }
                Instruction::Henek { destination, value } => {
                    self.henek(&mut actual_code, &destination, &value);
                }
                // 2. Unifie le NAMA avec BLAKE3 (SLS Pur)
                Instruction::Nama { name, value } => {
                    self.nama(&name, &value);
                }
                Instruction::Push { cible } => {
                    self.push(&mut actual_code, &cible);
                }
                Instruction::Pop { destination } => {
                    self.pop(&mut actual_code, &destination);
                }
                Instruction::In { port } => {
                    self.io_in(&mut actual_code, &port);
                }
                Instruction::Out { port } => {
                    self.io_out(&mut actual_code, &port);
                }
                Instruction::Her { cible } => {
                    self.her(&mut actual_code, &cible);
                }
                Instruction::Kher { cible } => {
                    self.kher(&mut actual_code, &cible);
                }
                Instruction::Isfet { cible } => {
                    self.isfet(&mut actual_code, &cible);
                }
                Instruction::Kheper { source, adresse } => {
                    self.kheper(&mut actual_code, &source, &adresse);
                }
                Instruction::Rdtsc => {
                    actual_code.push(0x0F);
                    actual_code.push(0x31);
                }
                // Traduction de : sema %registre, valeur (ADD)
                Instruction::Sema { destination, value } => {
                    self.sema(&mut actual_code, &destination, &value);
                }
                // Traduction de : shesa %registre, valeur (MUL)
                Instruction::Shesa { destination, value } => {
                    self.shesa(&mut actual_code, &destination, &value);
                }
                Instruction::Kherp => {
                    self.kherp(&mut actual_code);
                }
                // Traduction de : sena %registre, adresse (MOV reg, [mem])
                Instruction::Sena {
                    destination,
                    adresse,
                } => {
                    self.sena(&mut actual_code, &destination, &adresse);
                }
                Instruction::Sedjem { destination } => {
                    self.setjem(&mut actual_code, &destination);
                }
                Instruction::Henet { destination, value } => {
                    self.henet(&mut actual_code, &destination, &value);
                }
                Instruction::Mer { destination, value } => {
                    self.mer(&mut actual_code, &destination, &value);
                }
                Instruction::Return { resultat } => {
                    match resultat {
                        Expression::Number(n) => {
                            // MOV EAX, n (Opcode 0xB8)
                            self.emit_op32_prefix(actual_code); // LA PROTECTION V4
                            actual_code.push(0xB8);
                            actual_code.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(r) => {
                            let reg_spec = parse_general_register(&r);
                            ensure_supported_level("return", &r, reg_spec.level);
                            if !matches!(reg_spec.kind, RegKind::General(RegBase::Ka))
                                || reg_spec.level != Level::Base
                            {
                                panic!(
                                    "Pour l'instant, le Scribe ne sait renvoyer que des nombres purs ou %ka."
                                );
                            }
                        }
                        _ => panic!("Le Return de Maât ne gère que les nombres pour le moment."),
                    }
                    // Note : Pour l'instant, on ignore 'resultat'. Dans le futur,
                    // on pourra placer 'resultat' dans %ka juste avant de partir !

                    // Le processeur lit la Pile, retrouve son chemin, et reprend son exécution.
                    // OpCode pour RET (Return) : 0xC3
                    actual_code.push(0xC3);
                }
                Instruction::Wab => {
                    self.wab(&mut actual_code);
                }
                Instruction::Per { message } => {
                    self.per(&mut actual_code, &message);
                }
                Instruction::Label(nom) => {
                    // On utilise base_actuelle (0x7C00 ou 0x7E00) au lieu de base_addr !
                    self.labels
                        .insert(nom.clone(), base_actuelle + (actual_code.len() as isize));
                    if (nom == "kernel" || nom == "noyau") && !pmode_inserted {
                        let base_off = actual_code.len();
                        let prologue_base = base_actuelle + base_off as isize;
                        let (prologue, lgdt_off, lidt_off, _pmode_entry_off) =
                            self.emit_pmode_prologue(prologue_base);
                        actual_code.extend_from_slice(&prologue);
                        pmode_lgdt_patch = Some(base_off + lgdt_off);
                        pmode_lidt_patch = Some(base_off + lidt_off);
                        pmode_inserted = true;
                        self.pmode_enabled = true;
                    }
                }
                Instruction::Wdj { left, right } => {
                    let left_spec = parse_general_register(&left);
                    let left_base = match left_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    if left_spec.level <= Level::High {
                        // 1. On force le mode 32 bits pour la précision
                        self.emit_op32_prefix(actual_code);
                        ensure_supported_level("wdj", &left, left_spec.level);
                        // 2. OpCode universel de comparaison : 0x81
                        match right {
                            Expression::Number(n) => {
                                // MODE 1 : Comparer à un Nombre (CMP r/m32, imm32)
                                actual_code.push(0x81); // OpCode universel avec nombre
                                actual_code.push(modrm_imm(left_base, 7));
                                ensure_number_fits("wdj", &left, left_spec.level, n);
                                actual_code.extend_from_slice(&n.to_le_bytes());
                            }
                            Expression::Helix { ra, apophis } => {
                                let n = ((ra as i32) << 16) | (apophis as i32);
                                ensure_helix_fits(
                                    "wdj",
                                    &left,
                                    left_spec.level,
                                    ra as u128,
                                    apophis as u128,
                                );
                                actual_code.push(0x81);
                                actual_code.push(modrm_imm(left_base, 7));
                                actual_code.extend_from_slice(&n.to_le_bytes());
                            }
                            Expression::Register(right_reg) => {
                                // MODE 2 : Comparer à un autre Registre (CMP r/m32, reg32)
                                actual_code.push(0x39); // OpCode pour CMP registre à registre

                                let right_spec = parse_general_register(&right_reg);
                                let right_base = match right_spec.kind {
                                    RegKind::General(base) => base,
                                    _ => unreachable!(),
                                };
                                ensure_same_level(
                                    "wdj",
                                    &left,
                                    left_spec.level,
                                    &right_reg,
                                    right_spec.level,
                                );
                                ensure_supported_level("wdj", &right_reg, right_spec.level);

                                // Formule x86 magique (ModR/M) : 0xC0 (11000000) + (source * 8) + destination
                                let modrm = modrm_reg_reg(left_base, right_base);
                                actual_code.push(modrm);
                            }
                            _ => {
                                panic!("La Balance ne sait peser que des nombres ou des registres.")
                            }
                        }
                    } else if left_spec.level == Level::Extreme {
                        match right {
                            Expression::Register(right_reg) => {
                                let right_spec = parse_general_register(&right_reg);
                                let right_base = match right_spec.kind {
                                    RegKind::General(base) => base,
                                    _ => unreachable!(),
                                };
                                ensure_same_level(
                                    "wdj",
                                    &left,
                                    left_spec.level,
                                    &right_reg,
                                    right_spec.level,
                                );
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Di, left_base);
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Si, right_base);
                                self.emit_rel16_prefix(actual_code);
                                actual_code.push(0xE8);
                                let cible = Expression::Identifier("__helix_cmp128".to_string());
                                self.record_jump(actual_code, &cible);
                            }
                            Expression::Helix { ra, apophis } => {
                                let addr = self.alloc_helix_literal(left_spec.level, ra, apophis);
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Di, left_base);
                                self.emit_mov_reg_imm32(&mut actual_code, RegBase::Si, addr as u32);
                                self.emit_rel16_prefix(actual_code);
                                actual_code.push(0xE8);
                                let cible = Expression::Identifier("__helix_cmp128".to_string());
                                self.record_jump(actual_code, &cible);
                            }
                            _ => {
                                panic!("Wdj only supports Helix literals or registers for 128-bit.")
                            }
                        }
                    } else if left_spec.level == Level::Xenith {
                        match right {
                            Expression::Register(right_reg) => {
                                let right_spec = parse_general_register(&right_reg);
                                let right_base = match right_spec.kind {
                                    RegKind::General(base) => base,
                                    _ => unreachable!(),
                                };
                                ensure_same_level(
                                    "wdj",
                                    &left,
                                    left_spec.level,
                                    &right_reg,
                                    right_spec.level,
                                );
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Di, left_base);
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Si, right_base);
                                self.emit_rel16_prefix(actual_code);
                                actual_code.push(0xE8);
                                let cible = Expression::Identifier("__xenith_cmp256".to_string());
                                self.record_jump(actual_code, &cible);
                            }
                            Expression::Helix { ra, apophis } => {
                                let addr = self.alloc_xenith_literal(ra, apophis);
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Di, left_base);
                                self.emit_mov_reg_imm32(&mut actual_code, RegBase::Si, addr as u32);
                                self.emit_rel16_prefix(actual_code);
                                actual_code.push(0xE8);
                                let cible = Expression::Identifier("__xenith_cmp256".to_string());
                                self.record_jump(actual_code, &cible);
                            }
                            _ => {
                                panic!("Wdj only supports Helix literals or registers for 256-bit.")
                            }
                        }
                    } else {
                        panic!(
                            "Wdj does not yet support registers beyond Extreme: %{} ({})",
                            left, left_spec.level
                        );
                    }
                }
                // Traduction de : ankh cible (Saut Conditionnel : JE)
                Instruction::Ankh { cible } => {
                    self.ankh(&mut actual_code, &cible);
                }
                Instruction::Duat { phrase, address } => {
                    self.duat(&mut actual_code, &phrase, &address);
                }
                // Traduction de : kheb %registre, valeur (SUB)
                Instruction::Kheb { destination, value } => {
                    self.kheb(&mut actual_code, &destination, &value);
                }
                Instruction::HerAnkh { cible } => {
                    self.herankh(&mut actual_code, &cible);
                }
                Instruction::KherAnkh { cible } => {
                    self.kherankh(&mut actual_code, &cible);
                }
                Instruction::Dema { chemin } => {
                    panic!(
                        "Erreur fatale de Maât : L'Émetteur a trouvé une instruction 'dema' pointant vers '{}'. Le Tisserand a oublié de fusionner cette tablette avant la génération du binaire !",
                        chemin
                    );
                }
                Instruction::Smen { .. } => {}
                Instruction::CurrentAddress => {}
            }
        } // Injection de la routine print dans le Stage 2 (pour ne pas saturer le Stage 1)
        // --- Injection UNIQUE de la routine print améliorée ---
        // --- Helix 128 Helpers (always available in Stage2) ---
        self.labels.insert(
            "__helix_add128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_add = vec![
            0x60, // PUSHAD
            // RA (0..7)
            0x8B, 0x07, // MOV EAX, [EDI]
            0x03, 0x06, // ADD EAX, [ESI]
            0x89, 0x07, // MOV [EDI], EAX
            0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x13, 0x46, 0x04, // ADC EAX, [ESI+4]
            0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x73, 0x0A, // JNC +10 (skip RA saturation)
            0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // MOV EAX, 0xFFFFFFFF
            0x89, 0x07, // MOV [EDI], EAX
            0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0xF8, // CLC
            // Apophis (8..15)
            0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x03, 0x46, 0x08, // ADD EAX, [ESI+8]
            0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x13, 0x46, 0x0C, // ADC EAX, [ESI+12]
            0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x73, 0x0B, // JNC +11 (skip Apophis saturation)
            0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // MOV EAX, 0xFFFFFFFF
            0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_add);

        self.labels.insert(
            "__xenith_add256".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let xenith_add = vec![
            0x60, // PUSHAD
            0x8B, 0x07, 0x03, 0x06, 0x89, 0x07, // dword0
            0x8B, 0x47, 0x04, 0x13, 0x46, 0x04, 0x89, 0x47, 0x04, 0x8B, 0x47, 0x08, 0x13, 0x46,
            0x08, 0x89, 0x47, 0x08, 0x8B, 0x47, 0x0C, 0x13, 0x46, 0x0C, 0x89, 0x47, 0x0C, 0x8B,
            0x47, 0x10, 0x13, 0x46, 0x10, 0x89, 0x47, 0x10, 0x8B, 0x47, 0x14, 0x13, 0x46, 0x14,
            0x89, 0x47, 0x14, 0x8B, 0x47, 0x18, 0x13, 0x46, 0x18, 0x89, 0x47, 0x18, 0x8B, 0x47,
            0x1C, 0x13, 0x46, 0x1C, 0x89, 0x47, 0x1C, 0x73, 0x1C, // JNC +28 (skip saturation)
            0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0x89, 0x07, 0x89, 0x47, 0x04, 0x89, 0x47, 0x08, 0x89,
            0x47, 0x0C, 0x89, 0x47, 0x10, 0x89, 0x47, 0x14, 0x89, 0x47, 0x18, 0x89, 0x47, 0x1C,
            0x61, 0xC3,
        ];
        stage2_code.extend_from_slice(&xenith_add);

        self.labels.insert(
            "__xenith_sub256".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let xenith_sub = vec![
            0x60, 0xF8, // PUSHAD, CLC
            0x8B, 0x07, 0x2B, 0x06, 0x89, 0x07, 0x8B, 0x47, 0x04, 0x1B, 0x46, 0x04, 0x89, 0x47,
            0x04, 0x8B, 0x47, 0x08, 0x1B, 0x46, 0x08, 0x89, 0x47, 0x08, 0x8B, 0x47, 0x0C, 0x1B,
            0x46, 0x0C, 0x89, 0x47, 0x0C, 0x73, 0x0D, // JNC +13 (skip RA zero)
            0x31, 0xC0, 0x89, 0x07, 0x89, 0x47, 0x04, 0x89, 0x47, 0x08, 0x89, 0x47, 0x0C,
            0xF8, // CLC
            0x8B, 0x47, 0x10, 0x2B, 0x46, 0x10, 0x89, 0x47, 0x10, 0x8B, 0x47, 0x14, 0x1B, 0x46,
            0x14, 0x89, 0x47, 0x14, 0x8B, 0x47, 0x18, 0x1B, 0x46, 0x18, 0x89, 0x47, 0x18, 0x8B,
            0x47, 0x1C, 0x1B, 0x46, 0x1C, 0x89, 0x47, 0x1C, 0x73,
            0x0E, // JNC +14 (skip Apophis zero)
            0x31, 0xC0, 0x89, 0x47, 0x10, 0x89, 0x47, 0x14, 0x89, 0x47, 0x18, 0x89, 0x47, 0x1C,
            0x61, 0xC3, // POPAD, RET
        ];
        stage2_code.extend_from_slice(&xenith_sub);

        self.labels.insert(
            "__xenith_cmp256".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let xenith_cmp = vec![
            0x60, 0x83, 0xEC, 0x20, 0x89, 0xE5, 0x8B, 0x07, 0x03, 0x46, 0x10, 0x89, 0x45, 0x00,
            0x8B, 0x47, 0x04, 0x13, 0x46, 0x14, 0x89, 0x45, 0x04, 0x8B, 0x47, 0x08, 0x13, 0x46,
            0x18, 0x89, 0x45, 0x08, 0x8B, 0x47, 0x0C, 0x13, 0x46, 0x1C, 0x89, 0x45, 0x0C, 0x0F,
            0x92, 0xC1, 0x8B, 0x06, 0x03, 0x47, 0x10, 0x89, 0x45, 0x10, 0x8B, 0x46, 0x04, 0x13,
            0x47, 0x14, 0x89, 0x45, 0x14, 0x8B, 0x46, 0x08, 0x13, 0x47, 0x18, 0x89, 0x45, 0x18,
            0x8B, 0x46, 0x0C, 0x13, 0x47, 0x1C, 0x89, 0x45, 0x1C, 0x0F, 0x92, 0xC5, 0x38, 0xE9,
            0x75, 0x26, 0x8B, 0x45, 0x0C, 0x8B, 0x55, 0x1C, 0x39, 0xD0, 0x75, 0x1C, 0x8B, 0x45,
            0x08, 0x8B, 0x55, 0x18, 0x39, 0xD0, 0x75, 0x12, 0x8B, 0x45, 0x04, 0x8B, 0x55, 0x14,
            0x39, 0xD0, 0x75, 0x08, 0x8B, 0x45, 0x00, 0x8B, 0x55, 0x10, 0x39, 0xD0, 0x8D, 0x65,
            0x20, 0x61, 0xC3,
        ];
        stage2_code.extend_from_slice(&xenith_cmp);

        self.labels.insert(
            "__hapi_init".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let hapi_init = vec![
            0xA3, 0x08, 0x90, 0x00, 0x00, 0x89, 0x0D, 0x0C, 0x90, 0x00, 0x00, 0x89,
            0xCA, 0x83, 0xC2, 0x07, 0xC1, 0xEA, 0x03, 0x89, 0xC3, 0x01, 0xD3,
            0x83, 0xC3, 0x03, 0x83, 0xE3, 0xFC, 0x89, 0x1D, 0x14, 0x90, 0x00, 0x00,
            0x89, 0xCE, 0xC1, 0xE6, 0x02, 0x01, 0xF3, 0x81, 0xC3, 0xFF, 0x0F, 0x00, 0x00,
            0x81, 0xE3, 0x00, 0xF0, 0xFF, 0xFF, 0x89, 0x1D, 0x10, 0x90, 0x00, 0x00, 0x89,
            0xC7, 0x31, 0xC0, 0x89, 0xD1, 0xF3, 0xAA, 0x8B, 0x3D, 0x14, 0x90, 0x00, 0x00,
            0x8B, 0x0D, 0x0C, 0x90, 0x00, 0x00, 0x31, 0xC0, 0xF3, 0xAB, 0xC3,
        ];
        stage2_code.extend_from_slice(&hapi_init);

        self.labels.insert(
            "__hapi_alloc".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let hapi_alloc = vec![
            0x8B, 0x35, 0x08, 0x90, 0x00, 0x00, 0x8B, 0x2D, 0x0C, 0x90, 0x00,
            0x00, 0x8B, 0x1D, 0x10, 0x90, 0x00, 0x00, 0x31, 0xFF, 0x39, 0xEF,
            0x73, 0x33, 0x89, 0xF8, 0xC1, 0xE8, 0x03, 0x89, 0xF9, 0x83, 0xE1,
            0x07, 0xB2, 0x01, 0xD2, 0xE2, 0x8A, 0x34, 0x06, 0x84, 0xD6, 0x75,
            0x1B, 0x08, 0xD6, 0x88, 0x34, 0x06, 0xA1, 0x04, 0x90, 0x00, 0x00,
            0x8B, 0x15, 0x14, 0x90, 0x00, 0x00, 0x89, 0x04, 0xBA, 0x89, 0xF8,
            0xC1, 0xE0, 0x0C, 0x01, 0xD8, 0xC3, 0x47, 0xEB, 0xC9, 0x31, 0xC0,
            0xC3,
        ];
        stage2_code.extend_from_slice(&hapi_alloc);

        self.labels.insert(
            "__hapi_free".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let hapi_free = vec![
            0x8B, 0x1D, 0x10, 0x90, 0x00, 0x00, 0x39, 0xD8, 0x72, 0x43, 0x29,
            0xD8, 0xC1, 0xE8, 0x0C, 0x8B, 0x35, 0x08, 0x90, 0x00, 0x00, 0x8B,
            0x2D, 0x14, 0x90, 0x00, 0x00, 0x8B, 0x15, 0x04, 0x90, 0x00, 0x00,
            0x85, 0xD2, 0x74, 0x08, 0x8B, 0x4C, 0x85, 0x00, 0x39, 0xD1, 0x75,
            0x20, 0xC7, 0x44, 0x85, 0x00, 0x00, 0x00, 0x00, 0x00, 0x89, 0xC1,
            0xC1, 0xE9, 0x03, 0x83, 0xE0, 0x07, 0xB2, 0x01, 0x88, 0xC1, 0xD2,
            0xE2, 0xF6, 0xD2, 0x8A, 0x34, 0x0E, 0x20, 0xD6, 0x88, 0x34, 0x0E,
            0xC3,
        ];
        stage2_code.extend_from_slice(&hapi_free);

        self.labels.insert(
            "__hapi_transfer".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let hapi_transfer = vec![
            0x8B, 0x1D, 0x10, 0x90, 0x00, 0x00, 0x39, 0xD8, 0x72, 0x21, 0x29,
            0xD8, 0xC1, 0xE8, 0x0C, 0x8B, 0x2D, 0x14, 0x90, 0x00, 0x00, 0x8B,
            0x15, 0x04, 0x90, 0x00, 0x00, 0x85, 0xD2, 0x74, 0x08, 0x8B, 0x4C,
            0x85, 0x00, 0x39, 0xD1, 0x75, 0x04, 0x89, 0x7C, 0x85, 0x00, 0xC3,
        ];
        stage2_code.extend_from_slice(&hapi_transfer);

        self.labels.insert(
            "__cas_init".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let cas_init = vec![
            0xA3, 0x18, 0x90, 0x00, 0x00, 0x89, 0x0D, 0x1C, 0x90, 0x00, 0x00,
            0x89, 0xCA, 0xC1, 0xE2, 0x05, 0x89, 0xCB, 0xC1, 0xE3, 0x03, 0x01,
            0xDA, 0x89, 0xC7, 0x31, 0xC0, 0x89, 0xD1, 0xF3, 0xAA, 0xC3,
        ];
        stage2_code.extend_from_slice(&cas_init);

        self.labels.insert(
            "__cas_get".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let cas_get = vec![
            0x8B, 0x1D, 0x18, 0x90, 0x00, 0x00, 0x85, 0xDB, 0x74, 0x48, 0x8B,
            0x0D, 0x1C, 0x90, 0x00, 0x00, 0x85, 0xC9, 0x74, 0x3E, 0x89, 0xCA,
            0x4A, 0x8B, 0x06, 0x21, 0xD0, 0x89, 0xC7, 0x89, 0xF5, 0x83, 0xF9,
            0x00, 0x74, 0x2E, 0x89, 0xF8, 0xC1, 0xE0, 0x03, 0x8D, 0x04, 0x80,
            0x8D, 0x04, 0x03, 0x83, 0x38, 0x00, 0x74, 0x1E, 0x51, 0x57, 0x50,
            0x89, 0xEE, 0x89, 0xC7, 0xB9, 0x20, 0x00, 0x00, 0x00, 0xFC, 0xF3,
            0xA6, 0x58, 0x5F, 0x59, 0x74, 0x06, 0x47, 0x21, 0xD7, 0x49, 0xEB,
            0xD1, 0x8B, 0x40, 0x20, 0xC3, 0x31, 0xC0, 0xC3,
        ];
        stage2_code.extend_from_slice(&cas_get);

        self.labels.insert(
            "__cas_put".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let cas_put = vec![
            0x57, 0x51, 0x8B, 0x1D, 0x18, 0x90, 0x00, 0x00, 0x85, 0xDB, 0x74,
            0x70, 0x8B, 0x0D, 0x1C, 0x90, 0x00, 0x00, 0x85, 0xC9, 0x74, 0x66,
            0x89, 0xCA, 0x4A, 0x8B, 0x06, 0x21, 0xD0, 0x89, 0xC7, 0x89, 0xF5,
            0x83, 0xF9, 0x00, 0x74, 0x56, 0x89, 0xF8, 0xC1, 0xE0, 0x03, 0x8D,
            0x04, 0x80, 0x8D, 0x04, 0x03, 0x83, 0x38, 0x00, 0x74, 0x1A, 0x51,
            0x57, 0x50, 0x89, 0xEE, 0x89, 0xC7, 0xB9, 0x20, 0x00, 0x00, 0x00,
            0xFC, 0xF3, 0xA6, 0x58, 0x5F, 0x59, 0x74, 0x2B, 0x47, 0x21, 0xD7,
            0x49, 0xEB, 0xD1, 0x51, 0x57, 0x50, 0x89, 0xEE, 0x89, 0xC7, 0xB9,
            0x20, 0x00, 0x00, 0x00, 0xFC, 0xF3, 0xA4, 0x58, 0x5F, 0x59, 0x8B,
            0x14, 0x24, 0x8B, 0x74, 0x24, 0x04, 0x89, 0x70, 0x20, 0x89, 0x50,
            0x24, 0x89, 0xF0, 0x83, 0xC4, 0x08, 0xC3, 0x8B, 0x40, 0x20, 0x83,
            0xC4, 0x08, 0xC3, 0x83, 0xC4, 0x08, 0x31, 0xC0, 0xC3,
        ];
        stage2_code.extend_from_slice(&cas_put);

        self.labels.insert(
            "__cas_hash_eq".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let cas_hash_eq = vec![
            0xB9, 0x20, 0x00, 0x00, 0x00, 0xFC, 0xF3, 0xA6, 0x31, 0xC0, 0x75,
            0x05, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3,
        ];
        stage2_code.extend_from_slice(&cas_hash_eq);

        self.labels.insert(
            "__helix_sub128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_sub = vec![
            0x60, 0xF8, // PUSHAD, CLC
            // RA (0..7)
            0x8B, 0x07, // MOV EAX, [EDI]
            0x2B, 0x06, // SUB EAX, [ESI]
            0x89, 0x07, // MOV [EDI], EAX
            0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x1B, 0x46, 0x04, // SBB EAX, [ESI+4]
            0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x73, 0x07, // JNC +7 (skip RA zero)
            0x31, 0xC0, // XOR EAX, EAX
            0x89, 0x07, // MOV [EDI], EAX
            0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0xF8, // CLC
            // Apophis (8..15)
            0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x2B, 0x46, 0x08, // SUB EAX, [ESI+8]
            0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x1B, 0x46, 0x0C, // SBB EAX, [ESI+12]
            0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x73, 0x08, // JNC +8 (skip Apophis zero)
            0x31, 0xC0, // XOR EAX, EAX
            0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x61, 0xC3, // POPAD, RET
        ];
        stage2_code.extend_from_slice(&helix_sub);

        self.labels.insert(
            "__helix_mul128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_mul = vec![
            0x60, // PUSHAD
            // RA (0..7)
            0x31, 0xED, 0x8B, 0x07, 0xF7, 0x26, 0x89, 0xC3, 0x89, 0xD1, 0x8B, 0x07, 0xF7, 0x66,
            0x04, 0x0B, 0xEA, 0x50, 0x8B, 0x47, 0x04, 0xF7, 0x26, 0x0B, 0xEA, 0x50, 0x8B, 0x47,
            0x04, 0xF7, 0x66, 0x04, 0x0B, 0xEA, 0x0B, 0xE8, 0x5A, 0x58, 0x01, 0xC1, 0x83, 0xD5,
            0x00, 0x01, 0xD1, 0x83, 0xD5, 0x00, 0x85, 0xED, 0x75, 0x07, 0x89, 0x1F, 0x89, 0x4F,
            0x04, 0xEB, 0x0A, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0x89, 0x07, 0x89, 0x47, 0x04,
            // Apophis (8..15)
            0x31, 0xED, 0x8B, 0x47, 0x08, 0xF7, 0x66, 0x08, 0x89, 0xC3, 0x89, 0xD1, 0x8B, 0x47,
            0x08, 0xF7, 0x66, 0x0C, 0x0B, 0xEA, 0x50, 0x8B, 0x47, 0x0C, 0xF7, 0x66, 0x08, 0x0B,
            0xEA, 0x50, 0x8B, 0x47, 0x0C, 0xF7, 0x66, 0x0C, 0x0B, 0xEA, 0x0B, 0xE8, 0x5A, 0x58,
            0x01, 0xC1, 0x83, 0xD5, 0x00, 0x01, 0xD1, 0x83, 0xD5, 0x00, 0x85, 0xED, 0x75, 0x08,
            0x89, 0x5F, 0x08, 0x89, 0x4F, 0x0C, 0xEB, 0x0B, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0x89,
            0x47, 0x08, 0x89, 0x47, 0x0C, 0x61, 0xC3,
        ];
        stage2_code.extend_from_slice(&helix_mul);

        self.labels.insert(
            "__helix_and128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_and = vec![
            0x60, // PUSHAD
            0x8B, 0x07, 0x23, 0x06, 0x89, 0x07, 0x8B, 0x47, 0x04, 0x23, 0x46, 0x04, 0x89, 0x47,
            0x04, 0x8B, 0x47, 0x08, 0x23, 0x46, 0x08, 0x89, 0x47, 0x08, 0x8B, 0x47, 0x0C, 0x23,
            0x46, 0x0C, 0x89, 0x47, 0x0C, 0x61, 0xC3,
        ];
        stage2_code.extend_from_slice(&helix_and);

        self.labels.insert(
            "__helix_or128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_or = vec![
            0x60, // PUSHAD
            0x8B, 0x07, 0x0B, 0x06, 0x89, 0x07, 0x8B, 0x47, 0x04, 0x0B, 0x46, 0x04, 0x89, 0x47,
            0x04, 0x8B, 0x47, 0x08, 0x0B, 0x46, 0x08, 0x89, 0x47, 0x08, 0x8B, 0x47, 0x0C, 0x0B,
            0x46, 0x0C, 0x89, 0x47, 0x0C, 0x61, 0xC3,
        ];
        stage2_code.extend_from_slice(&helix_or);

        self.labels.insert(
            "__helix_cmp128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_cmp = vec![
            0x60, // PUSHAD
            0x8B, 0x07, 0x03, 0x46, 0x08, 0x8B, 0x5F, 0x04, 0x13, 0x5E, 0x0C, 0x0F, 0x92, 0xC1,
            0x8B, 0x16, 0x03, 0x57, 0x08, 0x8B, 0x6E, 0x04, 0x13, 0x6F, 0x0C, 0x0F, 0x92, 0xC5,
            0x38, 0xE9, 0x75, 0x08, 0x39, 0xEB, 0x75, 0x03, 0x39, 0xD0, 0x61, 0xC3,
        ];
        stage2_code.extend_from_slice(&helix_cmp);

        if is_bootloader {
            self.labels.insert(
                "std_print".to_string(),
                base_stage2 + (stage2_code.len() as isize),
            );

            // VGA texte direct (GS = 0xB8000, cursor @ 0x9000)
            let routine_print = vec![
                0x50, 0x57, 0xFC, // PUSH EAX, PUSH EDI, CLD
                0xAC, // LODSB
                0x08, 0xC0, // OR AL, AL
                0x74, 0x16, // JZ end_print
                0x8B, 0x3D, 0x00, 0x90, 0x00, 0x00, // MOV EDI, [0x9000]
                0xD1, 0xE7, // SHL EDI, 1
                0xB4, 0x0F, // MOV AH, 0x0F
                0x65, 0x66, 0x89, 0x07, // MOV [GS:EDI], AX
                0xFF, 0x05, 0x00, 0x90, 0x00, 0x00, // INC dword [0x9000]
                0xEB, 0xE5, // JMP print_loop
                0x5F, 0x58, 0xC3, // POP EDI, POP EAX, RET
            ];
            stage2_code.extend_from_slice(&routine_print);
            // --- NOUVEAU : Routine pour imprimer EAX (%ka) en Hexadécimal ---
            self.labels.insert(
                "print_hex_32".to_string(),
                base_stage2 + (stage2_code.len() as isize),
            );

            let routine_hex = vec![
                0x60, // PUSHAD
                0xB9, 0x08, 0x00, 0x00, 0x00, // MOV ECX, 8
                0xC1, 0xC0, 0x04, // ROL EAX, 4
                0x50, // PUSH EAX
                0x24, 0x0F, // AND AL, 0x0F
                0x04, 0x90, // ADD AL, 0x90
                0x27, // DAA
                0x14, 0x40, // ADC AL, 0x40
                0x27, // DAA
                0x8B, 0x3D, 0x00, 0x90, 0x00, 0x00, // MOV EDI, [0x9000]
                0xD1, 0xE7, // SHL EDI, 1
                0xB4, 0x0F, // MOV AH, 0x0F
                0x65, 0x66, 0x89, 0x07, // MOV [GS:EDI], AX
                0xFF, 0x05, 0x00, 0x90, 0x00, 0x00, // INC dword [0x9000]
                0x58, // POP EAX
                0xE2, 0xDD, // LOOP loop_start
                0x61, 0xC3, // POPAD, RET
            ];
            stage2_code.extend_from_slice(&routine_hex);
        }

        if pmode_inserted {
            // --- Sekhmet / Phenix : ISR de resurrection ---
            let isr_offset = stage2_code.len();
            let isr_len = 13usize;
            let phoenix_addr = (base_stage2 + isr_offset as isize + isr_len as isize) as u32;
            let mut isr_phoenix = Vec::new();
            isr_phoenix.push(0xFA); // CLI
            isr_phoenix.push(0xBC); // MOV ESP, imm32
            isr_phoenix.extend_from_slice(&STACK_TOP.to_le_bytes());
            isr_phoenix.push(0xB8); // MOV EAX, imm32
            isr_phoenix.extend_from_slice(&phoenix_addr.to_le_bytes());
            isr_phoenix.extend_from_slice(&[0xFF, 0xE0]); // JMP EAX
            stage2_code.extend_from_slice(&isr_phoenix);

            self.labels.insert(
                "__phoenix_rebirth".to_string(),
                base_stage2 + (stage2_code.len() as isize),
            );
            let phoenix_rebirth = vec![
                0x8B, 0x1D, 0x04, 0x90, 0x00, 0x00, 0x85, 0xDB, 0x74, 0x37, 0x8B,
                0x73, 0x70, 0x8D, 0x7E, 0x10, 0x8D, 0x73, 0x40, 0xB9, 0x20, 0x00,
                0x00, 0x00, 0xFC, 0xF3, 0xA6, 0x75, 0x24, 0x8B, 0x73, 0x70, 0x83,
                0xC6, 0x30, 0x8B, 0x7B, 0x74, 0x8B, 0x4B, 0x78, 0x89, 0xCA, 0xC1,
                0xE9, 0x02, 0xF3, 0xA5, 0x89, 0xD1, 0x83, 0xE1, 0x03, 0xF3, 0xA4,
                0xBC, 0x00, 0xFC, 0x09, 0x00, 0x8B, 0x43, 0x7C, 0xFF, 0xE0, 0xFA,
                0xF4, 0xEB, 0xFC,
            ];
            stage2_code.extend_from_slice(&phoenix_rebirth);

            // --- IDT (32 exceptions) ---
            let idt_offset = stage2_code.len();
            let isr_addr = (base_stage2 + isr_offset as isize) as u32;
            let isr_off16 = isr_addr as u16;
            let isr_off_hi = (isr_addr >> 16) as u16;
            let mut idt: Vec<u8> = Vec::new();
            for _ in 0..32 {
                idt.extend_from_slice(&isr_off16.to_le_bytes()); // offset low
                idt.extend_from_slice(&0x08u16.to_le_bytes()); // code selector
                idt.push(0x00); // zero
                idt.push(0x8E); // present, ring0, 32-bit interrupt gate
                idt.extend_from_slice(&isr_off_hi.to_le_bytes()); // offset high
            }
            stage2_code.extend_from_slice(&idt);

            // --- IDTR ---
            let idtr_offset = stage2_code.len();
            let idt_limit = (idt.len() - 1) as u16;
            let idt_base = (base_stage2 + idt_offset as isize) as u32;
            stage2_code.extend_from_slice(&idt_limit.to_le_bytes());
            stage2_code.extend_from_slice(&idt_base.to_le_bytes());

            // --- GDT ---
            let gdt_offset = stage2_code.len();
            let gdt: Vec<u8> = vec![
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // null
                0xFF, 0xFF, 0x00, 0x00, 0x00, 0x9A, 0xCF,
                0x00, // code (32-bit, base 0, limit 4GB)
                0xFF, 0xFF, 0x00, 0x00, 0x00, 0x92, 0xCF,
                0x00, // data (32-bit, base 0, limit 4GB)
                0xFF, 0x0F, 0x00, 0x80, 0x0B, 0x92, 0x00,
                0x00, // vga (base 0xB8000, limit 4KB)
            ];
            stage2_code.extend_from_slice(&gdt);

            // --- GDTR ---
            let gdtr_offset = stage2_code.len();
            let gdt_limit = (gdt.len() - 1) as u16;
            let gdt_base = (base_stage2 + gdt_offset as isize) as u32;
            stage2_code.extend_from_slice(&gdt_limit.to_le_bytes());
            stage2_code.extend_from_slice(&gdt_base.to_le_bytes());

            // Patch LGDT / LIDT displacements (real-mode absolute addresses)
            if let Some(off) = pmode_lgdt_patch {
                let addr = (base_stage2 + gdtr_offset as isize) as u16;
                stage2_code[off] = (addr & 0xFF) as u8;
                stage2_code[off + 1] = (addr >> 8) as u8;
            }
            if let Some(off) = pmode_lidt_patch {
                let addr = (base_stage2 + idtr_offset as isize) as u16;
                stage2_code[off] = (addr & 0xFF) as u8;
                stage2_code[off + 1] = (addr >> 8) as u8;
            }
        }
        // --- LE PATCHING ---
        for patch in &self.jump {
            let base = if patch.kernel {
                base_stage2
            } else {
                base_stage1
            };
            let buffer = if patch.kernel {
                &mut stage2_code
            } else {
                &mut stage1_code
            };
            if let Expression::Identifier(nom) = &patch.cible {
                let addr = self.labels.get(nom.as_str()).expect("Label manquant");
                let dist = addr - (base + patch.offset as isize + patch.size as isize);
                if patch.size == 4 {
                    let b = (dist as i32).to_le_bytes();
                    buffer[patch.offset] = b[0];
                    buffer[patch.offset + 1] = b[1];
                    buffer[patch.offset + 2] = b[2];
                    buffer[patch.offset + 3] = b[3];
                } else {
                    if dist < i16::MIN as isize || dist > i16::MAX as isize {
                        panic!("Jump out of range in real mode for label '{}'", nom);
                    }
                    let b = (dist as i16).to_le_bytes();
                    buffer[patch.offset] = b[0];
                    buffer[patch.offset + 1] = b[1];
                }
            }
        }
        // --- FUSION FINALE DES MONDES ---
        let mut binaire_final = stage1_code;

        if is_bootloader {
            // Mode OS (Bootloader) : Alignements stricts pour le matériel
            while binaire_final.len() < 510 {
                binaire_final.push(0);
            }
            binaire_final.extend_from_slice(&[0x55, 0xAA]); // Fin du Secteur 1

            // On aligne le Stage 2 pour que le Noun commence au Secteur 3 (octet 1024)
            let mut bloc_stage2 = stage2_code;
            while bloc_stage2.len() < 512 {
                bloc_stage2.push(0);
            }

            binaire_final.extend(bloc_stage2);
            binaire_final.extend(self.segment_noun.clone());

            // On s'assure que le fichier total est un multiple de 512 pour le BIOS
            while binaire_final.len() % 512 != 0 {
                binaire_final.push(0);
            }
        } else {
            // Mode Classique (Tests unitaires ou fichiers ELF Linux) : Zéro padding
            // On colle juste le code pur bout à bout
            binaire_final.extend(stage2_code);
            binaire_final.extend(self.segment_noun.clone());
        }

        binaire_final
    }
}
