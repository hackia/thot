// Fichier : src/parser.rs

use crate::ast::{Expression, Instruction};
use crate::lexer::{Lexer, Token};

#[derive(Clone)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    // Initialise le Parser et charge le premier jeton
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let first_token = lexer.next_token();
        Parser {
            lexer,
            current_token: first_token,
        }
    }

    pub fn current_token(&self) -> Token {
        self.current_token.clone()
    }
    pub fn eof(&self) -> bool {
        self.current_token() == Token::Eof
    }
    pub fn not_eof(&self) -> bool {
        !self.eof()
    }
    // Passe au jeton suivant
    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    // Vérifie qu'on a le bon jeton, sinon le compilateur hurle (Erreur de syntaxe)
    fn expect_token(&mut self, expected: Token) {
        if self.current_token == expected {
            self.advance();
        } else {
            panic!(
                "Syntax Error: Expected {:?}, but found {:?}",
                expected, self.current_token
            );
        }
    }

    // Extrait une expression (ex: 10, "Alerte", ou un identifiant)
    fn parse_expression(&mut self) -> Expression {
        let expr = match &self.current_token {
            Token::Number(n) => Expression::Number(*n),
            Token::StringLiteral(s) => Expression::StringLiteral(s.clone()),
            Token::Identifier(i) => Expression::Identifier(i.clone()),
            Token::Register(r) => Expression::Register(r.clone()),
            _ => panic!(
                "Syntax Error: Expected an expression, found {:?}",
                self.current_token
            ),
        };
        self.advance(); // On avance après avoir capturé l'expression
        expr
    }

    // Analyse une instruction complète
    pub fn parse_instruction(&mut self) -> Instruction {
        match &self.current_token {
            Token::Verb(v) if v == "henek" => {
                self.advance(); // Consomme 'henek'

                // On s'attend à un registre comme destination
                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'henek' requires a register as destination"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let valeur = self.parse_expression(); // Capture la valeur (ex: 10)

                Instruction::Henek {
                    destination,
                    valeur,
                }
            }
            // Traduction de : kheper %registre, adresse
            Token::Verb(v) if v == "kheper" => {
                self.advance(); // Consomme 'kheper'

                let source = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'kheper' requires a register as source"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let adresse = match self.parse_expression() {
                    Expression::Number(n) => n as u16, // On convertit en adresse 16-bit
                    _ => panic!("Syntax Error: 'kheper' requires a numeric memory address"),
                };
                Instruction::Kheper { source, adresse }
            }

            // Traduction de : sena %registre, adresse
            Token::Verb(v) if v == "sena" => {
                self.advance(); // Consomme 'sena'

                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'sena' requires a register as destination"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let adresse = match self.parse_expression() {
                    Expression::Number(n) => n as u16,
                    _ => panic!("Syntax Error: 'sena' requires a numeric memory address"),
                };
                Instruction::Sena { destination, adresse }
            }
            Token::Verb(v) if v == "wab" => {
                self.advance(); // Consomme le mot 'wab'
                Instruction::Wab
            }
            // Traduction du saut conditionnel : ankh cible
            Token::Verb(v) if v == "ankh" => {
                self.advance(); // Consomme 'ankh'
                let cible = match &self.current_token {
                    Token::Identifier(i) => i.clone(),
                    _ => panic!("Syntax Error: 'ankh' requires a target label"),
                };
                self.advance(); // Consomme la cible
                Instruction::Ankh { cible }
            }
            // Traduction d'une étiquette (ex: "debut:")
            Token::Identifier(nom) => {
                let nom_label = nom.clone();
                self.advance(); // Consomme l'identifiant
                self.expect_token(Token::Colon); // Exige les deux points ":"
                Instruction::Label(nom_label)
            }

            // Traduction du saut inconditionnel : neheh cible
            Token::Verb(v) if v == "neheh" => {
                self.advance(); // Consomme 'neheh'
                let cible = match &self.current_token {
                    Token::Identifier(i) => i.clone(),
                    _ => panic!("Syntax Error: 'neheh' nécessite le nom d'une étiquette (Identifier)"),
                };
                self.advance(); // Consomme la cible
                Instruction::Neheh { cible }
            }
            Token::Verb(v) if v == "sedjem" => {
                self.advance(); // Consomme 'sedjem'

                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'sedjem' requires a register as destination"),
                };
                self.advance(); // Consomme le registre

                Instruction::Sedjem { destination }
            }
            Token::Verb(v) if v == "per" => {
                self.advance(); // Consomme 'per'
                let message = self.parse_expression(); // Capture le message
                Instruction::Per { message }
            }
            // Traduction de : sema %registre, valeur
            Token::Verb(v) if v == "sema" => {
                self.advance(); // Consomme 'sema'

                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'sema' requires a register as destination"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let valeur = self.parse_expression(); // Capture la force à unir
                Instruction::Sema {
                    destination,
                    valeur,
                }
            }

            // Traduction de : wdj %registre, valeur
            Token::Verb(v) if v == "wdj" => {
                self.advance(); // Consomme 'wdj'

                let left = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'wdj' requires a register on the left"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let right = self.parse_expression(); // Capture la valeur à peser
                Instruction::Wdj { left, right }
            }

            // Traduction de : returne valeur
            Token::Verb(v) if v == "return" => {
                self.advance(); // Consomme 'returne'
                let resultat = self.parse_expression(); // Capture ce qu'on renvoie
                Instruction::Return { resultat } // (Assure-toi que ça correspond au nom exact dans ton ast.rs)
            }
            // (On ajoutera 'wdj', 'sema', etc. ici plus tard)
            _ => panic!("Syntax Error: Unknown instruction {:?}", self.current_token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_henek() {
        let lexer = Lexer::new("henek %ka, 10");
        let mut parser = Parser::new(lexer);

        let instruction = parser.parse_instruction();

        assert_eq!(
            instruction,
            Instruction::Henek {
                destination: "ka".to_string(),
                valeur: Expression::Number(10),
            }
        );
    }
    #[test]
    fn test_parse_per() {
        let lexer = Lexer::new("per \"System Ready\"");
        let mut parser = Parser::new(lexer);

        let instruction = parser.parse_instruction();

        assert_eq!(
            instruction,
            Instruction::Per {
                message: Expression::StringLiteral("System Ready".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_sema() {
        let lexer = Lexer::new("sema %ka, -1");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Sema {
                destination: "ka".to_string(),
                valeur: Expression::Number(-1),
            }
        );
    }

    #[test]
    fn test_parse_wdj() {
        let lexer = Lexer::new("wdj %ib, 0");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Wdj {
                left: "ib".to_string(),
                right: Expression::Number(0),
            }
        );
    }
    #[test]
    fn test_parse_return() {
        // Test simplifié : on s'assure que 'return' comprend bien une variable simple
        let lexer = Lexer::new("return Success");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Return {
                resultat: Expression::Identifier("Success".to_string()),
            }
        );
    }
}
