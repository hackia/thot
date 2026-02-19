use crate::ast::{Expression, Instruction};

pub struct Emitter {
    instructions: Vec<Instruction>,
    kbd_layout: String,
}

impl Emitter {
    // On charge l'Émetteur avec l'Arbre Syntaxique (AST)
    pub fn new(instructions: Vec<Instruction>, kbd_layout: String) -> Self {
        Emitter {
            instructions,
            kbd_layout,
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
                Instruction::Kheper { source, adresse } => {
                    if source != "ka" {
                        panic!("The Transmitter only handles %ka to write to RAM at the moment.");
                    }
                    // MOV [adresse], AL -> OpCode 0xA2 suivi de l'adresse en Little-Endian (2 octets)
                    code_machine.push(0xA2);
                    code_machine.extend_from_slice(&adresse.to_le_bytes());
                }

                Instruction::Sena {
                    destination,
                    adresse,
                } => {
                    if destination != "ka" {
                        panic!("The Transmitter only handles %ka to read RAM at the moment.");
                    }
                    // MOV AL, [adresse] -> OpCode 0xA0 suivi de l'adresse en Little-Endian
                    code_machine.push(0xA0);
                    code_machine.extend_from_slice(&adresse.to_le_bytes());
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
                Instruction::Wab => {
                    // Le Sortilège du Vide : MOV AX, 0x0003 puis INT 0x10
                    code_machine.extend_from_slice(&[0xB8, 0x03, 0x00, 0xCD, 0x10]);
                }
                Instruction::Per { message } => {
                    match message {
                        Expression::StringLiteral(s) => {
                            // 1. Le Grand Nettoyage (Clear Screen)
                            // On demande au BIOS de recharger le mode texte standard (80x25).
                            // Cela efface tout l'écran et place le curseur matériel à la position (0,0).
                            code_machine.extend_from_slice(&[0xB8, 0x03, 0x00]); // MOV AX, 0x0003
                            code_machine.extend_from_slice(&[0xCD, 0x10]); // INT 0x10

                            // 2. Écriture propre avec le BIOS (Teletype)
                            // Le curseur va avancer automatiquement après chaque lettre.
                            for c in s.chars() {
                                code_machine.extend_from_slice(&[0xB4, 0x0E]); // MOV AH, 0x0E (Teletype)
                                code_machine.push(0xB0); // MOV AL, caractère
                                code_machine.push(c as u8);
                                code_machine.extend_from_slice(&[0xCD, 0x10]); // INT 0x10
                            }
                        }
                        Expression::Register(r) => {
                            if r == "ka" {
                                // Le code "Blindé" pour afficher %ka (AL) correctement
                                let affichage_propre: [u8; 15] = [
                                    0xB4, 0x0E,       // MOV AH, 0x0E (Mode Teletype du BIOS)
                                    0xBB, 0x0F, 0x00, // MOV BX, 0x000F (Force la Page vidéo 0, Couleur Blanc)
                                    0xCD, 0x10,       // INT 0x10 (Affiche la lettre stockée dans AL)

                                    // --- Gestion spéciale de la touche Entrée ---
                                    0x3C, 0x0D,       // CMP AL, 0x0D (Est-ce que la lettre était "Entrée" ?)
                                    0x75, 0x04,       // JNE +4 (Si NON, on a fini, on saute les 4 prochains octets)
                                    0xB0, 0x0A,       // MOV AL, 0x0A (Si OUI, on charge le code "Line Feed" / Saut de ligne)
                                    0xCD, 0x10,       // INT 0x10 (Et on l'imprime pour descendre d'une ligne)
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
                    // Opcode de base pour une addition avec un nombre : 0x81
                    code_machine.push(0x81);

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
                // Traduction de : return valeur (RET)
                Instruction::Return { .. } => {
                    // En Bootloader, on ne peut pas faire "RET" car il n'y a pas d'OS vers qui retourner.
                    // On doit arrêter le processeur pour qu'il ne lise pas le vide.
                    // 1. CLI (0xFA) -> Coupe les interruptions (On devient sourd)
                    // 2. HLT (0xF4) -> Halt (Le processeur s'endort pour l'éternité)
                    code_machine.extend_from_slice(&[0xFA, 0xF4]);
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
