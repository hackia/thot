use crate::ast::{Expression, Instruction};
use std::collections::HashMap;

pub struct Emitter {
    instructions: Vec<Instruction>,
    _kbd_layout: String,
}

impl Emitter {
    // On charge l'Émetteur avec l'Arbre Syntaxique (AST)
    pub fn new(instructions: Vec<Instruction>, _kbd_layout: String) -> Self {
        Emitter {
            instructions,
            _kbd_layout,
        }
    }

    // Le grand convertisseur: AST -> Code Machine (Binaire)
    pub fn generer_binaire(&self, is_bootloader: bool) -> Vec<u8> {
        let mut stage1_code: Vec<u8> = Vec::new();
        let mut stage2_code: Vec<u8> = Vec::new();
        let mut segment_noun: Vec<u8> = Vec::new();
        let mut dans_noyau = false; // Le basculement vers l'infini

        let mut labels: HashMap<String, isize> = HashMap::new();
        let mut variables = HashMap::new();
        let mut dictionnaire_cas = HashMap::new();
        let mut sauts_a_patcher: Vec<(usize, Expression, bool)> = Vec::new();

        // Le Stage 1 est à 0x7C00, le Stage 2 commence à 0x7E00 (juste après 512 octets)
        let base_stage1 = 0x7C00;
        let base_stage2 = 0x7E00;
        let mut curseur_noun: u16 = 0x8000;
        for instruction in &self.instructions {
            if let Instruction::Label(nom) = instruction {
                if nom == "noyau" {
                    dans_noyau = true;
                }
            }
            let code_actuel = if dans_noyau {
                &mut stage2_code
            } else {
                &mut stage1_code
            };
            let base_actuelle = if dans_noyau { base_stage2 } else { base_stage1 };
            match instruction {
                Instruction::Neheh { cible } | Instruction::Jena { cible } => {
                    code_actuel.push(if matches!(instruction, Instruction::Jena { .. }) {
                        0xE8
                    } else {
                        0xE9
                    });
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));
                    code_actuel.extend_from_slice(&[0x00, 0x00]);
                }

                Instruction::Henek {
                    destination,
                    valeur,
                } => {
                    match destination.as_str() {
                        "ds" | "es" | "ss" => {
                            if let Expression::Register(src) = valeur {
                                code_actuel.push(0x8E); // Correction : on utilise code_actuel !
                                let sreg_code = match destination.as_str() {
                                    "es" => 0,
                                    "ss" => 2,
                                    "ds" => 3,
                                    _ => 0,
                                };
                                let src_code = match src.as_str() {
                                    "ka" => 0,
                                    "ib" => 1,
                                    "da" => 2,
                                    "ba" => 3,
                                    _ => 0,
                                };
                                code_actuel.push(0xC0 | (sreg_code << 3) | src_code);
                            } else {
                                panic!("Sreg exige un registre.");
                            }
                        }
                        _ => {
                            // Ton code Henek existant pour ka, ib, ba...
                            code_actuel.push(0x66);
                            let dest_code = match destination.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                _ => 0,
                            };
                            match valeur {
                                Expression::Number(n) => {
                                    code_actuel.push(0xB8 + dest_code);
                                    code_actuel.extend_from_slice(&(*n as i32).to_le_bytes());
                                }
                                Expression::Register(src_name) => {
                                    let src_code = match src_name.as_str() {
                                        "ka" => 0,
                                        "ib" => 1,
                                        "da" => 2,
                                        "ba" => 3,
                                        _ => 0,
                                    };
                                    code_actuel.push(0x8B);
                                    code_actuel.push(0xC0 | (dest_code << 3) | src_code);
                                }
                                _ => { /* ... identifiant ... */ }
                            }
                        }
                    }
                }

                // 2. Unifie le NAMA avec BLAKE3 (SLS Pur)
                Instruction::Nama { nom, valeur } => {
                    let contenu_brut = match valeur {
                        Expression::Number(n) => n.to_le_bytes().to_vec(),
                        Expression::StringLiteral(s) => {
                            let mut b = s.as_bytes().to_vec();
                            b.push(0); // Signe du Silence
                            b
                        }
                        _ => panic!("Type non supporté dans le Noun."),
                    };

                    let hash = blake3::hash(&contenu_brut);

                    let adresse = if let Some(addr) = dictionnaire_cas.get(&hash) {
                        *addr
                    } else {
                        let addr = curseur_noun;
                        segment_noun.extend_from_slice(&contenu_brut);
                        dictionnaire_cas.insert(hash, addr);
                        curseur_noun += contenu_brut.len() as u16;
                        addr
                    };
                    variables.insert(nom.clone(), adresse);
                }
                Instruction::Push { cible } => {
                    match cible {
                        Expression::Register(r) => {
                            code_actuel.push(0x66); // Protection 32-bit
                            // L'OpCode PUSH registre commence à 0x50
                            let opcode = 0x50
                                + match r.as_str() {
                                    "ka" => 0x00,
                                    "ib" => 0x01,
                                    "da" => 0x02,
                                    "ba" => 0x03,
                                    "si" => 0x06,
                                    "di" => 0x07,
                                    _ => panic!("Registre inconnu pour push : {}", r),
                                };
                            code_actuel.push(opcode);
                        }
                        Expression::Number(n) => {
                            code_actuel.push(0x66); // Protection 32-bit
                            code_actuel.push(0x68); // OpCode PUSH imm32
                            code_actuel.extend_from_slice(&n.to_le_bytes());
                        }
                        _ => panic!("Push ne supporte que les registres et les nombres."),
                    }
                }
                Instruction::Pop { destination } => {
                    code_actuel.push(0x66); // Protection 32-bit
                    // L'OpCode POP registre commence à 0x58
                    let opcode = 0x58
                        + match destination.as_str() {
                            "ka" => 0x00,
                            "ib" => 0x01,
                            "da" => 0x02,
                            "ba" => 0x03,
                            "si" => 0x06,
                            "di" => 0x07,
                            _ => panic!("Registre inconnu pour pop : {}", destination),
                        };
                    code_actuel.push(opcode);
                }
                Instruction::In { port } => {
                    // Lecture matérielle (toujours vers AL - 8 bits)
                    match port {
                        Expression::Number(n) => {
                            // IN AL, imm8 (Lit le port direct)
                            code_actuel.push(0xE4);
                            code_actuel.push(*n as u8);
                        }
                        Expression::Register(r) if r == "da" => {
                            // IN AL, DX (Lit le port contenu dans %da)
                            code_actuel.push(0xEC);
                        }
                        _ => panic!("Le port IN doit être un nombre ou le registre %da"),
                    }
                }
                Instruction::Out { port } => {
                    // Écriture matérielle (toujours depuis AL - 8 bits)
                    match port {
                        Expression::Number(n) => {
                            // OUT imm8, AL (Écrit vers le port direct)
                            code_actuel.push(0xE6);
                            code_actuel.push(*n as u8);
                        }
                        Expression::Register(r) if r == "da" => {
                            // OUT DX, AL (Écrit vers le port contenu dans %da)
                            code_actuel.push(0xEE);
                        }
                        _ => panic!("Le port OUT doit être un nombre ou le registre %da"),
                    }
                }

                Instruction::Her { cible } => {
                    code_actuel.push(0x0F);
                    code_actuel.push(0x8F); // OpCode pour JG (Saut si plus grand)
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));
                    code_actuel.extend_from_slice(&[0x00, 0x00]);
                }
                Instruction::Kher { cible } => {
                    code_actuel.push(0x0F);
                    code_actuel.push(0x8C); // OpCode pour JL (Saut si plus petit)
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));
                    code_actuel.extend_from_slice(&[0x00, 0x00]);
                }
                // Traduction de : isfet cible (Saut Conditionnel : JNE)
                Instruction::Isfet { cible } => {
                    // OpCode pour JNE near (Sauter si Différent, relatif 16-bit)
                    code_actuel.push(0x0F);
                    code_actuel.push(0x85);

                    // On enregistre l'endroit à patcher EXACTEMENT comme pour ankh
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));

                    // Placeholders (les zéros temporels)
                    code_actuel.push(0x00);
                    code_actuel.push(0x00);
                }
                Instruction::Kheper { source, adresse } => {
                    // Préfixes pour le Mode Réel : 32 bits data, 16 bits addr
                    code_actuel.push(0x66);
                    code_actuel.push(0x67);

                    // 1. On identifie le code du registre source
                    let reg_code: u8 = match source.as_str() {
                        "ka" => 0, // EAX
                        "ib" => 1, // ECX
                        "da" => 2, // EDX
                        "ba" => 3, // EBX
                        "si" => 6, // ESI
                        "di" => 7, // EDI
                        _ => panic!("Le Scribe ne connaît pas le registre source : {}", source),
                    };

                    match adresse {
                        Expression::Number(n) => {
                            // OpCode 0x89 avec ModR/M (0x06 | reg << 3) pour [disp16]
                            code_actuel.push(0x89);
                            code_actuel.push(0x06 | (reg_code << 3));
                            code_actuel.extend_from_slice(&(*n as u16).to_le_bytes());
                        }
                        Expression::Identifier(nom) => {
                            // On récupère l'adresse de la variable résolue par Thot
                            let addr = *variables
                                .get(nom)
                                .expect(&format!("Variable '{}' introuvable", nom));
                            code_actuel.push(0x89);
                            code_actuel.push(0x06 | (reg_code << 3));
                            code_actuel.extend_from_slice(addr.to_le_bytes().as_slice());
                        }
                        _ => panic!("L'adresse de destination est invalide pour kheper."),
                    }
                }
                Instruction::Rdtsc => {
                    code_actuel.push(0x0F);
                    code_actuel.push(0x31);
                }
                // Traduction de : sema %registre, valeur (ADD)
                Instruction::Sema {
                    destination,
                    valeur,
                } => {
                    code_actuel.push(0x66); // Stabilisation 32 bits

                    match valeur {
                        Expression::Number(n) => {
                            // MODE 1 : Additionner un Nombre (ADD r/m32, imm32)
                            code_actuel.push(0x81); // Opcode ADD
                            let modrm: u8 = match destination.as_str() {
                                "ka" => 0xC0,
                                "ib" => 0xC1,
                                "da" => 0xC2,
                                "ba" => 0xC3,
                                "si" => 0xC6,
                                "di" => 0xC7,
                                _ => panic!("Registre inconnu pour sema : {destination}"),
                            };
                            code_actuel.push(modrm);
                            code_actuel.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(src) => {
                            // MODE 2 : Additionner un Registre (ADD r/m32, reg32)
                            code_actuel.push(0x01); // Opcode ADD registre à registre

                            // On récupère le code numérique des deux registres (0 à 7)
                            let dest_code = match destination.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                "si" => 6,
                                "di" => 7,
                                _ => panic!("Registre de destination inconnu : {}", destination),
                            };
                            let src_code = match src.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                "si" => 6,
                                "di" => 7,
                                _ => panic!("Registre source inconnu : {src}"),
                            };

                            // Formule x86 magique (ModR/M) : 0xC0 (11000000 en binaire) + (source * 8) + destination
                            let modrm = 0xC0 | (src_code << 3) | dest_code;
                            code_actuel.push(modrm);
                        }
                        _ => panic!("Sema ne supporte que les nombres ou les registres."),
                    }
                }
                Instruction::Kherp => {
                    let setup_disque = vec![
                        0xB8, 0x08,
                        0x02, // AH=02 (Lecture), AL=08 (On lit 8 secteurs d'un coup !)
                        0xBB, 0x00, 0x7E, // Destination en RAM : 0x7E00
                        0xB9, 0x02, 0x00, // Commencer au Secteur n°2 du disque
                        0xBA, 0x80, 0x00, // Disque dur n°0
                        0xCD, 0x13, // Appel BIOS
                    ];
                    code_actuel.extend_from_slice(&setup_disque);
                }
                // Traduction de : sena %registre, adresse (MOV reg, [mem])
                Instruction::Sena {
                    destination,
                    adresse,
                } => {
                    // On prépare le terrain : Mode 32 bits et Adressage 16 bits
                    code_actuel.push(0x66);
                    code_actuel.push(0x67);

                    // 1. On identifie le code du registre cible
                    let reg_code: u8 = match destination.as_str() {
                        "ka" => 0, // EAX
                        "ib" => 1, // ECX
                        "da" => 2, // EDX
                        "ba" => 3, // EBX
                        "si" => 6, // ESI
                        "di" => 7, // EDI
                        _ => panic!("Le Scribe ne connaît pas le registre : {}", destination),
                    };

                    match adresse {
                        Expression::Number(n) => {
                            // OpCode 0x8B avec ModR/M (0x06 | reg << 3) pour [disp16]
                            code_actuel.push(0x8B);
                            code_actuel.push(0x06 | (reg_code << 3));
                            code_actuel.extend_from_slice(&(*n as u16).to_le_bytes());
                        }
                        Expression::Identifier(nom) => {
                            // On récupère l'adresse de la variable (SLS ou NAMA)
                            let addr = *variables
                                .get(nom)
                                .expect(&format!("Variable '{}' introuvable", nom));
                            code_actuel.push(0x8B);
                            code_actuel.push(0x06 | (reg_code << 3));
                            code_actuel.extend_from_slice(&(addr as u16).to_le_bytes());
                        }
                        Expression::Register(r) if r == "ba" => {
                            // Cas particulier : sena %reg, [%ba]
                            code_actuel.push(0x8B);
                            code_actuel.push(0x07 | (reg_code << 3));
                        }
                        _ => panic!("L'adresse de lecture est invalide pour Thot."),
                    }
                }
                Instruction::Sedjem { destination } => {
                    if destination == "ka" {
                        // Le Scribe du BIOS :
                        // Le BIOS met le CPU en pause, lit les impulsions électriques,
                        // les traduit en vrai code ASCII (A, B, C...) et place le résultat dans AL.
                        code_actuel.extend_from_slice(&[0xB4, 0x00]); // MOV AH, 0x00 (Attendre une touche)
                        code_actuel.extend_from_slice(&[0xCD, 0x16]); // INT 0x16 (Appel BIOS Clavier)
                    }
                }
                Instruction::Henet {
                    destination,
                    valeur,
                } => {
                    code_actuel.push(0x66); // Stabilisation 32 bits
                    code_actuel.push(0x81); // Opcode groupe logique
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xE0, // EAX
                        "ib" => 0xE1, // ECX
                        "da" => 0xE2, // EDX (Celui qui manquait !)
                        "ba" => 0xE3, // EBX
                        "si" => 0xE6, // ESI (Nouveau V5)
                        "di" => 0xE7, // EDI (Nouveau V5)
                        _ => panic!("Registre inconnu pour henet : {}", destination),
                    };
                    code_actuel.push(modrm);
                    if let Expression::Number(n) = valeur {
                        code_actuel.extend_from_slice(&n.to_le_bytes()); //
                    }
                }
                Instruction::Mer {
                    destination,
                    valeur,
                } => {
                    code_actuel.push(0x66);
                    code_actuel.push(0x81);
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xC8, // EAX
                        "ib" => 0xC9, // ECX
                        "da" => 0xCA, // EDX
                        "ba" => 0xCB, // EBX
                        "si" => 0xCE, // ESI
                        "di" => 0xCF, // EDI
                        _ => panic!("Registre inconnu pour mer : {}", destination),
                    };
                    code_actuel.push(modrm);
                    if let Expression::Number(n) = valeur {
                        code_actuel.extend_from_slice(&n.to_le_bytes());
                    }
                }
                Instruction::Return { resultat } => {
                    match resultat {
                        Expression::Number(n) => {
                            // MOV EAX, n (Opcode 0xB8)
                            code_actuel.push(0x66); // LA PROTECTION V4
                            code_actuel.push(0xB8);
                            code_actuel.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(r) => {
                            if r != "ka" {
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
                    code_actuel.push(0xC3);
                }
                Instruction::Wab => {
                    // Le Sortilège du Vide : MOV AX, 0x0003 puis INT 0x10
                    code_actuel.extend_from_slice(&[0xB8, 0x03, 0x00, 0xCD, 0x10]);
                }

                Instruction::Per { message } => {
                    match message {
                        Expression::StringLiteral(s) => {
                            // 1. On enregistre le texte dans le Noun (Secteur 3+)
                            let addr = curseur_noun;
                            segment_noun.extend_from_slice(s.as_bytes());
                            segment_noun.push(0); // Signe du Silence
                            curseur_noun += (s.len() + 1) as u16;

                            // 2. On génère le code machine pour l'afficher
                            code_actuel.push(0xBE); // MOV SI, adresse_du_texte
                            code_actuel.extend_from_slice(&addr.to_le_bytes());

                            code_actuel.push(0xE8); // CALL std_print
                            sauts_a_patcher.push((
                                code_actuel.len(),
                                Expression::Identifier("std_print".to_string()),
                                dans_noyau,
                            ));
                            code_actuel.extend_from_slice(&[0x00, 0x00]);
                        }
                        _ => { /* Gestion des registres si besoin */ }
                    }
                }
                Instruction::Label(nom) => {
                    // On utilise base_actuelle (0x7C00 ou 0x7E00) au lieu de base_addr !
                    labels.insert(nom.clone(), (base_actuelle + code_actuel.len()) as isize);
                }
                Instruction::Wdj { left, right } => {
                    // 1. On force le mode 32 bits pour la précision
                    code_actuel.push(0x66);
                    // 2. OpCode universel de comparaison : 0x81
                    match right {
                        Expression::Number(n) => {
                            // MODE 1 : Comparer à un Nombre (CMP r/m32, imm32)
                            code_actuel.push(0x81); // OpCode universel avec nombre
                            let modrm: u8 = match left.as_str() {
                                "ka" => 0xF8,
                                "ib" => 0xF9,
                                "da" => 0xFA,
                                "ba" => 0xFB,
                                "si" => 0xFE,
                                "di" => 0xFF,
                                _ => panic!("Registre inconnu pour la balance : {left}"),
                            };
                            code_actuel.push(modrm);
                            code_actuel.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(right_reg) => {
                            // MODE 2 : Comparer à un autre Registre (CMP r/m32, reg32)
                            code_actuel.push(0x39); // OpCode pour CMP registre à registre

                            // On récupère le code numérique des deux registres (0 à 7)
                            let dest_code = match left.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                "si" => 6,
                                "di" => 7,
                                _ => panic!("Registre de gauche inconnu : {left}"),
                            };
                            let src_code = match right_reg.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                "si" => 6,
                                "di" => 7,
                                _ => panic!("Registre de droite inconnu : {right_reg}"),
                            };

                            // Formule x86 magique (ModR/M) : 0xC0 (11000000) + (source * 8) + destination
                            let modrm = 0xC0 | (src_code << 3) | dest_code;
                            code_actuel.push(modrm);
                        }
                        _ => panic!("La Balance ne sait peser que des nombres ou des registres."),
                    }
                }
                // Traduction de : ankh cible (Saut Conditionnel : JE)
                Instruction::Ankh { cible } => {
                    // OpCode pour JE near (Sauter si Égal, relatif 16-bit)
                    code_actuel.push(0x0F);
                    code_actuel.push(0x84);

                    // On enregistre l'endroit à patcher (comme pour neheh)
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));

                    // Placeholders
                    code_actuel.push(0x00);
                    code_actuel.push(0x00);
                }
                Instruction::Duat { phrase, adresse } => {
                    for (i, c) in phrase.chars().enumerate() {
                        // Opcode 0xC6 0x06 = MOV [imm16], imm8
                        code_actuel.push(0xC6);
                        code_actuel.push(0x06);
                        let addr_actuelle = adresse + i as u16;
                        code_actuel.extend_from_slice(&addr_actuelle.to_le_bytes());
                        code_actuel.push(c as u8);
                    }
                    // AJOUT AUTOMATIQUE DU ZÉRO DE FIN
                    code_actuel.push(0xC6);
                    code_actuel.push(0x06);
                    let addr_zero = adresse + phrase.len() as u16;
                    code_actuel.extend_from_slice(&addr_zero.to_le_bytes());
                    code_actuel.push(0x00);
                }
                // Traduction de : kheb %registre, valeur (SUB)
                Instruction::Kheb {
                    destination,
                    valeur,
                } => {
                    code_actuel.push(0x66); // Stabilisation 32 bits

                    match valeur {
                        Expression::Number(n) => {
                            // MODE 1 : Soustraire un Nombre (SUB r/m32, imm32)
                            code_actuel.push(0x81);
                            let modrm: u8 = match destination.as_str() {
                                "ka" => 0xE8,
                                "ib" => 0xE9,
                                "da" => 0xEA,
                                "ba" => 0xEB,
                                "si" => 0xEE,
                                "di" => 0xEF,
                                _ => panic!("Registre inconnu pour kheb : {destination}"),
                            };
                            code_actuel.push(modrm);
                            code_actuel.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(src) => {
                            // MODE 2 : Soustraire un Registre (SUB r/m32, reg32)
                            code_actuel.push(0x29); // OpCode SUB registre à registre

                            let dest_code = match destination.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                "si" => 6,
                                "di" => 7,
                                _ => panic!("Registre destination inconnu : {}", destination),
                            };
                            let src_code = match src.as_str() {
                                "ka" => 0,
                                "ib" => 1,
                                "da" => 2,
                                "ba" => 3,
                                "si" => 6,
                                "di" => 7,
                                _ => panic!("Registre source inconnu : {}", src),
                            };

                            let modrm = 0xC0 | (src_code << 3) | dest_code;
                            code_actuel.push(modrm);
                        }
                        _ => panic!("Kheb ne supporte que les nombres ou les registres."),
                    }
                }
                Instruction::HerAnkh { cible } => {
                    // OpCode pour JGE (Jump if Greater or Equal)
                    code_actuel.push(0x0F);
                    code_actuel.push(0x8D);

                    // On enregistre l'emplacement pour le patcher plus tard
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));

                    // On laisse 2 octets vides pour la distance (le "trou" à patcher)
                    code_actuel.extend_from_slice(&[0x00, 0x00]);
                }
                Instruction::KherAnkh { cible } => {
                    // OpCode pour JLE (Jump if Less or Equal)
                    code_actuel.push(0x0F);
                    code_actuel.push(0x8E);

                    // On enregistre l'emplacement pour le patcher plus tard
                    sauts_a_patcher.push((code_actuel.len(), cible.clone(), dans_noyau));

                    // On laisse 2 octets vides pour la distance
                    code_actuel.extend_from_slice(&[0x00, 0x00]);
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
        if is_bootloader {
            labels.insert(
                "std_print".to_string(),
                (base_stage2 + stage2_code.len()) as isize,
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
            labels.insert("print_hex_32".to_string(), (base_stage2 + stage2_code.len()) as isize);

            let routine_hex = vec![
                0x66, 0x60,             // PUSHAD : Protège tous les registres de ton OS
                0xB9, 0x08, 0x00,       // MOV CX, 8 : On va extraire 8 caractères (32 bits = 8x4 bits)

                // --- loop_start: ---
                0x66, 0xC1, 0xC0, 0x04, // ROL EAX, 4 : Fait tourner le registre de 4 bits
                0x66, 0x50,             // PUSH EAX : Sauvegarde temporaire
                0x24, 0x0F,             // AND AL, 0x0F : Isole les 4 bits tout à droite

                // --- La Magie de Conversion (0-15 -> '0'-'9', 'A'-'F') ---
                0x04, 0x90,             // ADD AL, 0x90
                0x27,                   // DAA
                0x14, 0x40,             // ADC AL, 0x40
                0x27,                   // DAA

                // --- Appel BIOS ---
                0xB4, 0x0E,             // MOV AH, 0x0E
                0xCD, 0x10,             // INT 0x10 : Affiche le caractère

                0x66, 0x58,             // POP EAX : Récupère la valeur pour le prochain tour
                0xE2, 0xEA,             // LOOP loop_start : Recommence (Saute 22 octets en arrière)

                0x66, 0x61,             // POPAD : Restaure les registres de l'OS
                0xC3                    // RET : Retourne au programme principal
            ];
            stage2_code.extend_from_slice(&routine_hex);
        }
        // --- LE PATCHING ---
        for (offset, cible, est_noyau) in sauts_a_patcher {
            let base = if est_noyau { base_stage2 } else { base_stage1 };
            let buffer = if est_noyau {
                &mut stage2_code
            } else {
                &mut stage1_code
            };
            if let Expression::Identifier(nom) = cible {
                let addr = *labels.get(&nom).expect("Label manquant");
                let dist = addr - (base + offset + 2) as isize;
                let b = (dist as i16).to_le_bytes();
                buffer[offset] = b[0];
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
            binaire_final.extend(segment_noun);

            // On s'assure que le fichier total est un multiple de 512 pour le BIOS
            while binaire_final.len() % 512 != 0 {
                binaire_final.push(0);
            }
        } else {
            // Mode Classique (Tests unitaires ou fichiers ELF Linux) : Zéro padding
            // On colle juste le code pur bout à bout
            binaire_final.extend(stage2_code);
            binaire_final.extend(segment_noun);
        }

        binaire_final
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expression, Instruction};

    #[test]
    fn test_generer_binaire_push_pop() {
        let ast = vec![
            Instruction::Push {
                cible: Expression::Register("ka".to_string()),
            },
            Instruction::Pop {
                destination: "ib".to_string(),
            },
        ];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // PUSH %ka (EAX) -> 0x66 (Protection 32-bit), 0x50
        // POP %ib (ECX)  -> 0x66 (Protection 32-bit), 0x59
        assert_eq!(binaire, vec![0x66, 0x50, 0x66, 0x59]);
    }
    #[test]
    fn test_emitter_smen_est_silencieux() {
        let ast = vec![Instruction::Smen {
            nom: "X".to_string(),
            valeur: 100,
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // Une constante ne doit produire aucun code machine x86
        assert!(binaire.is_empty());
    }
    #[test]
    fn test_generer_binaire_in_out() {
        let ast = vec![
            Instruction::In {
                port: Expression::Number(96),
            },
            Instruction::Out {
                port: Expression::Register("da".to_string()),
            },
        ];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // IN 96 (Lit depuis le port 0x60 vers AL) -> 0xE4, 0x60
        // OUT %da (Écrit AL vers le port contenu dans DX) -> 0xEE
        assert_eq!(binaire, vec![0xE4, 0x60, 0xEE]);
    }
    #[test]
    fn test_generer_binaire_henek() {
        let ast = vec![Instruction::Henek {
            destination: "ka".to_string(),
            valeur: Expression::Number(10),
        }];

        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // 0x66 = Préfixe 32-bit
        // 0xB8 = MOV EAX
        // 0x0A, 0x00, 0x00, 0x00 = 10 en Little-Endian
        assert_eq!(binaire, vec![0x66, 0xB8, 0x0A, 0x00, 0x00, 0x00]);
    }
    #[test]
    fn test_generer_binaire_sema() {
        let ast = vec![Instruction::Sema {
            destination: "ka".to_string(),
            valeur: Expression::Number(5),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // 0x66 = Préfixe
        // 0x81 0xC0 = ADD EAX
        // 0x05 0x00 0x00 0x00 = Le chiffre 5
        assert_eq!(binaire, vec![0x66, 0x81, 0xC0, 0x05, 0x00, 0x00, 0x00]);
    }
    #[test]
    fn test_generer_binaire_return() {
        let ast = vec![Instruction::Return {
            resultat: Expression::Number(0),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // 0x66 0xB8 00 00 00 00 = MOV EAX, 0
        // 0xC3 = RET
        assert_eq!(binaire, vec![0x66, 0xB8, 0x00, 0x00, 0x00, 0x00, 0xC3]);
    }
    #[test]
    fn test_generer_binaire_wdj() {
        // wdj %ib, 0 -> On s'attend à CMP ECX, 0
        let ast = vec![Instruction::Wdj {
            left: "ib".to_string(),
            right: Expression::Number(0),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // 0x66 (Prefix) 0x81 0xF9 (CMP ECX) 00 00 00 00 (Valeur)
        assert_eq!(binaire, vec![0x66, 0x81, 0xF9, 0x00, 0x00, 0x00, 0x00]);
    }
    #[test]
    fn test_generer_binaire_wdj_registres() {
        // Test : wdj %ka, %ib -> Doit générer CMP EAX, ECX
        let ast = vec![Instruction::Wdj {
            left: "ka".to_string(),
            right: Expression::Register("ib".to_string()),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire(false);

        // 0x66 = Préfixe 32 bits pour la stabilité
        // 0x39 = OpCode pour CMP registre à registre
        // 0xC8 = Octet ModR/M fusionnant ECX (source) et EAX (destination)
        assert_eq!(binaire, vec![0x66, 0x39, 0xC8]);
    }
}
