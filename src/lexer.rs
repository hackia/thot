#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Sacred Verbs (Instructions)
    Verb(String), // e.g., "henek", "wdj", "sema"

    // The Vessels (Registers)
    Register(String), // e.g., "ka", "mba", "hdi"

    // Standard words (Variables or Types)
    Identifier(String), // e.g., "effort", "Helix"

    // Pure values
    Number(i32),
    Helix(u16, u16),
    StringLiteral(String), // e.g., "Alert !"

    // Punctuation
    Comma,      // ,
    Colon,      // :
    Dot,        // .
    OpenParen,  // (
    CloseParen, // )
    Equals,
    Plus,   // +
    Minus,  // -
    Star,   // *
    Slash,  // /
    Dollar, // $ (L'adresse actuelle)
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
            '$' => Token::Dollar,
            '-' => Token::Minus,
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
                        let c = self.input.next().unwrap();
                        // --- LE DÉTECTEUR D'ÉCHAPPEMENT ---
                        if c == '\\' {
                            if let Some(escaped) = self.input.next() {
                                match escaped {
                                    'n' => string_content.push('\n'), // 0x0A
                                    'r' => string_content.push('\r'), // 0x0D
                                    't' => string_content.push('\t'), // 0x09
                                    '\\' => string_content.push('\\'),
                                    '"' => string_content.push('"'),
                                    _ => string_content.push(escaped),
                                }
                            }
                        } else {
                            string_content.push(c);
                        }
                    } else {
                        self.input.next(); // Ferme les guillemets
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
            // Si c'est un chiffre -> C'est le début d'un HELIX (Ra:Apophis)
            '0'..='9' => {
                let mut is_hex = false;
                if c == '0' {
                    if let Some(&next_char) = self.input.peek() {
                        if next_char == 'x' || next_char == 'X' {
                            is_hex = true;
                            self.input.next(); // On mange le 'x'
                        }
                    }
                }

                // 1. On lit la première force (Ra)
                let mut ra_str = String::new();
                if !is_hex {
                    ra_str.push(c);
                }
                while let Some(&next_char) = self.input.peek() {
                    if (is_hex && next_char.is_ascii_hexdigit())
                        || (!is_hex && next_char.is_digit(10))
                    {
                        ra_str.push(self.input.next().unwrap());
                    } else {
                        break;
                    }
                }

                let ra_val = if is_hex {
                    u16::from_str_radix(&ra_str, 16).unwrap()
                } else {
                    ra_str.parse::<u16>().unwrap()
                };

                // 2. On cherche le point d'équilibre ':' (L'opposition)
                if let Some(&':') = self.input.peek() {
                    self.input.next(); // On mange le ':'

                    // On lit la seconde force (Apophis)
                    let mut apo_str = String::new();
                    while let Some(&next_char) = self.input.peek() {
                        if next_char.is_digit(10) {
                            apo_str.push(self.input.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let apophis_val = if !apo_str.is_empty() {
                        apo_str.parse::<u16>().unwrap()
                    } else {
                        0
                    };

                    Token::Helix(ra_val, apophis_val)
                } else {
                    Token::Number(ra_val as i32)
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
                    | "dema" | "push" | "pop" | "in" | "out" | "nama" | "smen" | "rdtsc"
                    | "kherp" => Token::Verb(word),
                    _ => Token::Identifier(word), // Otherwise, it's a variable/type
                }
            }
            _ => panic!("Thot encountered an unknown character: {c}"),
        }
    }
}
