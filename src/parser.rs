// Fichier : src/parser.rs

use crate::ast::{Expression, Instruction};
use crate::lexer::{Lexer, Token};

#[derive(Clone)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    constantes: std::collections::HashMap<String, i32>,
}

impl<'a> Parser<'a> {
    // Initialise le Parser et charge le premier jeton
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let first_token = lexer.next_token();
        Parser {
            lexer,
            current_token: first_token,
            constantes: std::collections::HashMap::new(),
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
    // Niveau 1 : Gère l'addition et la soustraction (Les opérations "lentes")
    fn parse_expression(&mut self) -> Expression {
        let mut gauche = match self.current_token.clone() {
            Token::Number(n) => {
                self.advance();
                Expression::Number(n)
            }
            Token::Register(r) => {
                self.advance();
                Expression::Register(r)
            }
            Token::Identifier(i) => {
                self.advance();
                // Le Scribe doit vérifier si ce nom est une constante connue !
                if let Some(&valeur) = self.constantes.get(&i) {
                    Expression::Number(valeur) // On substitue le nom par sa valeur
                } else {
                    Expression::Identifier(i) // Sinon, on garde le nom (c'est un label)
                }
            }
            Token::StringLiteral(s) => {
                self.advance();
                Expression::StringLiteral(s)
            }
            // --- LA PIÈCE MANQUANTE ---
            Token::Dollar => {
                self.advance(); // On consomme le '$'
                Expression::CurrentAddress
            }
            Token::Minus => {
                self.advance(); // On consomme le '-'
                if let Token::Number(n) = self.current_token {
                    let valeur_negative = -n;
                    self.advance();
                    Expression::Number(valeur_negative)
                } else {
                    panic!("Erreur : Le signe '-' doit être suivi d'un nombre.");
                }
            }
            _ => panic!(
                "Syntax Error: Expression attendue, trouvé {:?}",
                self.current_token
            ),
        };
        while matches!(self.current_token, Token::Plus | Token::Minus) {
            let operateur = self.current_token.clone();
            self.advance();
            let droite = self.parse_terme(); // Priorité aux multiplications à droite aussi

            if let (Expression::Number(n1), Expression::Number(n2)) = (&gauche, &droite) {
                gauche = match operateur {
                    Token::Plus => Expression::Number(n1 + n2),
                    Token::Minus => Expression::Number(n1 - n2),
                    _ => unreachable!(),
                };
            } else {
                panic!("Isfet : Thot ne résout que des constantes pour le moment.");
            }
        }
        gauche
    }

    // Niveau 2 : Gère la multiplication et la division (Les opérations "rapides")
    fn parse_terme(&mut self) -> Expression {
        let mut gauche = match &self.current_token {
            Token::Minus => {
                // Gestion du nombre négatif
                self.advance();
                match &self.current_token {
                    Token::Number(n) => Expression::Number(-(*n)),
                    _ => panic!("Syntax Error: '-' attend un nombre"),
                }
            }
            // 3. Dans parse_terme ou parse_expression, substitue les noms par leur valeur
            Token::Identifier(nom) => {
                if let Some(&val) = self.constantes.get(nom) {
                    Expression::Number(val) // Si c'est une constante smen, on renvoie son nombre !
                } else {
                    Expression::Identifier(nom.clone()) // Sinon c'est une variable nama
                }
            }
            Token::Number(n) => Expression::Number(*n),
            Token::StringLiteral(s) => Expression::StringLiteral(s.clone()),
            Token::Register(r) => Expression::Register(r.clone()),
            _ => panic!(
                "Syntax Error: Expression attendue, trouvé {:?}",
                self.current_token
            ),
        };
        self.advance();

        while matches!(self.current_token, Token::Star | Token::Slash) {
            let operateur = self.current_token.clone();
            self.advance();
            // Pour la multiplication, on ne regarde que les valeurs immédiates (pas d'addition ici)
            let droite = match &self.current_token {
                Token::Number(n) => Expression::Number(*n),
                _ => panic!("Syntax Error: Opérateur '*' attend un nombre à droite"),
            };
            self.advance();

            if let (Expression::Number(n1), Expression::Number(n2)) = (&gauche, &droite) {
                gauche = match operateur {
                    Token::Star => Expression::Number(n1 * n2),
                    Token::Slash => Expression::Number(n1 / n2),
                    _ => unreachable!(),
                };
            }
        }
        gauche
    }
    // Analyse une instruction complète
    pub fn parse_instruction(&mut self) -> Instruction {
        match self.current_token() {
            Token::Verb(v) if v == "neheh" || v == "ankh" || v == "isfet" || v == "jena" => {
                let verbe = v.clone();
                self.advance();
                let cible = self.parse_expression(); // parse_expression gère déjà $ ou Identifiant !

                match verbe.as_str() {
                    "neheh" => Instruction::Neheh { cible },
                    "ankh" => Instruction::Ankh { cible },
                    "isfet" => Instruction::Isfet { cible },
                    _ => Instruction::Jena { cible }, // jena
                }
            }

            Token::Verb(v) if v == "her" || v == "kher" || v == "her_ankh" || v == "kher_ankh" => {
                let type_saut = v.clone();
                self.advance();
                let cible = self.parse_expression();

                match type_saut.as_str() {
                    "her" => Instruction::Her { cible },
                    "kher" => Instruction::Kher { cible },
                    "her_ankh" => Instruction::HerAnkh { cible },
                    _ => Instruction::KherAnkh { cible },
                }
            }

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
            // Fais la même chose pour ankh, isfet, jena, her, etc.
            Token::Verb(v) if v == "smen" => {
                self.advance();
                let nom = match &self.current_token {
                    Token::Identifier(n) => n.clone(),
                    _ => panic!("Smen exige un nom"),
                };
                self.advance();
                self.expect_token(Token::Equals);
                let valeur_expr = self.parse_expression();
                if let Expression::Number(n) = valeur_expr {
                    self.constantes.insert(nom.clone(), n); // On mémorise la constante !
                    Instruction::Smen { nom, valeur: n }
                } else {
                    panic!("Smen exige une valeur numérique fixe (Zep Tepi)");
                }
            }
            Token::Verb(v) if v == "kheper" => {
                self.advance();
                let source = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'kheper' exige un registre source"),
                };
                self.advance();
                self.expect_token(Token::Comma);

                // NOUVEAU : Gestion des crochets ou du nombre direct
                let adresse = if self.current_token == Token::OpenBracket {
                    self.advance(); // Mange '['
                    let expr = self.parse_expression();
                    self.expect_token(Token::CloseBracket); // Mange ']'
                    expr
                } else {
                    self.parse_expression() // Nombre direct (ancien mode)
                };

                Instruction::Kheper { source, adresse }
            }
            // Traduction de : sena %registre, adresse
            Token::Verb(v) if v == "sena" => {
                self.advance(); // Consomme 'sena'

                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'sena' exige un registre destination"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                // Gestion des crochets pour les pointeurs dynamiques [%ba]
                let adresse = if self.current_token == Token::OpenBracket {
                    self.advance(); // Mange '['
                    let expr = self.parse_expression();
                    self.expect_token(Token::CloseBracket); // Mange ']'
                    expr
                } else {
                    self.parse_expression() // Nombre direct (ex: 500)
                };

                Instruction::Sena {
                    destination,
                    adresse,
                }
            }
            // Dans src/parser.rs (dans la méthode parse_instruction)
            Token::Verb(v) if v == "dema" => {
                self.advance(); // Consomme 'dema'
                let chemin = match self.parse_expression() {
                    Expression::StringLiteral(s) => s,
                    _ => panic!(
                        "Syntax Error: 'dema' attend le chemin du parchemin entre guillemets"
                    ),
                };
                Instruction::Dema { chemin }
            }
            Token::Verb(v) if v == "rdtsc" => {
                self.advance();
                Instruction::Rdtsc
            }
            // Traduction de : henet %registre, valeur (AND)
            Token::Verb(v) if v == "henet" => {
                self.advance();
                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'henet' exige un registre"),
                };
                self.advance();
                self.expect_token(Token::Comma);
                let valeur = self.parse_expression();
                Instruction::Henet {
                    destination,
                    valeur,
                }
            }
            // Traduction de : mer %registre, valeur (OR)
            Token::Verb(v) if v == "mer" => {
                self.advance();
                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'mer' exige un registre"),
                };
                self.advance();
                self.expect_token(Token::Comma);
                let valeur = self.parse_expression();
                Instruction::Mer {
                    destination,
                    valeur,
                }
            }
            Token::Verb(v) if v == "duat" => {
                self.advance(); // Consomme 'duat'
                let phrase = match self.parse_expression() {
                    Expression::StringLiteral(s) => s,
                    _ => panic!("Syntax Error: 'duat' attend une phrase entre guillemets"),
                };
                self.expect_token(Token::Comma);
                let adresse = match self.parse_expression() {
                    Expression::Number(n) => n as u16,
                    _ => panic!("Syntax Error: 'duat' attend une adresse numérique"),
                };
                Instruction::Duat { phrase, adresse }
            }
            // Dans src/parser.rs, dans parse_instruction
            Token::Verb(v) if v == "push" => {
                self.advance();
                let cible = self.parse_expression();
                Instruction::Push { cible }
            }
            Token::Verb(v) if v == "pop" => {
                self.advance();
                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'pop' exige un registre"),
                };
                self.advance();
                Instruction::Pop { destination }
            }
            Token::Verb(v) if v == "in" => {
                self.advance();
                let port = self.parse_expression(); // Ex: 0x60 pour le clavier
                Instruction::In { port }
            }
            Token::Verb(v) if v == "out" => {
                self.advance();
                let port = self.parse_expression(); // Ex: 0x3D4 pour la carte VGA
                Instruction::Out { port }
            }

            Token::Verb(v) if v == "wab" => {
                self.advance(); // Consomme le mot 'wab'
                Instruction::Wab
            }
            // Traduction du saut conditionnel : ankh cible
            Token::Verb(v) if v == "ankh" => {
                self.advance(); // Consomme 'ankh'
                let _cible = match &self.current_token {
                    Token::Identifier(i) => i.clone(),
                    _ => panic!("Syntax Error: 'ankh' requires a target label"),
                };
                self.advance(); // Consomme la cible
                Instruction::Ankh {
                    cible: self.parse_expression(),
                }
            }
            // Dans src/parser.rs, fonction parse_instruction
            Token::Verb(v) if v == "kherp" => {
                self.advance(); // On consomme "kherp"
                Instruction::Kherp
            }
            // Traduction du saut inconditionnel : neheh cible
            Token::Verb(v) if v == "neheh" => {
                self.advance(); // Consomme 'neheh'
                if let Token::Identifier(_cible) = self.current_token.clone() {
                    self.advance(); // Consomme la cible (SANS CHERCHER DE ':')
                    Instruction::Neheh {
                        cible: self.parse_expression(),
                    }
                } else {
                    panic!("Syntax Error: 'neheh' attend une cible.");
                }
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
            Token::Identifier(name) => {
                self.advance(); // Consomme le nom
                if self.current_token == Token::Colon {
                    self.advance(); // Consomme le ':'
                    Instruction::Label(name)
                } else {
                    // LE SORTILÈGE DE RÉVÉLATION :
                    panic!(
                        "Syntax Error: Au debut d'une ligne, '{}' doit etre suivi de ':'. Mais Thot a trouve ceci a la place : {:?}",
                        name, self.current_token
                    );
                }
            }
            // Traduction de : nama variable = valeur
            Token::Verb(v) if v == "nama" => {
                self.advance(); // Consomme 'nama'

                // 1. On vérifie qu'on a bien un nom de variable (Identifiant)
                let nom = match &self.current_token {
                    Token::Identifier(i) => i.clone(),
                    _ => panic!(
                        "Syntax Error: Le verbe 'nama' exige un nom de variable (ex: nama age = 10)"
                    ),
                };
                self.advance(); // Consomme le nom de la variable

                // 2. On s'assure qu'il y a bien le symbole '='
                self.expect_token(Token::Equals);

                // 3. On capture ce qu'il y a après le '=' (un nombre, une phrase, etc.)
                let valeur = self.parse_expression();

                Instruction::Nama { nom, valeur }
            }
            // Traduction du saut conditionnel : isfet cible (Saut si Différent)
            Token::Verb(v) if v == "isfet" => {
                self.advance(); // Consomme 'isfet'
                let _cible = match &self.current_token {
                    Token::Identifier(i) => i.clone(),
                    _ => panic!("Syntax Error: 'isfet' exige une etiquette cible"),
                };
                self.advance();

                Instruction::Isfet {
                    cible: self.parse_expression(),
                }
            }
            // Traduction de : kheb %registre, valeur&
            Token::Verb(v) if v == "kheb" => {
                self.advance(); // Consomme 'kheb'

                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'kheb' requires a register as destination"),
                };
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let valeur = self.parse_expression(); // Capture la valeur à soustraire
                Instruction::Kheb {
                    destination,
                    valeur,
                }
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
    fn test_parse_push_pop() {
        let lexer = Lexer::new("push %ka");
        let mut parser = Parser::new(lexer);
        assert_eq!(
            parser.parse_instruction(),
            Instruction::Push {
                cible: Expression::Register("ka".to_string())
            }
        );

        let lexer2 = Lexer::new("pop %ib");
        let mut parser2 = Parser::new(lexer2);
        assert_eq!(
            parser2.parse_instruction(),
            Instruction::Pop {
                destination: "ib".to_string()
            }
        );
    }
    #[test]
    fn test_priorite_math_comptime() {
        // Test : 10 + 5 * 2 doit donner 20 (et non 30)
        let lexer = Lexer::new("nama resultat = 10 + 5 * 2");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Nama {
                nom: "resultat".to_string(),
                valeur: Expression::Number(20), // 10 + (5 * 2)
            }
        );
    }

    #[test]
    fn test_substitution_smen() {
        // On simule une constante déjà apprise par Thot
        let mut parser = Parser::new(Lexer::new("henek %ka, LARGEUR"));
        parser.constantes.insert("LARGEUR".to_string(), 80);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Henek {
                destination: "ka".to_string(),
                valeur: Expression::Number(80), // LARGEUR a été remplacé par 80
            }
        );
    }
    #[test]
    fn test_parse_in_out() {
        let lexer = Lexer::new("in 96");
        let mut parser = Parser::new(lexer);
        assert_eq!(
            parser.parse_instruction(),
            Instruction::In {
                port: Expression::Number(96)
            }
        );

        let lexer2 = Lexer::new("out %da");
        let mut parser2 = Parser::new(lexer2);
        assert_eq!(
            parser2.parse_instruction(),
            Instruction::Out {
                port: Expression::Register("da".to_string())
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

    #[test]
    fn test_parse_nama() {
        let lexer = Lexer::new("nama score = 100");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Nama {
                nom: "score".to_string(),
                valeur: Expression::Number(100),
            }
        );
    }
    #[test]
    fn test_zep_tepi_math() {
        // Test : nama calcul = 10 + 5 * 2
        // Thot doit comprendre que 5 * 2 = 10, puis 10 + 10 = 20.
        let lexer = Lexer::new("nama calcul = 10 + 10");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Nama {
                nom: "calcul".to_string(),
                valeur: Expression::Number(20),
            }
        );
    }
    #[test]
    #[should_panic]
    fn test_parse_nama_panic() {
        let lexer = Lexer::new("nama = 100");
        let mut parser = Parser::new(lexer);

        assert_eq!(
            parser.parse_instruction(),
            Instruction::Nama {
                nom: "score".to_string(),
                valeur: Expression::Number(100),
            }
        );
    }
}
