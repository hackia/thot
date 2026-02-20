// Une Valeur peut être plusieurs choses dans Maât
#[derive(Debug, PartialEq)]
pub enum Expression {
    Number(i32),
    Register(String),
    Identifier(String),
    StringLiteral(String),
}

// Les Instructions pures (La Loi)
#[derive(Debug, PartialEq)]
pub enum Instruction {
    // nama mon_identifiant = valeur
    Nama {
        nom: String,
        valeur: Expression,
    },
    Rdtsc, // Lit le compteur de cycles CPU
    // push %registre ou push nombre
    Push {
        cible: Expression,
    },

    // pop %registre
    Pop {
        destination: String,
    },

    // in port (Lit un octet depuis un port matériel vers %ka)
    In {
        port: Expression,
    },
    // smen NOM = VALEUR (Constante de compilation)
    Smen {
        nom: String,
        valeur: i32,
    },
    // out port (Écrit l'octet de %ka vers un port matériel)
    Out {
        port: Expression,
    },
    // henek %registre, valeur
    Henek {
        destination: String,
        valeur: Expression,
    },
    Dema {
        chemin: String,
    },
    // henet %registre, valeur (AND logique)
    Henet {
        destination: String,
        valeur: Expression,
    },
    // mer %registre, valeur (OR logique)
    Mer {
        destination: String,
        valeur: Expression,
    },
    Her {
        cible: String,
    }, // JG
    Kher {
        cible: String,
    }, // JL
    HerAnkh {
        cible: String,
    }, // JGE
    KherAnkh {
        cible: String,
    }, // JLE
    // duat "Ma phrase", adresse
    Duat {
        phrase: String,
        adresse: u16,
    },
    // jena cible (Appel de fonction / CALL)
    Jena {
        cible: String,
    },
    // kheper %registre, adresse (Sauvegarder dans la RAM)
    Kheper {
        source: String,
        adresse: Expression,
    },
    // ankh cible (Saute vers l'étiquette SI la comparaison précédente est égale)
    Ankh {
        cible: String,
    },
    // isfet cible (Saute SI la comparaison précédente est DIFFERENTE)
    Isfet {
        cible: String,
    },
    Kheb {
        destination: String,
        valeur: Expression,
    },

    // sena %registre, adresse (Charger depuis la RAM)
    Sena {
        destination: String,
        adresse: Expression,
    },
    // sema %registre, valeur
    Sema {
        destination: String,
        valeur: Expression,
    },
    // Une étiquette dans le code (ex: "boucle:")
    Label(String),

    // neheh cible (Saute vers l'étiquette)
    Neheh {
        cible: String,
    },
    // wdj %registre, valeur
    Wdj {
        left: String,
        right: Expression,
    },
    Wab,
    // per "message"
    Per {
        message: Expression,
    },
    // sedjem %registre
    Sedjem {
        destination: String,
    },
    // return value
    Return {
        resultat: Expression,
    },
}
