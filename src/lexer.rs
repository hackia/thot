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
            // Si c'est un signe moins -> C'est un Nombre négatif
            '-' => {
                let mut number_str = String::from("-");
                while let Some(&next_char) = self.input.peek() {
                    if next_char.is_digit(10) {
                        number_str.push(self.input.next().unwrap());
                    } else {
                        break;
                    }
                }
                if number_str == "-" {
                    panic!("Erreur fatale : Un signe '-' doit être suivi d'un chiffre.");
                }
                Token::Number(number_str.parse::<i32>().unwrap())
            }
            // If it's a digit -> It's a Number
            '0'..='9' => {
                let mut number_str = c.to_string();
                while let Some(&next_char) = self.input.peek() {
                    if next_char.is_digit(10) {
                        number_str.push(self.input.next().unwrap());
                    } else {
                        break;
                    }
                }
                Token::Number(number_str.parse::<i32>().unwrap())
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
                    | "dema" => Token::Verb(word),
                    _ => Token::Identifier(word), // Otherwise, it's a variable/type
                }
            }

            _ => panic!("Thot encountered an unknown character: {c}"),
        }
    }
}
// Fichier : src/lexer.rs
// (À placer tout en bas du fichier, après l'implémentation du Lexer)

#[cfg(test)]
mod tests {
    use super::*;

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
