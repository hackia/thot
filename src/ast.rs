use std::fmt;

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let label = match self {
            Level::Base => "Base",
            Level::Medium => "Medium",
            Level::High => "High",
            Level::Very => "Very",
            Level::Extreme => "Extreme",
            Level::Xenith => "Xenith",
        };
        write!(f, "{} ({}-bit)", label, self.bits())
    }
}

impl fmt::Display for Registry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.name();
        // On affiche le nom, l'adresse finale en Hex et le niveau
        write!(
            f,
            "[{}] 0x{:02X} (Level: {})",
            name,
            self.to_u8(),
            self.level()
        )
    }
}
#[derive(Debug)]
pub enum RegistryError {
    AddressOverflow,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum Level {
    Base = 8,    // 8bits
    Medium = 16, // 16bits
    High = 32,   // 32bits
    Very = 64,   // 64bits
    Extreme = 128, // 128bits
    Xenith = 256,  // 256bits
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Registry {
    Ka(Level),
    Ba(Level),
    Da(Level),
    Ib(Level),
    Si(Level),
    Di(Level),
}
impl Level {
    pub const fn bits(self) -> u16 {
        self as u16
    }

    pub const fn bytes(self) -> u16 {
        self.bits() / 8
    }

    pub const fn index(self) -> u8 {
        match self {
            Level::Base => 0,
            Level::Medium => 1,
            Level::High => 2,
            Level::Very => 3,
            Level::Extreme => 4,
            Level::Xenith => 5,
        }
    }

    pub fn reset(&mut self) {
        *self = Level::Base;
    }

    /// Vérifie si on est au sommet
    pub fn is_max(&self) -> bool {
        matches!(self, Level::Xenith)
    }

    /// Vérifie si on est déjà tout en bas
    pub fn is_min(&self) -> bool {
        matches!(self, Level::Base)
    }

    /// Passe au niveau supérieur (s'arrête à Xenith)
    pub fn up(&mut self) {
        *self = match self {
            Level::Base => Level::Medium,
            Level::Medium => Level::High,
            Level::High => Level::Very,
            Level::Very => Level::Extreme,
            Level::Extreme => Level::Xenith,
            Level::Xenith => Level::Xenith, // Reste au max
        };
    }

    /// Passe au niveau inférieur (s'arrête à Base)
    pub fn down(&mut self) {
        *self = match self {
            Level::Xenith => Level::Extreme,
            Level::Extreme => Level::Very,
            Level::Very => Level::High,
            Level::High => Level::Medium,
            Level::Medium => Level::Base,
            Level::Base => Level::Base, // Reste au min
        };
    }
}
impl Registry {
    pub fn try_new(variant: fn(Level) -> Registry, level: Level) -> Result<Self, RegistryError> {
        let temp_reg = variant(level);
        Ok(temp_reg)
    }
    pub fn safe_up(&mut self) {
        // On ne monte le niveau que si on n'est pas déjà au max.
        if !self.is_max() {
            self.up();
        } else {
            panic!("Impossible to climb: material limit reached!");
        }
    }
    pub fn is_min(&self) -> bool {
        match self {
            Registry::Ka(l)
            | Registry::Ib(l)
            | Registry::Da(l)
            | Registry::Ba(l)
            | Registry::Si(l)
            | Registry::Di(l) => l.is_min(),
        }
    }
    /// Descend le niveau de sécurité, renvoie une erreur si on est déjà au minimum.
    pub fn safe_down(&mut self) -> Result<(), &'static str> {
        if self.is_min() {
            // On refuse de descendre si on est déjà à Base
            Err("Action impossible: The registry is already at Base level.")
        } else {
            // On utilise la méthode down() qu'on a codée précédemment
            self.down();
            Ok(())
        }
    }

    /// Quickly retrieve the level regardless of the registry type.
    pub fn level(&self) -> Level {
        match self {
            Registry::Ka(l)
            | Registry::Ba(l)
            | Registry::Da(l)
            | Registry::Ib(l)
            | Registry::Si(l)
            | Registry::Di(l) => *l,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Registry::Ka(_) => "ka",
            Registry::Ib(_) => "ib",
            Registry::Da(_) => "da",
            Registry::Ba(_) => "ba",
            Registry::Si(_) => "si",
            Registry::Di(_) => "di",
        }
    }

    pub fn base(&self) -> u8 {
        self.reg_id()
    }

    pub fn reg_id(&self) -> u8 {
        match self {
            Registry::Ka(_) => 0,
            Registry::Ib(_) => 1,
            Registry::Da(_) => 2,
            Registry::Ba(_) => 3,
            Registry::Si(_) => 4,
            Registry::Di(_) => 5,
        }
    }

    pub fn bits(&self) -> u16 {
        self.level().bits()
    }

    pub fn bytes(&self) -> u16 {
        self.level().bytes()
    }
    pub fn reset(&mut self) {
        self.get_mut_level().reset();
    }

    pub fn is_max(&self) -> bool {
        self.get_level().is_max()
    }

    // Petite astuce de refactoring : on crée des helpers internes
    // pour éviter de répéter le gros "match" partout.
    fn get_level(&self) -> &Level {
        match self {
            Registry::Ka(l)
            | Registry::Ib(l)
            | Registry::Da(l)
            | Registry::Ba(l)
            | Registry::Si(l)
            | Registry::Di(l) => l,
        }
    }

    fn get_mut_level(&mut self) -> &mut Level {
        match self {
            Registry::Ka(l)
            | Registry::Ib(l)
            | Registry::Da(l)
            | Registry::Ba(l)
            | Registry::Si(l)
            | Registry::Di(l) => l,
        }
    }
    pub fn up(&mut self) {
        match self {
            // Le pattern "|" permet d'appliquer la logique à toutes les variantes
            Registry::Ka(l)
            | Registry::Ib(l)
            | Registry::Da(l)
            | Registry::Ba(l)
            | Registry::Si(l)
            | Registry::Di(l) => l.up(),
        }
    }

    pub fn down(&mut self) {
        match self {
            Registry::Ka(l)
            | Registry::Ib(l)
            | Registry::Da(l)
            | Registry::Ba(l)
            | Registry::Si(l)
            | Registry::Di(l) => l.down(),
        }
    }
    pub fn to_u8(&self) -> u8 {
        let reg_id = self.reg_id();
        let level_index = self.level().index();

        // Encodage stable : [level:3 bits][reg:3 bits]
        (level_index << 3) | reg_id
    }
}
// Une Valeur peut être plusieurs choses dans Maât
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum Expression {
    Number(i32),
    Helix { ra: u16, apophis: u16 },
    Register(String),
    Identifier(String),
    StringLiteral(String),
    CurrentAddress,
}

// Les Instructions pures (La Loi)
#[derive(Debug, Eq, Hash, Clone, PartialEq)]
pub enum Instruction {
    CurrentAddress, // Le symbole $
    // nama mon_identifiant = valeur
    Nama {
        name: String,
        value: Expression,
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
        value: Expression,
    },
    Dema {
        chemin: String,
    },
    // henet %registre, valeur (AND logique)
    Henet {
        destination: String,
        value: Expression,
    },
    // mer %registre, valeur (OR logique)
    Mer {
        destination: String,
        value: Expression,
    },
    // Change String en Expression pour tous les sauts
    Neheh {
        cible: Expression,
    },
    Ankh {
        cible: Expression,
    },
    Isfet {
        cible: Expression,
    },
    Jena {
        cible: Expression,
    },
    Her {
        cible: Expression,
    },
    Kher {
        cible: Expression,
    },
    HerAnkh {
        cible: Expression,
    },
    KherAnkh {
        cible: Expression,
    },
    // duat "Ma phrase", adresse
    Duat {
        phrase: String,
        address: u16,
    },

    // kheper %registre, adresse (Sauvegarder dans la RAM)
    Kheper {
        source: String,
        adresse: Expression,
    },
    Kheb {
        destination: String,
        value: Expression,
    },

    // sena %registre, adresse (Charger depuis la RAM)
    Sena {
        destination: String,
        adresse: Expression,
    },
    // sema %registre, valeur
    Sema {
        destination: String,
        value: Expression,
    },
    // shesa %registre, valeur
    Shesa {
        destination: String,
        value: Expression,
    },
    // Une étiquette dans le code (ex: "boucle:")
    Label(String),
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
    Kherp,
}
