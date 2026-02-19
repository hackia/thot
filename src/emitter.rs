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
                        "ka" => 0x00, // %ka devient EAX (l'accumulateur mathématique)
                        "ib" => 0x01, // %ib devient ECX (le compteur/cœur)
                        "ba" => 0x03, // %ba devient EBX (la base mémoire)
                        _ => panic!("Fatal error : unknow registre '{destination}'"),
                    };

                    // L'Opcode principal (0xB8 + 0 = 0xB8 pour EAX)
                    let opcode_mov = 0xB8 + code_registre;
                    code_machine.push(opcode_mov);

                    // 2. Écriture de la valeur
                    match valeur {
                        Expression::Number(n) => {
                            // Les processeurs lisent les chiffres à l'envers (Little-Endian).
                            // Ex: 10 (0x0000000A) devient [0x0A, 0x00, 0x00, 0x00]
                            let octets_valeur = n.to_le_bytes();
                            code_machine.extend_from_slice(&octets_valeur);
                        }
                        _ => {
                            panic!(
                                "The Transmitter only knows how to handle raw numbers at the moment"
                            )
                        }
                    }
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
                            // Mode Direct (Ancien)
                            code_machine.push(0xA2);
                            code_machine.extend_from_slice(&(*n as u16).to_le_bytes());
                        }
                        Expression::Register(r) if r == "ba" => {
                            // Mode Indirect : MOV [EBX], AL
                            // 0x67 = Prefixe adresse 32-bit, 0x88 = MOV r/m8, r8, 0x03 = [EBX]
                            code_machine.extend_from_slice(&[0x67, 0x88, 0x03]);
                        }
                        _ => panic!("Le Scribe ne sait pointer qu'avec %ba ou une adresse fixe."),
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
                            // MODE DIRECT (Ancien) : MOV AL, [adresse_fixe]
                            code_machine.push(0xA0); // OpCode 0xA0
                            code_machine.extend_from_slice(&(*n as u16).to_le_bytes());
                        }
                        Expression::Register(r) if r == "ba" => {
                            // MODE INDIRECT (Pointeur) : MOV AL, [EBX]
                            // 0x67 = Préfixe adresse 32-bit
                            // 0x8A = MOV r8, r/m8
                            // 0x03 = ModR/M pour [EBX]
                            code_machine.extend_from_slice(&[0x67, 0x8A, 0x03]);
                        }
                        _ => {
                            panic!("Le Scribe ne sait lire la RAM qu'avec %ba ou une adresse fixe.")
                        }
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
                // Traduction de : sema %registre, valeur (ADD reg32, imm32)
                Instruction::Sema {
                    destination,
                    valeur,
                } => {
                    code_machine.push(0x66); // INDISPENSABLE : On force le mode 32 bits !
                    code_machine.push(0x81); // Opcode ADD

                    // On détermine le registre physique (Le ModR/M byte)
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xC0, // EAX
                        "ib" => 0xC1, // ECX
                        "ba" => 0xC3, // EBX
                        _ => panic!("Fatal error : unknown registre '{destination}' for sema"),
                    };
                    code_machine.push(modrm);

                    match valeur {
                        Expression::Number(n) => {
                            let octets_valeur = n.to_le_bytes();
                            code_machine.extend_from_slice(&octets_valeur);
                        }
                        _ => panic!("The Emitter only handles numbers for 'sema' currently"),
                    }
                }
                // Traduction de : wdj %registre, valeur (CMP AL, imm8)
                Instruction::Wdj { left, right } => {
                    if left != "ka" {
                        panic!("In Bare-Metal OS mode, we only compare %ka for the moment.");
                    }
                    match right {
                        Expression::Number(n) => {
                            // OpCode 0x3C = Compare AL (notre registre %ka) avec une valeur d'1 octet
                            code_machine.push(0x3C);
                            code_machine.push(*n as u8); // Le code ASCII à comparer
                        }
                        _ => panic!("The Transmitter only compares numbers (ASCII codes)."),
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
                Instruction::Kheb {
                    destination,
                    valeur,
                } => {
                    code_machine.push(0x66); // INDISPENSABLE : On force le mode 32 bits !
                    code_machine.push(0x81); // Opcode SUB

                    // On détermine le registre physique pour la soustraction
                    let modrm: u8 = match destination.as_str() {
                        "ka" => 0xE8, // EAX
                        "ib" => 0xE9, // ECX
                        "ba" => 0xEB, // EBX
                        _ => panic!("Fatal error : unknown registre '{}' for kheb", destination),
                    };
                    code_machine.push(modrm);

                    match valeur {
                        Expression::Number(n) => {
                            let octets_valeur = n.to_le_bytes();
                            code_machine.extend_from_slice(&octets_valeur);
                        }
                        _ => panic!("The Emitter only handles numbers for 'kheb' currently"),
                    }
                }
            }
        }
        // --- LA PASSE DE PATCH (Résolution des Sauts) ---
        for (offset_du_trou, cible) in sauts_a_patcher {
            // On cherche où se trouve vraiment l'étiquette
            let adresse_cible = labels.get(&cible).expect(&format!(
                "Erreur fatale : Étiquette '{}' introuvable",
                cible
            ));

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
    fn test_generer_binaire_henek() {
        // On simule l'Arbre (AST) donné par le Parser pour "henek %ka, 10"
        let ast = vec![Instruction::Henek {
            destination: "ka".to_string(),
            valeur: Expression::Number(10), // 10 en décimal = 0x0A en hexadécimal
        }];

        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire();

        // Le Moment de Vérité :
        // 0xB8 = MOV EAX
        // 0x0A, 0x00, 0x00, 0x00 = Le nombre 10 en 32 bits (Little-Endian)
        assert_eq!(binaire, vec![0xB8, 0x0A, 0x00, 0x00, 0x00]);
    }
    #[test]
    fn test_generer_binaire_sema() {
        // sema %ka, 5 -> On s'attend à ADD EAX, 5
        let ast = vec![Instruction::Sema {
            destination: "ka".to_string(),
            valeur: Expression::Number(5),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire();

        // 0x81 0xC0 = ADD EAX
        // 0x05 0x00 0x00 0x00 = Le chiffre 5
        assert_eq!(binaire, vec![0x81, 0xC0, 0x05, 0x00, 0x00, 0x00]);
    }

    #[test]
    #[should_panic]
    fn test_generer_binaire_wdj() {
        // wdj %ib, 0 -> On s'attend à CMP ECX, 0
        let ast = vec![Instruction::Wdj {
            left: "ib".to_string(),
            right: Expression::Number(0),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire();

        // 0x81 0xF9 = CMP ECX
        // 0x00 0x00 0x00 0x00 = Le chiffre 0 (Le Zéro)
        assert_eq!(binaire, vec![0x81, 0xF9, 0x00, 0x00, 0x00, 0x00]);
    }
    #[test]
    fn test_generer_binaire_return() {
        let ast = vec![Instruction::Return {
            resultat: Expression::Identifier("Success".to_string()),
        }];
        let emetteur = Emitter::new(ast, "qwerty".to_string());
        let binaire = emetteur.generer_binaire();

        // Un seul octet : RET
        assert_eq!(binaire, vec![0xFA, 0xF4]);
    }
}
