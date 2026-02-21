use crate::ast::{Expression, Instruction, Level};
use crate::register::{
    ensure_helix_fits, ensure_number_fits, ensure_same_level, ensure_supported_level, modrm_imm,
    modrm_mov_reg_rm, modrm_reg_reg, parse_general_register, parse_register, reg_code, seg_code,
    RegBase, RegKind,
};
use std::collections::HashMap;
const STAGE_ONE: isize = 0x7C00;
const STAGE_TWO: isize = 0x7E00;
const CURSOR_NOUN: u16 = 0x8000;

pub struct Emitter {
    instructions: Vec<Instruction>,
    kbd_layout: String,
    in_kernel: bool,
    segment_noun: Vec<u8>,
    variables: HashMap<String, u16>,
    dictionnaire_cas: HashMap<blake3::Hash, u16>,
    jump: Vec<(usize, Expression, bool)>,
    cursor_noun: u16,
    labels: HashMap<String, isize>,
}

pub enum Bin {
    Bootloader(Vec<u8>),
    Noun(Vec<u8>),
    Stage1(Vec<u8>),
    Stage2(Vec<u8>),
    Binaire(Vec<u8>),
}

impl Emitter {
    // On charge l'Émetteur avec l'Arbre Syntaxique (AST)
    pub fn new() -> Self {
        Emitter {
            instructions: Vec::new(),
            kbd_layout: String::new(),
            in_kernel: false,
            segment_noun: Vec::new(),
            variables: HashMap::new(),
            dictionnaire_cas: HashMap::new(),
            jump: Vec::new(),
            cursor_noun: CURSOR_NOUN,
            labels: HashMap::new(),
        }
    }
    pub fn add_instruction(&mut self, instruction: Vec<Instruction>) -> &mut Self {
        self.instructions.extend(instruction);
        self
    }

    pub fn add_variable(&mut self, nom: String, valeur: u16) -> &mut Self {
        self.variables.insert(nom, valeur);
        self
    }

    fn emit_mov_reg_reg(&self, code: &mut Vec<u8>, dest: RegBase, src: RegBase) {
        if dest == src {
            return;
        }
        code.push(0x66); // 32-bit operand
        code.push(0x8B); // MOV reg, r/m
        code.push(modrm_mov_reg_rm(dest, src));
    }

    fn emit_mov_reg_imm32(&self, code: &mut Vec<u8>, dest: RegBase, imm: u32) {
        code.push(0x66); // 32-bit operand
        code.push(0xB8 + reg_code(dest)); // MOV r32, imm32
        code.extend_from_slice(&imm.to_le_bytes());
    }

    fn emit_rep_movsd_4(&self, code: &mut Vec<u8>) {
        // MOV ECX, 4 ; CLD ; REP MOVSD (32-bit addr/data)
        code.extend_from_slice(&[0x66, 0xB9, 0x04, 0x00, 0x00, 0x00]);
        code.push(0xFC); // CLD
        code.extend_from_slice(&[0x66, 0x67, 0xF3, 0xA5]);
    }

    fn alloc_helix_literal(&mut self, level: Level, ra: u16, apophis: u16) -> u16 {
        if level != Level::Extreme {
            panic!("Helix literal storage is only supported for Extreme (128) right now.");
        }
        let addr = self.cursor_noun;
        let mut block = vec![0u8; level.bytes() as usize];
        let ra64 = (ra as u64).to_le_bytes();
        let ap64 = (apophis as u64).to_le_bytes();
        block[0..8].copy_from_slice(&ra64);
        block[8..16].copy_from_slice(&ap64);
        self.segment_noun.extend_from_slice(&block);
        self.cursor_noun += block.len() as u16;
        addr
    }

    fn alloc_xenith_literal(&mut self, ra: u16, apophis: u16) -> u16 {
        let addr = self.cursor_noun;
        let mut block = vec![0u8; Level::Xenith.bytes() as usize];
        let ra64 = (ra as u64).to_le_bytes();
        let ap64 = (apophis as u64).to_le_bytes();
        block[0..8].copy_from_slice(&ra64);
        block[8..16].copy_from_slice(&ap64);
        // Reste du bloc (16..32) = 0
        self.segment_noun.extend_from_slice(&block);
        self.cursor_noun += block.len() as u16;
        addr
    }
    pub fn kherankh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JLE (Jump if Less or Equal)
        actual_code.push(0x0F);
        actual_code.push(0x8E);

        // On enregistre l'emplacement pour le patcher plus tard
        self.jump
            .push((actual_code.len(), cible.clone(), self.in_kernel));

        // On laisse 2 octets vides pour la distance
        actual_code.extend_from_slice(&[0x00, 0x00]);
    }
    pub fn herankh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JGE (Jump if Greater or Equal)
        actual_code.push(0x0F);
        actual_code.push(0x8D);

        // On enregistre l'emplacement pour le patcher plus tard
        self.jump
            .push((actual_code.len(), cible.clone(), self.in_kernel));

        // On laisse 2 octets vides pour la distance (le "trou" à patcher)
        actual_code.extend_from_slice(&[0x00, 0x00]);
    }
    pub fn ankh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        // OpCode pour JE near (Sauter si Égal, relatif 16-bit)
        actual_code.push(0x0F);
        actual_code.push(0x84);

        // On enregistre l'endroit à patcher (comme pour neheh)
        self.jump
            .push((actual_code.len(), cible.clone(), self.in_kernel));

        // Placeholders
        actual_code.push(0x00);
        actual_code.push(0x00);
    }
    pub fn per(&mut self, actual_code: &mut Vec<u8>, message: &Expression) {
        match message {
            Expression::StringLiteral(s) => {
                // 1. On enregistre le texte dans le Noun (Secteur 3+)
                let addr = self.cursor_noun;
                self.segment_noun.extend_from_slice(s.as_bytes());
                self.segment_noun.push(0); // Signe du Silence
                self.cursor_noun += (s.len() + 1) as u16;

                // 2. On génère le code machine pour l'afficher
                actual_code.push(0xBE); // MOV SI, adresse_du_texte
                actual_code.extend_from_slice(&addr.to_le_bytes());

                actual_code.push(0xE8); // CALL std_print
                self.jump.push((
                    actual_code.len(),
                    Expression::Identifier("std_print".to_string()),
                    self.in_kernel,
                ));
                actual_code.extend_from_slice(&[0x00, 0x00]);
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
            actual_code.push(0x66);
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
                    actual_code.push(0xE8);
                    self.jump.push((
                        actual_code.len(),
                        Expression::Identifier("__helix_or128".to_string()),
                        self.in_kernel,
                    ));
                    actual_code.extend_from_slice(&[0x00, 0x00]);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    actual_code.push(0xE8);
                    self.jump.push((
                        actual_code.len(),
                        Expression::Identifier("__helix_or128".to_string()),
                        self.in_kernel,
                    ));
                    actual_code.extend_from_slice(&[0x00, 0x00]);
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
            actual_code.push(0x66); // Stabilisation 32 bits
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
                    actual_code.push(0xE8);
                    self.jump.push((
                        actual_code.len(),
                        Expression::Identifier("__helix_and128".to_string()),
                        self.in_kernel,
                    ));
                    actual_code.extend_from_slice(&[0x00, 0x00]);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    actual_code.push(0xE8);
                    self.jump.push((
                        actual_code.len(),
                        Expression::Identifier("__helix_and128".to_string()),
                        self.in_kernel,
                    ));
                    actual_code.extend_from_slice(&[0x00, 0x00]);
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
            actual_code.push(0x66); // Stabilisation 32 bits
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
                    actual_code.push(0xE8);
                    self.jump.push((
                        actual_code.len(),
                        Expression::Identifier("__helix_sub128".to_string()),
                        self.in_kernel,
                    ));
                    actual_code.extend_from_slice(&[0x00, 0x00]);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(actual_code, RegBase::Si, addr as u32);
                    actual_code.push(0xE8);
                    self.jump.push((
                        actual_code.len(),
                        Expression::Identifier("__helix_sub128".to_string()),
                        self.in_kernel,
                    ));
                    actual_code.extend_from_slice(&[0x00, 0x00]);
                }
                _ => panic!("Kheb only supports Helix literals or registers for 128-bit."),
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
    pub fn set_cursor_noun(&mut self, cursor_noun: u16) {
        self.cursor_noun = cursor_noun;
    }
    pub fn neheh(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        actual_code.push(0xE9);
        self.jump
            .push((actual_code.len(), cible.clone(), self.in_kernel));
        actual_code.extend_from_slice(&[0x00, 0x00]);
    }
    pub fn jena(&mut self, actual_code: &mut Vec<u8>, cible: &Expression) {
        actual_code.push(0xE8);
        self.jump
            .push((actual_code.len(), cible.clone(), self.in_kernel));
        actual_code.extend_from_slice(&[0x00, 0x00]);
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
        code_actual.push(0x0F);
        code_actual.push(0x85);

        // On enregistre l'endroit à patcher EXACTEMENT comme pour ankh
        self.jump
            .push((code_actual.len(), cible.clone(), self.in_kernel));

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

        // Préfixes pour le Mode Réel : 32 bits data, 16 bits addr
        code_actual.push(0x66);
        code_actual.push(0x67);
        ensure_supported_level("kheper", source, source_spec.level);

        let reg_code: u8 = reg_code(source_base);
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
                    actual_code.extend_from_slice(&[0x66, 0x83, 0xEC, 0x10]);
                    // ESI = source pointer
                    self.emit_mov_reg_reg(actual_code, RegBase::Si, reg_base);
                    // EDI = ESP
                    actual_code.extend_from_slice(&[0x66, 0x8B, 0xFC]);
                    self.emit_rep_movsd_4(actual_code);
                } else if reg_spec.level == Level::Xenith {
                    panic!(
                        "Push does not yet support registers beyond Extreme: %{} ({})",
                        r, reg_spec.level
                    );
                } else {
                    actual_code.push(0x66); // Protection 32-bit
                    // L'OpCode PUSH registre commence à 0x50
                    let opcode = 0x50 + reg_code(reg_base);
                    actual_code.push(opcode);
                }
            }
            Expression::Number(n) => {
                actual_code.push(0x66); // Protection 32-bit
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

        // On prépare le terrain : Mode 32 bits et Adressage 16 bits
        code_actual.push(0x66);
        code_actual.push(0x67);

        // 1. On identifie le code du registre cible
        let reg_code: u8 = reg_code(dest_base);
        ensure_supported_level("sena", destination, dest_spec.level);

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
                    .expect(&format!("Variable '{}' introuvable", nom));
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
            actual_code.extend_from_slice(&[0x66, 0x8B, 0xF4]);
            // EDI = destination pointer
            self.emit_mov_reg_reg(actual_code, RegBase::Di, dest_base);
            self.emit_rep_movsd_4(actual_code);
            // ADD ESP, 16
            actual_code.extend_from_slice(&[0x66, 0x83, 0xC4, 0x10]);
        } else if dest_spec.level == Level::Xenith {
            panic!(
                "Pop does not yet support registers beyond Extreme: %{} ({})",
                destination, dest_spec.level
            );
        } else {
            actual_code.push(0x66); // Protection 32-bit
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
        code_actual.push(0x0F);
        code_actual.push(0x8F); // OpCode pour JG (Saut si plus grand)
        self.jump
            .push((code_actual.len(), cible.clone(), self.in_kernel));
        code_actual.extend_from_slice(&[0x00, 0x00]);
    }
    pub fn kher(&mut self, code_actual: &mut Vec<u8>, cible: &Expression) {
        code_actual.push(0x0F);
        code_actual.push(0x8C); // OpCode pour JL (Saut si plus petit)
        self.jump
            .push((code_actual.len(), cible.clone(), self.in_kernel));
        code_actual.extend_from_slice(&[0x00, 0x00]);
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

        let hash = blake3::hash(&contenu_brut);

        let adresse = if let Some(addr) = self.dictionnaire_cas.get(&hash) {
            *addr
        } else {
            let addr = self.cursor_noun;
            self.segment_noun.extend_from_slice(&contenu_brut);
            self.dictionnaire_cas.insert(hash, addr);
            self.cursor_noun += contenu_brut.len() as u16;
            addr
        };
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
                    code.push(0x66);
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
    fn emmit(&mut self, bin: Bin, v: &mut Vec<u8>) -> &mut Self {
        match bin {
            Bin::Bootloader(x) => {
                v.extend_from_slice(&x);
            }
            Bin::Noun(x) => {
                v.extend_from_slice(&x);
            }
            Bin::Stage1(x) => {
                v.extend_from_slice(&x);
            }
            Bin::Stage2(x) => {
                v.extend_from_slice(&x);
            }
            Bin::Binaire(x) => {
                v.extend_from_slice(&x);
            }
        }
        self
    }
    pub fn sema(&mut self, code_actual: &mut Vec<u8>, destination: &String, value: &Expression) {
        let dest_spec = parse_general_register(destination);
        let dest_base = match dest_spec.kind {
            RegKind::General(base) => base,
            _ => unreachable!(),
        };
        if dest_spec.level <= Level::High {
            code_actual.push(0x66); // Stabilisation 32 bits
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
                    code_actual.push(0xE8);
                    self.jump.push((
                        code_actual.len(),
                        Expression::Identifier("__helix_add128".to_string()),
                        self.in_kernel,
                    ));
                    code_actual.extend_from_slice(&[0x00, 0x00]);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, addr as u32);
                    code_actual.push(0xE8);
                    self.jump.push((
                        code_actual.len(),
                        Expression::Identifier("__helix_add128".to_string()),
                        self.in_kernel,
                    ));
                    code_actual.extend_from_slice(&[0x00, 0x00]);
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
                    code_actual.push(0xE8);
                    self.jump.push((
                        code_actual.len(),
                        Expression::Identifier("__xenith_add256".to_string()),
                        self.in_kernel,
                    ));
                    code_actual.extend_from_slice(&[0x00, 0x00]);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_xenith_literal(*ra, *apophis);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, addr as u32);
                    code_actual.push(0xE8);
                    self.jump.push((
                        code_actual.len(),
                        Expression::Identifier("__xenith_add256".to_string()),
                        self.in_kernel,
                    ));
                    code_actual.extend_from_slice(&[0x00, 0x00]);
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
            code_actual.push(0x66); // Stabilisation 32 bits
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
                    code_actual.push(0xE8);
                    self.jump.push((
                        code_actual.len(),
                        Expression::Identifier("__helix_mul128".to_string()),
                        self.in_kernel,
                    ));
                    code_actual.extend_from_slice(&[0x00, 0x00]);
                }
                Expression::Helix { ra, apophis } => {
                    let addr = self.alloc_helix_literal(dest_spec.level, *ra, *apophis);
                    self.emit_mov_reg_reg(code_actual, RegBase::Di, dest_base);
                    self.emit_mov_reg_imm32(code_actual, RegBase::Si, addr as u32);
                    code_actual.push(0xE8);
                    self.jump.push((
                        code_actual.len(),
                        Expression::Identifier("__helix_mul128".to_string()),
                        self.in_kernel,
                    ));
                    code_actual.extend_from_slice(&[0x00, 0x00]);
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

        // Le Stage 1 est à 0x7C00, le Stage 2 commence à 0x7E00 (juste après 512 octets)
        let base_stage1 = STAGE_ONE;
        let base_stage2 = STAGE_TWO;
        let instructions = self.instructions.clone();
        for instruction in instructions {
            if let Instruction::Label(ref nom) = instruction {
                if nom == "kernel" {
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
                            actual_code.push(0x66); // LA PROTECTION V4
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
                }
                Instruction::Wdj { left, right } => {
                    let left_spec = parse_general_register(&left);
                    let left_base = match left_spec.kind {
                        RegKind::General(base) => base,
                        _ => unreachable!(),
                    };
                    if left_spec.level <= Level::High {
                        // 1. On force le mode 32 bits pour la précision
                        actual_code.push(0x66);
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
                            _ => panic!(
                                "La Balance ne sait peser que des nombres ou des registres."
                            ),
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
                                actual_code.push(0xE8);
                                self.jump.push((
                                    actual_code.len(),
                                    Expression::Identifier("__helix_cmp128".to_string()),
                                    self.in_kernel,
                                ));
                                actual_code.extend_from_slice(&[0x00, 0x00]);
                            }
                            Expression::Helix { ra, apophis } => {
                                let addr = self.alloc_helix_literal(left_spec.level, ra, apophis);
                                self.emit_mov_reg_reg(&mut actual_code, RegBase::Di, left_base);
                                self.emit_mov_reg_imm32(&mut actual_code, RegBase::Si, addr as u32);
                                actual_code.push(0xE8);
                                self.jump.push((
                                    actual_code.len(),
                                    Expression::Identifier("__helix_cmp128".to_string()),
                                    self.in_kernel,
                                ));
                                actual_code.extend_from_slice(&[0x00, 0x00]);
                            }
                            _ => panic!("Wdj only supports Helix literals or registers for 128-bit."),
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
            0x66, 0x60, // PUSHAD
            // RA (0..7)
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0x03, 0x06, // ADD EAX, [ESI]
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0x13, 0x46, 0x04, // ADC EAX, [ESI+4]
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x73, 0x0F, // JNC +15 (skip RA saturation)
            0x66, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // MOV EAX, 0xFFFFFFFF
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0xF8, // CLC
            // Apophis (8..15)
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0x03, 0x46, 0x08, // ADD EAX, [ESI+8]
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0x13, 0x46, 0x0C, // ADC EAX, [ESI+12]
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x73, 0x10, // JNC +16 (skip Apophis saturation)
            0x66, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // MOV EAX, 0xFFFFFFFF
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_add);

        self.labels.insert(
            "__xenith_add256".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let xenith_add = vec![
            0x66, 0x60, // PUSHAD
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0x03, 0x06, // ADD EAX, [ESI]
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0x13, 0x46, 0x04, // ADC EAX, [ESI+4]
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0x13, 0x46, 0x08, // ADC EAX, [ESI+8]
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0x13, 0x46, 0x0C, // ADC EAX, [ESI+12]
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x67, 0x8B, 0x47, 0x10, // MOV EAX, [EDI+16]
            0x66, 0x67, 0x13, 0x46, 0x10, // ADC EAX, [ESI+16]
            0x66, 0x67, 0x89, 0x47, 0x10, // MOV [EDI+16], EAX
            0x66, 0x67, 0x8B, 0x47, 0x14, // MOV EAX, [EDI+20]
            0x66, 0x67, 0x13, 0x46, 0x14, // ADC EAX, [ESI+20]
            0x66, 0x67, 0x89, 0x47, 0x14, // MOV [EDI+20], EAX
            0x66, 0x67, 0x8B, 0x47, 0x18, // MOV EAX, [EDI+24]
            0x66, 0x67, 0x13, 0x46, 0x18, // ADC EAX, [ESI+24]
            0x66, 0x67, 0x89, 0x47, 0x18, // MOV [EDI+24], EAX
            0x66, 0x67, 0x8B, 0x47, 0x1C, // MOV EAX, [EDI+28]
            0x66, 0x67, 0x13, 0x46, 0x1C, // ADC EAX, [ESI+28]
            0x66, 0x67, 0x89, 0x47, 0x1C, // MOV [EDI+28], EAX
            0x73, 0x30, // JNC +48 (skip saturation)
            0x66, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // MOV EAX, 0xFFFFFFFF
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x67, 0x89, 0x47, 0x10, // MOV [EDI+16], EAX
            0x66, 0x67, 0x89, 0x47, 0x14, // MOV [EDI+20], EAX
            0x66, 0x67, 0x89, 0x47, 0x18, // MOV [EDI+24], EAX
            0x66, 0x67, 0x89, 0x47, 0x1C, // MOV [EDI+28], EAX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&xenith_add);

        self.labels.insert(
            "__helix_sub128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_sub = vec![
            0x66, 0x60, // PUSHAD
            0xF8, // CLC
            // RA (0..7)
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0x2B, 0x06, // SUB EAX, [ESI]
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0x1B, 0x46, 0x04, // SBB EAX, [ESI+4]
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x73, 0x0C, // JNC +12 (skip RA zero)
            0x66, 0x31, 0xC0, // XOR EAX, EAX
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0xF8, // CLC
            // Apophis (8..15)
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0x2B, 0x46, 0x08, // SUB EAX, [ESI+8]
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0x1B, 0x46, 0x0C, // SBB EAX, [ESI+12]
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x73, 0x0D, // JNC +13 (skip Apophis zero)
            0x66, 0x31, 0xC0, // XOR EAX, EAX
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_sub);

        self.labels.insert(
            "__helix_mul128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_mul = vec![
            0x66, 0x60, // PUSHAD
            // RA (0..7)
            0x66, 0x31, 0xED, // XOR EBP, EBP (overflow flag)
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0xF7, 0x26, // MUL dword [ESI]
            0x66, 0x89, 0xC3, // MOV EBX, EAX (low32)
            0x66, 0x89, 0xD1, // MOV ECX, EDX (high32)
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0xF7, 0x66, 0x04, // MUL dword [ESI+4]
            0x66, 0x0B, 0xEA, // OR EBP, EDX (p1_hi)
            0x66, 0x50, // PUSH EAX (p1_lo)
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0xF7, 0x26, // MUL dword [ESI]
            0x66, 0x0B, 0xEA, // OR EBP, EDX (p2_hi)
            0x66, 0x50, // PUSH EAX (p2_lo)
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0xF7, 0x66, 0x04, // MUL dword [ESI+4]
            0x66, 0x0B, 0xEA, // OR EBP, EDX (p3_hi)
            0x66, 0x0B, 0xE8, // OR EBP, EAX (p3_lo)
            0x66, 0x5A, // POP EDX (p2_lo)
            0x66, 0x58, // POP EAX (p1_lo)
            0x66, 0x01, 0xC1, // ADD ECX, EAX
            0x66, 0x83, 0xD5, 0x00, // ADC EBP, 0
            0x66, 0x01, 0xD1, // ADD ECX, EDX
            0x66, 0x83, 0xD5, 0x00, // ADC EBP, 0
            0x66, 0x85, 0xED, // TEST EBP, EBP
            0x75, 0x0B, // JNZ saturate_ra
            0x66, 0x67, 0x89, 0x1F, // MOV [EDI], EBX
            0x66, 0x67, 0x89, 0x4F, 0x04, // MOV [EDI+4], ECX
            0xEB, 0x0F, // JMP done_ra
            0x66, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // saturate_ra: MOV EAX, 0xFFFFFFFF
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            // Apophis (8..15)
            0x66, 0x31, 0xED, // XOR EBP, EBP (overflow flag)
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0xF7, 0x66, 0x08, // MUL dword [ESI+8]
            0x66, 0x89, 0xC3, // MOV EBX, EAX (low32)
            0x66, 0x89, 0xD1, // MOV ECX, EDX (high32)
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0xF7, 0x66, 0x0C, // MUL dword [ESI+12]
            0x66, 0x0B, 0xEA, // OR EBP, EDX (p1_hi)
            0x66, 0x50, // PUSH EAX (p1_lo)
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0xF7, 0x66, 0x08, // MUL dword [ESI+8]
            0x66, 0x0B, 0xEA, // OR EBP, EDX (p2_hi)
            0x66, 0x50, // PUSH EAX (p2_lo)
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0xF7, 0x66, 0x0C, // MUL dword [ESI+12]
            0x66, 0x0B, 0xEA, // OR EBP, EDX (p3_hi)
            0x66, 0x0B, 0xE8, // OR EBP, EAX (p3_lo)
            0x66, 0x5A, // POP EDX (p2_lo)
            0x66, 0x58, // POP EAX (p1_lo)
            0x66, 0x01, 0xC1, // ADD ECX, EAX
            0x66, 0x83, 0xD5, 0x00, // ADC EBP, 0
            0x66, 0x01, 0xD1, // ADD ECX, EDX
            0x66, 0x83, 0xD5, 0x00, // ADC EBP, 0
            0x66, 0x85, 0xED, // TEST EBP, EBP
            0x75, 0x0C, // JNZ saturate_apo
            0x66, 0x67, 0x89, 0x5F, 0x08, // MOV [EDI+8], EBX
            0x66, 0x67, 0x89, 0x4F, 0x0C, // MOV [EDI+12], ECX
            0xEB, 0x10, // JMP done_apo
            0x66, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, // saturate_apo: MOV EAX, 0xFFFFFFFF
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_mul);

        self.labels.insert(
            "__helix_and128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_and = vec![
            0x66, 0x60, // PUSHAD
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0x23, 0x06, // AND EAX, [ESI]
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0x23, 0x46, 0x04, // AND EAX, [ESI+4]
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0x23, 0x46, 0x08, // AND EAX, [ESI+8]
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0x23, 0x46, 0x0C, // AND EAX, [ESI+12]
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_and);

        self.labels.insert(
            "__helix_or128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_or = vec![
            0x66, 0x60, // PUSHAD
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0x0B, 0x06, // OR EAX, [ESI]
            0x66, 0x67, 0x89, 0x07, // MOV [EDI], EAX
            0x66, 0x67, 0x8B, 0x47, 0x04, // MOV EAX, [EDI+4]
            0x66, 0x67, 0x0B, 0x46, 0x04, // OR EAX, [ESI+4]
            0x66, 0x67, 0x89, 0x47, 0x04, // MOV [EDI+4], EAX
            0x66, 0x67, 0x8B, 0x47, 0x08, // MOV EAX, [EDI+8]
            0x66, 0x67, 0x0B, 0x46, 0x08, // OR EAX, [ESI+8]
            0x66, 0x67, 0x89, 0x47, 0x08, // MOV [EDI+8], EAX
            0x66, 0x67, 0x8B, 0x47, 0x0C, // MOV EAX, [EDI+12]
            0x66, 0x67, 0x0B, 0x46, 0x0C, // OR EAX, [ESI+12]
            0x66, 0x67, 0x89, 0x47, 0x0C, // MOV [EDI+12], EAX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_or);

        self.labels.insert(
            "__helix_cmp128".to_string(),
            base_stage2 + (stage2_code.len() as isize),
        );
        let helix_cmp = vec![
            0x66, 0x60, // PUSHAD
            0x66, 0x67, 0x8B, 0x07, // MOV EAX, [EDI]
            0x66, 0x67, 0x03, 0x46, 0x08, // ADD EAX, [ESI+8]
            0x66, 0x67, 0x8B, 0x5F, 0x04, // MOV EBX, [EDI+4]
            0x66, 0x67, 0x13, 0x5E, 0x0C, // ADC EBX, [ESI+12]
            0x0F, 0x92, 0xC1, // SETC CL
            0x66, 0x67, 0x8B, 0x16, // MOV EDX, [ESI]
            0x66, 0x67, 0x03, 0x57, 0x08, // ADD EDX, [EDI+8]
            0x66, 0x67, 0x8B, 0x6E, 0x04, // MOV EBP, [ESI+4]
            0x66, 0x67, 0x13, 0x6F, 0x0C, // ADC EBP, [EDI+12]
            0x0F, 0x92, 0xC5, // SETC CH
            0x38, 0xE9, // CMP CL, CH
            0x75, 0x08, // JNE done
            0x66, 0x39, 0xEB, // CMP EBX, EBP
            0x75, 0x03, // JNE done
            0x66, 0x39, 0xD0, // CMP EAX, EDX
            0x66, 0x61, // POPAD
            0xC3, // RET
        ];
        stage2_code.extend_from_slice(&helix_cmp);

        if is_bootloader {
            self.labels.insert(
                "std_print".to_string(),
                base_stage2 + (stage2_code.len() as isize),
            );

            let routine_print = vec![
                0x2E, 0x8A, 0x04, // 00: MOV AL, CS:[SI]
                0x08, 0xC0, // 03: OR AL, AL
                0x74, 0x19, // 05: JZ end_print (Saute vers RET s'il voit un 0)
                // --- Détecteur de Tabulation ---
                0x3C, 0x09, // 07: CMP AL, 0x09
                0x75,
                0x0E, // 09: JNE regular_char (Si ce n'est pas un Tab, va à l'affichage normal)
                // --- Si c'est un Tab (Affichage de 4 espaces) ---
                0xB0, 0x20, // 0B: MOV AL, ' '
                0xB4, 0x0E, // 0D: MOV AH, 0x0E
                0xCD, 0x10, // 0F: INT 10h (Espace 1)
                0xCD, 0x10, // 11: INT 10h (Espace 2)
                0xCD, 0x10, // 13: INT 10h (Espace 3)
                0xCD, 0x10, // 15: INT 10h (Espace 4)
                0xEB, 0x04, // 17: JMP next_char (Passe à la lettre suivante)
                // --- Affichage Classique ---
                0xB4, 0x0E, // 19: regular_char: MOV AH, 0x0E
                0xCD, 0x10, // 1B: INT 10h
                // --- Boucle ---
                0x46, // 1D: next_char: INC SI
                0xEB, 0xE0, // 1E: JMP print_loop (Saute précisément au début)
                0xC3, // 20: end_print: RET
            ];
            stage2_code.extend_from_slice(&routine_print);
            // --- NOUVEAU : Routine pour imprimer EAX (%ka) en Hexadécimal ---
            self.labels.insert(
                "print_hex_32".to_string(),
                base_stage2 + (stage2_code.len() as isize),
            );

            let routine_hex = vec![
                0x66, 0x60, // PUSHAD : Protège tous les registres de ton OS
                0xB9, 0x08,
                0x00, // MOV CX, 8 : On va extraire 8 caractères (32 bits = 8x4 bits)
                // --- loop_start: ---
                0x66, 0xC1, 0xC0, 0x04, // ROL EAX, 4 : Fait tourner le registre de 4 bits
                0x66, 0x50, // PUSH EAX : Sauvegarde temporaire
                0x24, 0x0F, // AND AL, 0x0F : Isole les 4 bits tout à droite
                // --- La Magie de Conversion (0-15 -> '0'-'9', 'A'-'F') ---
                0x04, 0x90, // ADD AL, 0x90
                0x27, // DAA
                0x14, 0x40, // ADC AL, 0x40
                0x27, // DAA
                // --- Appel BIOS ---
                0xB4, 0x0E, // MOV AH, 0x0E
                0xCD, 0x10, // INT 0x10 : Affiche le caractère
                0x66, 0x58, // POP EAX : Récupère la valeur pour le prochain tour
                0xE2, 0xEA, // LOOP loop_start : Recommence (Saute 22 octets en arrière)
                0x66, 0x61, // POPAD : Restaure les registres de l'OS
                0xC3, // RET : Retourne au programme principal
            ];
            stage2_code.extend_from_slice(&routine_hex);
        }
        // --- LE PATCHING ---
        for (offset, cible, kernel) in &self.jump {
            let base = if *kernel { base_stage2 } else { base_stage1 };
            let buffer = if *kernel {
                &mut stage2_code
            } else {
                &mut stage1_code
            };
            if let Expression::Identifier(nom) = cible {
                let addr = self.labels.get(nom.as_str()).expect("Label manquant");
                let dist = addr - (base + *offset as isize + 2);
                let b = (dist as i16).to_le_bytes();
                buffer[*offset] = b[0];
                buffer[offset + 1] = b[1];
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
