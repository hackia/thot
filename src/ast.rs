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
    // henek %registre, valeur
    Henek {
        destination: String,
        valeur: Expression,
    },
    // kheper %registre, adresse (Sauvegarder dans la RAM)
    Kheper {
        source: String,
        adresse: u16,
    },
    // ankh cible (Saute vers l'étiquette SI la comparaison précédente est égale)
    Ankh {
        cible: String,
    },
    // sena %registre, adresse (Charger depuis la RAM)
    Sena {
        destination: String,
        adresse: u16,
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
