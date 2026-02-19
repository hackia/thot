// Fichier : src/elf.rs

pub struct Sarcophage;

impl Sarcophage {
    // Emballe le code binaire brut dans un fichier exécutable ELF 64-bit
    pub fn emballer(code_machine: &[u8]) -> Vec<u8> {
        let mut binaire_final = Vec::new();

        // L'en-tête ELF (120 octets). 
        // Il dit à Linux : "Ceci est un exécutable x86_64, charge-le en mémoire à l'adresse 0x400000"
        let elf_header: [u8; 120] = [
            // --- 1. ELF Header (64 octets) ---
            0x7F, 0x45, 0x4C, 0x46, // Magie : \x7F E L F (Le mot de passe de Linux)
            0x02, // Classe : 64-bit
            0x01, // Données : Little Endian
            0x01, // Version ELF : 1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Remplissage
            0x02, 0x00, // Type : Fichier Exécutable
            0x3E, 0x00, // Machine : Advanced Micro Devices x86-64
            0x01, 0x00, 0x00, 0x00, // Version
            0x78, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, // Point d'entrée : 0x400078 (Notre code commence ici)
            0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Début du Program Header (à l'octet 64)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Début du Section Header (Aucun pour l'instant)
            0x00, 0x00, 0x00, 0x00, // Flags du processeur
            0x40, 0x00, // Taille de cet en-tête (64)
            0x38, 0x00, // Taille d'un Program Header (56)
            0x01, 0x00, // Nombre de Program Headers (1)
            0x00, 0x00, // Taille d'un Section Header
            0x00, 0x00, // Nombre de Section Headers
            0x00, 0x00, // Index des noms de sections

            // --- 2. Program Header (56 octets) ---
            0x01, 0x00, 0x00, 0x00, // Type : LOAD (On demande à Linux de charger ça en RAM)
            0x05, 0x00, 0x00, 0x00, // Permissions : Lecture + Exécution (Read + Execute)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset dans le fichier (0)
            0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, // Adresse virtuelle en RAM (0x400000)
            0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, // Adresse physique (ignorée sur PC)
            0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Taille dans le fichier (120 octets = 0x78)
            0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Taille occupée en RAM
            0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Alignement mémoire (4096 octets)
        ];

        binaire_final.extend_from_slice(&elf_header);

        // On calcule la taille totale (120 octets d'en-tête + la taille de notre code)
        let taille_totale = (120 + code_machine.len()) as u64;
        let bytes_taille = taille_totale.to_le_bytes();

        // On patch le Program Header pour lui dire exactement combien d'octets charger en RAM
        binaire_final[96..104].copy_from_slice(&bytes_taille);  // Taille du fichier
        binaire_final[104..112].copy_from_slice(&bytes_taille); // Taille en mémoire

        // On insère notre véritable code machine juste après l'en-tête (à l'octet 120)
        binaire_final.extend_from_slice(code_machine);

        binaire_final
    }
}