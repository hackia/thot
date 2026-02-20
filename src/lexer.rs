#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Sacred Verbs (Instructions)
    Verb(String), // e.g., "henek", "wdj", "sema"

    // The Vessels (Registers)
    Register(String), // e.g., "ka", "ba", "ib"

    // Standard words (Variables or Types)
    Identifier(String), // e.g., "effort", "Helix"

    // Pure values
    Number(i32),           // e.g., 10, 0
    StringLiteral(String), // e.g., "Alert !"

    // Punctuation
    Comma,      // ,
    Colon,      // :
    Dot,        // .
    OpenParen,  // (
    CloseParen, // )
    Equals,
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /
    // End of File
    Eof,
    OpenBracket,
    CloseBracket,
}
use std::iter::Peekable;
use std::str::Chars;

#[derive(Clone)]
pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    // Initializes Thot with the source code
    pub fn new(source: &'a str) -> Self {
        Lexer {
            input: source.chars().peekable(),
        }
    }

    // Extracts the next Token
    pub fn next_token(&mut self) -> Token {
        // 1. Skip whitespace and newlines
        while let Some(&c) = self.input.peek() {
            if c.is_whitespace() {
                self.input.next();
            } else {
                break;
            }
        }

        // 2. Look at the current character
        let c = match self.input.next() {
            Some(c) => c,
            None => return Token::Eof,
        };

        // 3. Classify the character
        match c {
            '=' => Token::Equals,
            '+' => Token::Plus,
            '-' => {
                // On vérifie s'il s'agit d'un nombre négatif ou de l'opérateur Moins
                if let Some(&next_char) = self.input.peek() {
                    if next_char.is_digit(10) {
                        // ... garde ici ton ancienne logique pour les nombres négatifs
                    }
                }
                Token::Minus
            }
            '*' => Token::Star,
            '/' => Token::Slash,
            ';' => {
                // C'est un Murmure ! On ignore tout jusqu'à la fin de la ligne.
                while let Some(&next_char) = self.input.peek() {
                    if next_char != '\n' {
                        self.input.next(); // On "mange" le caractère sans rien en faire
                    } else {
                        break; // On s'arrête au saut de ligne
                    }
                }
                // La ligne est finie, on relance la machine pour chercher le VRAI prochain jeton
                self.next_token()
            }
            '[' => Token::OpenBracket,
            ']' => Token::CloseBracket,
            ',' => Token::Comma,
            ':' => Token::Colon,
            '.' => Token::Dot,
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '"' => {
                let mut string_content = String::new();
                while let Some(&next_char) = self.input.peek() {
                    if next_char != '"' {
                        // On ajoute les lettres tant qu'on ne trouve pas le guillemet de fin
                        string_content.push(self.input.next().unwrap());
                    } else {
                        // On "mange" le guillemet de fin et on arrête la boucle
                        self.input.next();
                        break;
                    }
                }
                Token::StringLiteral(string_content)
            }
            // If it's a '%' -> It's a Sacred Register!
            '%' => {
                let mut name = String::new();
                while let Some(&next_char) = self.input.peek() {
                    if next_char.is_alphabetic() {
                        name.push(self.input.next().unwrap());
                    } else {
                        break;
                    }
                }
                Token::Register(name)
            }
            // If it's a digit -> It's a Number (Decimal or Hexadecimal)
            '0'..='9' => {
                let mut is_hex = false;
                // On vérifie le sceau de l'hexadécimal : "0x" ou "0X"
                if c == '0' {
                    if let Some(&next_char) = self.input.peek() {
                        if next_char == 'x' || next_char == 'X' {
                            is_hex = true;
                            self.input.next(); // On "mange" le 'x'
                        }
                    }
                }

                let mut number_str = String::new();
                if !is_hex {
                    number_str.push(c); // On garde le premier chiffre (ex: '5') si c'est du décimal
                }

                // On extrait la suite des caractères
                while let Some(&next_char) = self.input.peek() {
                    if is_hex && next_char.is_ascii_hexdigit() {
                        // is_ascii_hexdigit() accepte 0-9, a-f, et A-F !
                        number_str.push(self.input.next().unwrap());
                    } else if !is_hex && next_char.is_digit(10) {
                        number_str.push(self.input.next().unwrap());
                    } else {
                        break;
                    }
                }

                if is_hex {
                    if number_str.is_empty() {
                        panic!("Erreur fatale : '0x' doit être suivi de chiffres hexadécimaux.");
                    }
                    // On convertit la chaîne hexadécimale en un i32 pur
                    Token::Number(i32::from_str_radix(&number_str, 16).unwrap())
                } else {
                    // Conversion classique en base 10
                    Token::Number(number_str.parse::<i32>().unwrap())
                }
            }

            // If it's a letter -> It's a Verb or an Identifier
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut word = c.to_string();
                while let Some(&next_char) = self.input.peek() {
                    if next_char.is_alphanumeric() || next_char == '_' {
                        word.push(self.input.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Check if it's a known Maât verb
                match word.as_str() {
                    "sokh" | "henek" | "sema" | "wdj" | "duat" | "ankh" | "sena" | "neheh"
                    | "kheper" | "per" | "return" | "sedjem" | "wab" | "jena" | "isfet"
                    | "kheb" | "henet" | "mer" | "her" | "kher" | "her_ankh" | "kher_ankh"
                    | "dema" | "push" | "pop" | "in" | "out" | "nama" | "smen" => Token::Verb(word),
                    _ => Token::Identifier(word), // Otherwise, it's a variable/type
                }
            }
            _ => panic!("Thot encountered an unknown character: {c}"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_math_et_smen() {
        let mut lexer = Lexer::new("smen X = 10 + 5 * 2 / 1");

        assert_eq!(lexer.next_token(), Token::Verb("smen".to_string()));
        assert_eq!(lexer.next_token(), Token::Identifier("X".to_string()));
        assert_eq!(lexer.next_token(), Token::Equals);
        assert_eq!(lexer.next_token(), Token::Number(10));
        assert_eq!(lexer.next_token(), Token::Plus);
        assert_eq!(lexer.next_token(), Token::Number(5));
        assert_eq!(lexer.next_token(), Token::Star);
        assert_eq!(lexer.next_token(), Token::Number(2));
        assert_eq!(lexer.next_token(), Token::Slash);
        assert_eq!(lexer.next_token(), Token::Number(1));
    }

    #[test]
    fn test_nouveaux_verbes_materiels() {
        // Test : Est-ce que Thot lit correctement les accès matériels et la pile ?
        let mut lexer = Lexer::new("push %ka \n pop %ib \n in 96 \n out %da");

        // push %ka
        assert_eq!(lexer.next_token(), Token::Verb("push".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("ka".to_string()));

        // pop %ib
        assert_eq!(lexer.next_token(), Token::Verb("pop".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("ib".to_string()));

        // in 96 (96 est l'équivalent décimal du port 0x60 pour le clavier)
        assert_eq!(lexer.next_token(), Token::Verb("in".to_string()));
        assert_eq!(lexer.next_token(), Token::Number(96));

        // out %da
        assert_eq!(lexer.next_token(), Token::Verb("out".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("da".to_string()));

        assert_eq!(lexer.next_token(), Token::Eof);
    }
    #[test]
    fn test_basic_instruction() {
        // Test : Est-ce que Thot lit correctement une offrande simple ?
        let mut lexer = Lexer::new("henek %ka, 10");

        assert_eq!(lexer.next_token(), Token::Verb("henek".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("ka".to_string()));
        assert_eq!(lexer.next_token(), Token::Comma);
        assert_eq!(lexer.next_token(), Token::Number(10));
        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_whitespace_and_newlines() {
        // Test : Thot doit ignorer les espaces, les tabulations et les sauts de ligne
        let mut lexer = Lexer::new("  wdj   \n  %ib  ,  0  ");

        assert_eq!(lexer.next_token(), Token::Verb("wdj".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("ib".to_string()));
        assert_eq!(lexer.next_token(), Token::Comma);
        assert_eq!(lexer.next_token(), Token::Number(0));
        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_function_signature() {
        // Test : Est-ce que Thot différencie bien les verbes (sokh) des identifiants (Helix) ?
        let mut lexer = Lexer::new("sokh tenter_mouvement(effort: Helix): Wdj");

        assert_eq!(lexer.next_token(), Token::Verb("sokh".to_string()));
        assert_eq!(
            lexer.next_token(),
            Token::Identifier("tenter_mouvement".to_string())
        );
        assert_eq!(lexer.next_token(), Token::OpenParen);
        assert_eq!(lexer.next_token(), Token::Identifier("effort".to_string()));
        assert_eq!(lexer.next_token(), Token::Colon);
        assert_eq!(lexer.next_token(), Token::Identifier("Helix".to_string()));
        assert_eq!(lexer.next_token(), Token::CloseParen);
        assert_eq!(lexer.next_token(), Token::Colon);
        assert_eq!(lexer.next_token(), Token::Identifier("Wdj".to_string()));
        assert_eq!(lexer.next_token(), Token::Eof);
    }
    #[test]
    fn test_nombres_hexadecimaux() {
        // Test : Thot sait-il lire l'hexadécimal et le décimal dans la même phrase ?
        let mut lexer = Lexer::new("henek %ka, 0x60 \n sema %ib, 42 \n in 0xFF");

        // henek %ka, 0x60 (96 en décimal)
        assert_eq!(lexer.next_token(), Token::Verb("henek".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("ka".to_string()));
        assert_eq!(lexer.next_token(), Token::Comma);
        assert_eq!(lexer.next_token(), Token::Number(96)); // 0x60 = 96

        // sema %ib, 42 (Le décimal classique doit toujours marcher)
        assert_eq!(lexer.next_token(), Token::Verb("sema".to_string()));
        assert_eq!(lexer.next_token(), Token::Register("ib".to_string()));
        assert_eq!(lexer.next_token(), Token::Comma);
        assert_eq!(lexer.next_token(), Token::Number(42));

        // in 0xFF (255 en décimal)
        assert_eq!(lexer.next_token(), Token::Verb("in".to_string()));
        assert_eq!(lexer.next_token(), Token::Number(255)); // 0xFF = 255

        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_string_literal() {
        // Test : Est-ce que Thot arrive à lire une phrase entre guillemets ?
        let mut lexer = Lexer::new("per \"Alerte !\"");

        assert_eq!(lexer.next_token(), Token::Verb("per".to_string()));
        assert_eq!(
            lexer.next_token(),
            Token::StringLiteral("Alerte !".to_string())
        );
        assert_eq!(lexer.next_token(), Token::Eof);
    }
}
