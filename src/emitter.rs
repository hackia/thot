use crate::ast::{Expression, Instruction};

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
    pub fn generer_binaire(&self) -> Vec<u8> {
        let mut code_machine: Vec<u8> = Vec::new();
        let mut labels = std::collections::HashMap::new();
        let mut sauts_a_patcher = Vec::new();
        let mut variables = std::collections::HashMap::new();
        let mut prochaine_adresse_ram: u16 = 8192;
        for instruction in &self.instructions {
            match instruction {
                // Traduction de : henek %registre, valeur
                Instruction::Henek {
                    destination,
                    valeur,
                } => {
                    code_machine.push(0x66); // Préfixe pour forcer le mode 32 bits !
                    // 1. Mapping du Registre Sacré vers le Registre Physique (x86_64)
                    // Dans x86, l'instruction "MOV registre, valeur" commence à l'octet 0xB8
                    // On ajoute un code selon le registre cible.
                    let code_registre: u8 = match destination.as_str() {
                        "ka" => 0x00, // EAX
                        "ib" => 0x01, // ECX
                        "da" => 0x02, // EDX (Nouveau !)
                        "ba" => 0x03, // EBX
                        "si" => 0x06, // ESI (Nouveau !)
                        "di" => 0x07, // EDI (Nouveau !)
                        _ => panic!("Registre inconnu : {}", destination),
                    };

                    // L'Opcode principal (0xB8 + 0 = 0xB8 pour EAX)
                    let opcode_mov = 0xB8 + code_registre;
                    code_machine.push(opcode_mov);

                    // 2. Écriture de la valeur
                    match valeur {
                        Expression::Number(n) => {
                            let octets_valeur = n.to_le_bytes();
                            code_machine.extend_from_slice(&octets_valeur);
                        }
                        Expression::Identifier(nom) => {
                            // NOUVEAU : Thot cherche l'adresse de la variable dans son Registre
                            let adresse = *variables.get(nom).expect(&format!(
                                "Erreur fatale : La variable '{}' n'existe pas !",
                                nom
                            ));

                            // On convertit l'adresse (u16) en 32 bits (u32) pour remplir le registre
                            let octets_valeur = (adresse as u32).to_le_bytes();
                            code_machine.extend_from_slice(&octets_valeur);
                        }
                        _ => panic!(
                            "L'Émetteur ne sait gérer que des nombres purs ou des noms de variables connus pour l'instant."
                        ),
                    }
                }
                Instruction::Push { cible } => {
                    match cible {
                        Expression::Register(r) => {
                            code_machine.push(0x66); // Protection 32-bit
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
                            code_machine.push(opcode);
                        }
                        Expression::Number(n) => {
                            code_machine.push(0x66); // Protection 32-bit
                            code_machine.push(0x68); // OpCode PUSH imm32
                            code_machine.extend_from_slice(&n.to_le_bytes());
                        }
                        _ => panic!("Push ne supporte que les registres et les nombres."),
                    }
                }
                Instruction::Pop { destination } => {
                    code_machine.push(0x66); // Protection 32-bit
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
                    code_machine.push(opcode);
                }
                Instruction::In { port } => {
                    // Lecture matérielle (toujours vers AL - 8 bits)
                    match port {
                        Expression::Number(n) => {
                            // IN AL, imm8 (Lit le port direct)
                            code_machine.push(0xE4);
                            code_machine.push(*n as u8);
                        }
                        Expression::Register(r) if r == "da" => {
                            // IN AL, DX (Lit le port contenu dans %da)
                            code_machine.push(0xEC);
                        }
                        _ => panic!("Le port IN doit être un nombre ou le registre %da"),
                    }
                }
                Instruction::Out { port } => {
                    // Écriture matérielle (toujours depuis AL - 8 bits)
                    match port {
                        Expression::Number(n) => {
                            // OUT imm8, AL (Écrit vers le port direct)
                            code_machine.push(0xE6);
                            code_machine.push(*n as u8);
                        }
                        Expression::Register(r) if r == "da" => {
                            // OUT DX, AL (Écrit vers le port contenu dans %da)
                            code_machine.push(0xEE);
                        }
                        _ => panic!("Le port OUT doit être un nombre ou le registre %da"),
                    }
                }
                Instruction::Her { cible } => {
                    code_machine.push(0x0F);
                    code_machine.push(0x8F); // OpCode pour JG (Saut si plus grand)
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));
                    code_machine.extend_from_slice(&[0x00, 0x00]);
                }
                Instruction::Kher { cible } => {
                    code_machine.push(0x0F);
                    code_machine.push(0x8C); // OpCode pour JL (Saut si plus petit)
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));
                    code_machine.extend_from_slice(&[0x00, 0x00]);
                }
                // Traduction de : isfet cible (Saut Conditionnel : JNE)
                Instruction::Isfet { cible } => {
                    // OpCode pour JNE near (Sauter si Différent, relatif 16-bit)
                    code_machine.push(0x0F);
                    code_machine.push(0x85);

                    // On enregistre l'endroit à patcher EXACTEMENT comme pour ankh
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));

                    // Placeholders (les zéros temporels)
                    code_machine.push(0x00);
                    code_machine.push(0x00);
                }
                Instruction::Kheper { source, adresse } => {
                    if source != "ka" {
                        panic!("Seul %ka peut écrire en RAM pour l'instant.");
                    }
                    match adresse {
                        Expression::Number(n) => {
                            code_machine.push(0xA2);
                            code_machine.extend_from_slice(&(*n as u16).to_le_bytes());
                        }
                        Expression::Register(r) if r == "ba" => {
                            code_machine.extend_from_slice(&[0x67, 0x88, 0x03]);
                        }
                        Expression::Identifier(nom) => {
                            // NOUVEAU : Sauvegarde directe dans la variable
                            let adresse = *variables
                                .get(nom)
                                .expect(&format!("Erreur : Variable '{}' introuvable", nom));
                            code_machine.push(0xA2); // OpCode MOV [moffs8], AL
                            code_machine.extend_from_slice(&(adresse as u16).to_le_bytes());
                        }
                        _ => panic!(
                            "Le Scribe ne sait pointer qu'avec %ba, une adresse fixe, ou une variable."
                        ),
                    }
                }
                // Traduction de : sema %registre, valeur (ADD)
                Instruction::Sema {
                    destination,
                    valeur,
                } => {
                    code_machine.push(0x66); // Stabilisation 32 bits

                    match valeur {
                        Expression::Number(n) => {
                            // MODE 1 : Additionner un Nombre (ADD r/m32, imm32)
                            code_machine.push(0x81); // Opcode ADD
                            let modrm: u8 = match destination.as_str() {
                                "ka" => 0xC0,
                                "ib" => 0xC1,
                                "da" => 0xC2,
                                "ba" => 0xC3,
                                "si" => 0xC6,
                                "di" => 0xC7,
                                _ => panic!("Registre inconnu pour sema : {}", destination),
                            };
                            code_machine.push(modrm);
                            code_machine.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(src) => {
                            // MODE 2 : Additionner un Registre (ADD r/m32, reg32)
                            code_machine.push(0x01); // Opcode ADD registre à registre

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
                                _ => panic!("Registre source inconnu : {}", src),
                            };

                            // Formule x86 magique (ModR/M) : 0xC0 (11000000 en binaire) + (source * 8) + destination
                            let modrm = 0xC0 | (src_code << 3) | dest_code;
                            code_machine.push(modrm);
                        }
                        _ => panic!("Sema ne supporte que les nombres ou les registres."),
                    }
                }
                Instruction::Sena {
                    destination,
                    adresse,
                } => {
                    if destination != "ka" {
                        panic!("Le Transmetteur ne gère que %ka pour lire en RAM pour l'instant.");
                    }
                    match adresse {
                        Expression::Number(n) => {
                            code_machine.push(0xA0);
                            code_machine.extend_from_slice(&(*n as u16).to_le_bytes());
                        }
                        Expression::Register(r) if r == "ba" => {
                            code_machine.extend_from_slice(&[0x67, 0x8A, 0x03]);
                        }
                        Expression::Identifier(nom) => {
                            // NOUVEAU : Lecture directe depuis la variable
                            let adresse = *variables
                                .get(nom)
                                .expect(&format!("Erreur : Variable '{}' introuvable", nom));
                            code_machine.push(0xA0); // OpCode MOV AL, [moffs8]
                            code_machine.extend_from_slice(&(adresse as u16).to_le_bytes());
                        }
                        _ => panic!(
                            "Le Scribe ne sait lire la RAM qu'avec %ba, une adresse fixe, ou une variable."
                        ),
                    }
                }
                Instruction::Sedjem { destination } => {
                    if destination == "ka" {
                        // Le Scribe du BIOS :
                        // Le BIOS met le CPU en pause, lit les impulsions électriques,
                        // les traduit en vrai code ASCII (A, B, C...) et place le résultat dans AL.
                        code_machine.extend_from_slice(&[0xB4, 0x00]); // MOV AH, 0x00 (Attendre une touche)
                        code_machine.extend_from_slice(&[0xCD, 0x16]); // INT 0x16 (Appel BIOS Clavier)
                    }
                }
                Instruction::Jena { cible } => {
                    // OpCode pour CALL near (Saut relatif 16-bit avec sauvegarde de l'adresse de retour)
                    code_machine.push(0xE8);

                    // On enregistre l'endroit à patcher EXACTEMENT comme pour Neheh ou Ankh
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));

                    // Placeholders
                    code_machine.push(0x00);
                    code_machine.push(0x00);
                }
                Instruction::Henet {
                    destination,
                    valeur,
                } => {
                    code_machine.push(0x66); // Stabilisation 32 bits
                    code_machine.push(0x81); // Opcode groupe logique
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xE0, // EAX
                        "ib" => 0xE1, // ECX
                        "da" => 0xE2, // EDX (Celui qui manquait !)
                        "ba" => 0xE3, // EBX
                        "si" => 0xE6, // ESI (Nouveau V5)
                        "di" => 0xE7, // EDI (Nouveau V5)
                        _ => panic!("Registre inconnu pour henet : {}", destination),
                    };
                    code_machine.push(modrm);
                    if let Expression::Number(n) = valeur {
                        code_machine.extend_from_slice(&n.to_le_bytes()); //
                    }
                }
                Instruction::Mer {
                    destination,
                    valeur,
                } => {
                    code_machine.push(0x66);
                    code_machine.push(0x81);
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xC8, // EAX
                        "ib" => 0xC9, // ECX
                        "da" => 0xCA, // EDX
                        "ba" => 0xCB, // EBX
                        "si" => 0xCE, // ESI
                        "di" => 0xCF, // EDI
                        _ => panic!("Registre inconnu pour mer : {}", destination),
                    };
                    code_machine.push(modrm);
                    if let Expression::Number(n) = valeur {
                        code_machine.extend_from_slice(&n.to_le_bytes());
                    }
                }
                Instruction::Return { resultat } => {
                    match resultat {
                        Expression::Number(n) => {
                            // MOV EAX, n (Opcode 0xB8)
                            code_machine.push(0x66); // LA PROTECTION V4
                            code_machine.push(0xB8);
                            code_machine.extend_from_slice(&n.to_le_bytes());
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
                    code_machine.push(0xC3);
                }
                Instruction::Wab => {
                    // Le Sortilège du Vide : MOV AX, 0x0003 puis INT 0x10
                    code_machine.extend_from_slice(&[0xB8, 0x03, 0x00, 0xCD, 0x10]);
                }
                Instruction::Per { message } => {
                    match message {
                        Expression::StringLiteral(s) => {
                            // FINI LE NETTOYAGE ICI ! On ne garde QUE l'écriture propre avec le BIOS.
                            for c in s.chars() {
                                code_machine.extend_from_slice(&[0xB4, 0x0E]); // MOV AH, 0x0E (Teletype)
                                code_machine.push(0xB0); // MOV AL, caractère
                                code_machine.push(c as u8);
                                code_machine.extend_from_slice(&[0xCD, 0x10]); // INT 0x10
                            }
                        }
                        Expression::Register(r) => {
                            if r == "ka" {
                                let affichage_propre: Vec<u8> = vec![
                                    0x66,
                                    0x53, // PUSH EBX : On sauve ton adresse (1000, 1001...)
                                    0xB4, 0x0E, // MOV AH, 0x0E
                                    0xBB, 0x0F,
                                    0x00, // MOV BX, 0x000F (Ici on peut l'écraser, c'est sauvé !)
                                    0xCD, 0x10, // INT 0x10
                                    0x3C, 0x0D, // Gestion de l'Entrée...
                                    0x75, 0x04, 0xB0, 0x0A, 0xCD, 0x10, 0x66,
                                    0x5B, // POP EBX : On remet l'adresse intacte dans le registre !
                                ];
                                code_machine.extend_from_slice(&affichage_propre);
                            }
                        }
                        _ => {}
                    }
                }
                Instruction::Label(nom) => {
                    // On retient l'octet exact où se trouve cette étiquette
                    labels.insert(nom.clone(), code_machine.len());
                }

                // 2. Génération du saut
                Instruction::Neheh { cible } => {
                    // OpCode pour JMP near (Saut relatif 16-bit)
                    code_machine.push(0xE9);

                    // On enregistre l'endroit où on doit écrire la distance, et la cible voulue
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));

                    // On met des zéros temporaires (placeholder) qui seront écrasés plus tard
                    code_machine.push(0x00);
                    code_machine.push(0x00);
                }
                // Traduction de : wdj %registre, valeur (CMP AL, imm8)
                // Traduction de : wdj %registre, valeur (CMP reg32, imm32)
                Instruction::Wdj { left, right } => {
                    // 1. On force le mode 32 bits pour la précision
                    code_machine.push(0x66);
                    // 2. OpCode universel de comparaison : 0x81
                    match right {
                        Expression::Number(n) => {
                            // MODE 1 : Comparer à un Nombre (CMP r/m32, imm32)
                            code_machine.push(0x81); // OpCode universel avec nombre
                            let modrm: u8 = match left.as_str() {
                                "ka" => 0xF8,
                                "ib" => 0xF9,
                                "da" => 0xFA,
                                "ba" => 0xFB,
                                "si" => 0xFE,
                                "di" => 0xFF,
                                _ => panic!("Registre inconnu pour la balance : {left}"),
                            };
                            code_machine.push(modrm);
                            code_machine.extend_from_slice(&n.to_le_bytes());
                        }
                        Expression::Register(right_reg) => {
                            // MODE 2 : Comparer à un autre Registre (CMP r/m32, reg32)
                            code_machine.push(0x39); // OpCode pour CMP registre à registre

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
                            code_machine.push(modrm);
                        }
                        _ => panic!("La Balance ne sait peser que des nombres ou des registres."),
                    }
                }
                // Traduction de : ankh cible (Saut Conditionnel : JE)
                Instruction::Ankh { cible } => {
                    // OpCode pour JE near (Sauter si Égal, relatif 16-bit)
                    code_machine.push(0x0F);
                    code_machine.push(0x84);

                    // On enregistre l'endroit à patcher (comme pour neheh)
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));

                    // Placeholders
                    code_machine.push(0x00);
                    code_machine.push(0x00);
                }
                Instruction::Duat { phrase, adresse } => {
                    for (i, c) in phrase.chars().enumerate() {
                        // Opcode 0xC6 0x06 = MOV [imm16], imm8
                        code_machine.push(0xC6);
                        code_machine.push(0x06);
                        let addr_actuelle = adresse + i as u16;
                        code_machine.extend_from_slice(&addr_actuelle.to_le_bytes());
                        code_machine.push(c as u8);
                    }
                    // AJOUT AUTOMATIQUE DU ZÉRO DE FIN
                    code_machine.push(0xC6);
                    code_machine.push(0x06);
                    let addr_zero = adresse + phrase.len() as u16;
                    code_machine.extend_from_slice(&addr_zero.to_le_bytes());
                    code_machine.push(0x00);
                }
                // Traduction de : kheb %registre, valeur (SUB reg32, imm32)
                // Traduction de : kheb %registre, valeur (SUB reg32, imm32)
                Instruction::Kheb {
                    destination,
                    valeur,
                } => {
                    // 1. Protection 32 bits pour la stabilité du CPU
                    code_machine.push(0x66);

                    // 2. OpCode de base pour les opérations mathématiques étendues
                    code_machine.push(0x81);

                    // 3. Mapping complet des registres pour la soustraction (/5)
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xE8, // EAX
                        "ib" => 0xE9, // ECX
                        "da" => 0xEA, // EDX (Nouveau V5)
                        "ba" => 0xEB, // EBX
                        "si" => 0xEE, // ESI (Nouveau V5)
                        "di" => 0xEF, // EDI (Nouveau V5)
                        _ => panic!("Registre inconnu pour kheb : {}", destination),
                    };
                    code_machine.push(modrm);

                    // 4. Écriture de la valeur à soustraire
                    match valeur {
                        Expression::Number(n) => {
                            let octets_valeur = n.to_le_bytes();
                            code_machine.extend_from_slice(&octets_valeur);
                        }
                        _ => panic!("Le Scribe ne sait soustraire que des nombres pour l'instant."),
                    }
                }
                Instruction::HerAnkh { cible } => {
                    // OpCode pour JGE (Jump if Greater or Equal)
                    code_machine.push(0x0F);
                    code_machine.push(0x8D);

                    // On enregistre l'emplacement pour le patcher plus tard
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));

                    // On laisse 2 octets vides pour la distance (le "trou" à patcher)
                    code_machine.extend_from_slice(&[0x00, 0x00]);
                }
                Instruction::KherAnkh { cible } => {
                    // OpCode pour JLE (Jump if Less or Equal)
                    code_machine.push(0x0F);
                    code_machine.push(0x8E);

                    // On enregistre l'emplacement pour le patcher plus tard
                    sauts_a_patcher.push((code_machine.len(), cible.clone()));

                    // On laisse 2 octets vides pour la distance
                    code_machine.extend_from_slice(&[0x00, 0x00]);
                }
                Instruction::Dema { chemin } => {
                    panic!(
                        "Erreur fatale de Maât : L'Émetteur a trouvé une instruction 'dema' pointant vers '{}'. Le Tisserand a oublié de fusionner cette tablette avant la génération du binaire !",
                        chemin
                    );
                }
                // Traduction de : nama nom = valeur
                Instruction::Nama { nom, valeur } => {
                    // 1. ALLOCATION MÉMOIRE (Où va-t-on écrire ?)
                    let adresse = if let Some(addr) = variables.get(nom) {
                        *addr // La variable existe déjà, on réutilise son adresse
                    } else {
                        let addr = prochaine_adresse_ram;
                        variables.insert(nom.clone(), addr);
                        // On réserve 4 octets (32 bits) pour un nombre
                        prochaine_adresse_ram += 4;
                        addr
                    };

                    // 2. ÉCRITURE PHYSIQUE (Génération des Opcodes)
                    match valeur {
                        Expression::Number(n) => {
                            // Étape A : On place le nombre dans l'Accumulateur %ka (EAX)
                            code_machine.push(0x66); // Mode 32-bit
                            code_machine.push(0xB8); // OpCode MOV EAX, imm32
                            code_machine.extend_from_slice(&n.to_le_bytes());

                            // Étape B : On sauvegarde EAX dans la RAM à l'adresse allouée
                            // 0xA3 est l'OpCode magique pour : MOV [adresse_16_bits], EAX
                            code_machine.push(0x66);
                            code_machine.push(0xA3);
                            code_machine.extend_from_slice(&adresse.to_le_bytes());
                        }
                        Expression::StringLiteral(s) => {
                            // Si c'est une phrase (ex: nama titre = "Amentys")
                            // On écrit lettre par lettre comme pour l'instruction 'duat'
                            for (i, c) in s.chars().enumerate() {
                                code_machine.push(0xC6); // MOV [imm16], imm8
                                code_machine.push(0x06);
                                let addr_actuelle = adresse + (i as u16);
                                code_machine.extend_from_slice(&addr_actuelle.to_le_bytes());
                                code_machine.push(c as u8);
                            }
                            // On n'oublie pas le zéro de fin (Signe du Silence)
                            code_machine.push(0xC6);
                            code_machine.push(0x06);
                            let addr_zero = adresse + s.len() as u16;
                            code_machine.extend_from_slice(&addr_zero.to_le_bytes());
                            code_machine.push(0x00);
                            // On met à jour l'espace total occupé par la phrase
                            // pour que la prochaine variable ne l'écrase pas !
                            prochaine_adresse_ram = addr_zero + 1;
                        }
                        _ => panic!(
                            "Le Scribe ne sait assigner que des nombres ou des phrases pour l'instant."
                        ),
                    }
                }
                Instruction::Smen { .. } => {}
            }
        }
        // --- LA PASSE DE PATCH (Résolution des Sauts) ---
        for (offset_du_trou, cible) in sauts_a_patcher {
            // On cherche où se trouve vraiment l'étiquette
            let adresse_cible = labels
                .get(&cible)
                .expect(&format!("Erreur fatale : Étiquette '{cible}' introuvable",));

            // Le processeur calcule la distance à partir de l'octet *suivant* l'instruction complète (offset + 2)
            let distance = (*adresse_cible as isize) - ((offset_du_trou + 2) as isize);

            // On convertit la distance en 16-bit (Little-Endian)
            let bytes_distance = (distance as i16).to_le_bytes();

            // On remonte dans le temps pour écraser les zéros avec la vraie distance
            code_machine[offset_du_trou] = bytes_distance[0];
            code_machine[offset_du_trou + 1] = bytes_distance[1];
        }
        code_machine
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
        let binaire = emetteur.generer_binaire();

        // PUSH %ka (EAX) -> 0x66 (Protection 32-bit), 0x50
        // POP %ib (ECX)  -> 0x66 (Protection 32-bit), 0x59
        assert_eq!(binaire, vec![0x66, 0x50, 0x66, 0x59]);
    }
    #[test]
    fn test_emitter_smen_est_silencieux() {
        let ast = vec![Instruction::Smen {
            nom: "X".to_string(),
            valeur: 100
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire();

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
        let binaire = emetteur.generer_binaire();

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
        let binaire = emetteur.generer_binaire();

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
        let binaire = emetteur.generer_binaire();

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
        let binaire = emetteur.generer_binaire();

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
        let binaire = emetteur.generer_binaire();

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
        let binaire = emetteur.generer_binaire();

        // 0x66 = Préfixe 32 bits pour la stabilité
        // 0x39 = OpCode pour CMP registre à registre
        // 0xC8 = Octet ModR/M fusionnant ECX (source) et EAX (destination)
        assert_eq!(binaire, vec![0x66, 0x39, 0xC8]);
    }
}
