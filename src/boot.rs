pub struct Naos;

impl Naos {
    // Emballe le code binaire brut dans un Secteur d'Amorçage (Bootloader) de 512 octets
    pub fn emballer(code_machine: &[u8]) -> Vec<u8> {
        let mut binaire_final = Vec::new();

        // 1. On insère notre véritable code machine au tout début
        binaire_final.extend_from_slice(code_machine);

        // La limite absolue d'un secteur de boot est 510 octets (pour laisser la place à la signature)
        if binaire_final.len() > 510 {
            panic!("Erreur fatale : Le code du bootloader dépasse la limite matérielle de 510 octets !");
        }

        // 2. On remplit le reste de l'espace avec des zéros (Padding)
        while binaire_final.len() < 510 {
            binaire_final.push(0x00);
        }

        // 3. La Signature Magique du BIOS (0x55, 0xAA)
        // C'est le mot de passe matériel pour que la carte mère accepte de booter
        binaire_final.push(0x55);
        binaire_final.push(0xAA);

        binaire_final
    }
}