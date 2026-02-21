use crate::ast::{Expression, Instruction, Level};
use crate::lexer::{Lexer, Token};
use crate::register::{
    ensure_helix_fits, ensure_number_fits, ensure_same_level, parse_general_register,
    parse_register, RegKind, RegBase,
};

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
            Token::Helix(ra, apophis) => {
                self.advance();
                Expression::Helix { ra, apophis }
            }
            Token::Register(r) => {
                let _ = parse_register(&r);
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
                    panic!("Error: The '-' sign must be followed by a number.");
                }
            }
            _ => panic!(
                "Syntax Error: Expression expected, found {:?}",
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
                panic!("Isfet: Thot only solves constants at the moment.");
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
                    _ => panic!("Syntax Error: '-' expects a number"),
                }
            }
            Token::Helix(ra, apophis) => Expression::Helix {
                ra: *ra,
                apophis: *apophis,
            },
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
            Token::Register(r) => {
                let _ = parse_register(r);
                Expression::Register(r.clone())
            }
            _ => panic!(
                "Syntax Error: Expression expected, found {:?}",
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

                let value = self.parse_expression(); // Capture la valeur (ex: 10)
                let dest_spec = parse_register(&destination);
                match dest_spec.kind {
                    RegKind::Segment(_) => match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            if src_spec.level != Level::Base {
                                panic!("Segment moves require base registers: %{src}");
                            }
                        }
                        _ => panic!("Segment registers require a register source."),
                    },
                    RegKind::General(_) => {
                        if dest_spec.level <= Level::High {
                            match &value {
                                Expression::Register(src) => {
                                    let src_spec = parse_general_register(src);
                                    ensure_same_level(
                                        "henek",
                                        &destination,
                                        dest_spec.level,
                                        src,
                                        src_spec.level,
                                    );
                                }
                                Expression::Helix { ra, apophis } => {
                                    ensure_helix_fits(
                                        "henek",
                                        &destination,
                                        dest_spec.level,
                                        *ra as u128,
                                        *apophis as u128,
                                    );
                                }
                                Expression::Number(n) => {
                                    ensure_number_fits("henek", &destination, dest_spec.level, *n);
                                }
                                _ => {}
                            }
                        } else if dest_spec.level == Level::Extreme {
                            match &value {
                                Expression::Register(src) => {
                                    let src_spec = parse_general_register(src);
                                    ensure_same_level(
                                        "henek",
                                        &destination,
                                        dest_spec.level,
                                        src,
                                        src_spec.level,
                                    );
                                }
                                Expression::Helix { .. } => {}
                                _ => panic!(
                                    "Henek for 128-bit registers only accepts Helix literals or registers."
                                ),
                            }
                        } else {
                            panic!(
                                "Henek does not yet support registers beyond Extreme: %{} ({})",
                                destination, dest_spec.level
                            );
                        }
                    }
                }

                Instruction::Henek {
                    destination,
                    value,
                }
            }
            // Fais la même chose pour ankh, isfet, jena, her, etc.
            Token::Verb(v) if v == "smen" => {
                self.advance();
                let nom = match &self.current_token {
                    Token::Identifier(n) => n.clone(),
                    _ => panic!("Smen required a name"),
                };
                self.advance();
                self.expect_token(Token::Equals);
                let valeur_expr = self.parse_expression();
                if let Expression::Number(n) = valeur_expr {
                    self.constantes.insert(nom.clone(), n); // On mémorise la constante !
                    Instruction::Smen { nom, valeur: n }
                } else {
                    panic!("Smen requires a fixed numerical value (Zep Tepi)");
                }
            }
            Token::Verb(v) if v == "kheper" => {
                self.advance();
                let source = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'kheper' requires a source registry"),
                };
                let _ = parse_general_register(&source);
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
                let _ = parse_general_register(&destination);
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                // Gestion des crochets pour les pointeurs dynamiques [%ba]
                let adresse = if self.current_token == Token::OpenBracket {
                    self.advance(); // Mange '['
                    let expr = self.parse_expression();
                    self.expect_token(Token::CloseBracket); // Mange ']'
                    if let Expression::Register(r) = &expr {
                        let _ = parse_general_register(r);
                    }
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
                        "Syntax Error: 'dema' waits for the path of the scroll in quotes"
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
                let dest_spec = parse_general_register(&destination);
                self.advance();
                self.expect_token(Token::Comma);
                let value = self.parse_expression();
                if dest_spec.level <= Level::High {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "henet",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "henet",
                                &destination,
                                dest_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                        }
                        Expression::Number(n) => {
                            ensure_number_fits("henet", &destination, dest_spec.level, *n);
                        }
                        _ => {}
                    }
                } else if dest_spec.level == Level::Extreme {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "henet",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Henet for 128-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else {
                    panic!(
                        "Henet does not yet support registers beyond Extreme: %{} ({})",
                        destination, dest_spec.level
                    );
                }
                Instruction::Henet {
                    destination,
                    value,
                }
            }
            // Traduction de : mer %registre, valeur (OR)
            Token::Verb(v) if v == "mer" => {
                self.advance();
                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'mer' requires a registry"),
                };
                let dest_spec = parse_general_register(&destination);
                self.advance();
                self.expect_token(Token::Comma);
                let value = self.parse_expression();
                if dest_spec.level <= Level::High {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "mer",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "mer",
                                &destination,
                                dest_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                        }
                        Expression::Number(n) => {
                            ensure_number_fits("mer", &destination, dest_spec.level, *n);
                        }
                        _ => {}
                    }
                } else if dest_spec.level == Level::Extreme {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "mer",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Mer for 128-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else {
                    panic!(
                        "Mer does not yet support registers beyond Extreme: %{} ({})",
                        destination, dest_spec.level
                    );
                }
                Instruction::Mer {
                    destination,
                    value,
                }
            }
            Token::Verb(v) if v == "duat" => {
                self.advance(); // Consomme 'duat'
                let phrase = match self.parse_expression() {
                    Expression::StringLiteral(s) => s,
                    _ => panic!("Syntax Error: 'duat' attend une phrase entre guillemets"),
                };
                self.expect_token(Token::Comma);
                let address = match self.parse_expression() {
                    Expression::Number(n) => n as u16,
                    _ => panic!("Syntax Error: 'duat' attend une adresse numérique"),
                };
                Instruction::Duat { phrase, address }
            }
            // Dans src/parser.rs, dans parse_instruction
            Token::Verb(v) if v == "push" => {
                self.advance();
                let cible = self.parse_expression();
                if let Expression::Register(r) = &cible {
                    let _ = parse_general_register(r);
                }
                Instruction::Push { cible }
            }
            Token::Verb(v) if v == "pop" => {
                self.advance();
                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'pop' exige un registre"),
                };
                let _ = parse_general_register(&destination);
                self.advance();
                Instruction::Pop { destination }
            }
            Token::Verb(v) if v == "in" => {
                self.advance();
                let port = self.parse_expression(); // Ex: 0x60 pour le clavier
                if let Expression::Register(r) = &port {
                    let reg_spec = parse_general_register(r);
                    if !matches!(reg_spec.kind, RegKind::General(RegBase::Da))
                        || reg_spec.level != Level::Base
                    {
                        panic!("Syntax Error: 'in' requires %da as register port");
                    }
                }
                Instruction::In { port }
            }
            Token::Verb(v) if v == "out" => {
                self.advance();
                let port = self.parse_expression(); // Ex: 0x3D4 pour la carte VGA
                if let Expression::Register(r) = &port {
                    let reg_spec = parse_general_register(r);
                    if !matches!(reg_spec.kind, RegKind::General(RegBase::Da))
                        || reg_spec.level != Level::Base
                    {
                        panic!("Syntax Error: 'out' requires %da as register port");
                    }
                }
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
                let dest_spec = parse_general_register(&destination);
                if !matches!(dest_spec.kind, RegKind::General(RegBase::Ka))
                    || dest_spec.level != Level::Base
                {
                    panic!("Syntax Error: 'sedjem' requires %ka as destination");
                }
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
                let dest_spec = parse_general_register(&destination);
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let value = self.parse_expression(); // Capture la force à unir
                if dest_spec.level <= Level::High {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "sema",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "sema",
                                &destination,
                                dest_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                        }
                        Expression::Number(n) => {
                            ensure_number_fits("sema", &destination, dest_spec.level, *n);
                        }
                        _ => {}
                    }
                } else if dest_spec.level == Level::Extreme {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "sema",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Sema for 128-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else if dest_spec.level == Level::Xenith {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "sema",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Sema for 256-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else {
                    panic!(
                        "Sema does not yet support registers beyond Extreme: %{} ({})",
                        destination, dest_spec.level
                    );
                }
                Instruction::Sema {
                    destination,
                    value,
                }
            }
            // Traduction de : shesa %registre, valeur
            Token::Verb(v) if v == "shesa" => {
                self.advance(); // Consomme 'shesa'

                let destination = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'shesa' requires a register as destination"),
                };
                let dest_spec = parse_general_register(&destination);
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let value = self.parse_expression(); // Capture la force à multiplier
                if dest_spec.level <= Level::High {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "shesa",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "shesa",
                                &destination,
                                dest_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                        }
                        Expression::Number(n) => {
                            ensure_number_fits("shesa", &destination, dest_spec.level, *n);
                        }
                        _ => {}
                    }
                } else if dest_spec.level == Level::Extreme {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "shesa",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Shesa for 128-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else {
                    panic!(
                        "Shesa does not yet support registers beyond Extreme: %{} ({})",
                        destination, dest_spec.level
                    );
                }
                Instruction::Shesa {
                    destination,
                    value,
                }
            }

            // Traduction de : wdj %registre, valeur
            Token::Verb(v) if v == "wdj" => {
                self.advance(); // Consomme 'wdj'

                let left = match &self.current_token {
                    Token::Register(r) => r.clone(),
                    _ => panic!("Syntax Error: 'wdj' requires a register on the left"),
                };
                let left_spec = parse_general_register(&left);
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let right = self.parse_expression(); // Capture la valeur à peser
                if left_spec.level <= Level::High {
                    match &right {
                        Expression::Register(r) => {
                            let right_spec = parse_general_register(r);
                            ensure_same_level("wdj", &left, left_spec.level, r, right_spec.level);
                        }
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "wdj",
                                &left,
                                left_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                        }
                        Expression::Number(n) => {
                            ensure_number_fits("wdj", &left, left_spec.level, *n);
                        }
                        _ => {}
                    }
                } else if left_spec.level == Level::Extreme {
                    match &right {
                        Expression::Register(r) => {
                            let right_spec = parse_general_register(r);
                            ensure_same_level("wdj", &left, left_spec.level, r, right_spec.level);
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Wdj for 128-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else {
                    panic!(
                        "Wdj does not yet support registers beyond Extreme: %{} ({})",
                        left, left_spec.level
                    );
                }
                Instruction::Wdj { left, right }
            }

            // Traduction de : returne valeur
            Token::Verb(v) if v == "return" => {
                self.advance(); // Consomme 'returne'
                let resultat = self.parse_expression(); // Capture ce qu'on renvoie
                if let Expression::Register(r) = &resultat {
                    let reg_spec = parse_general_register(r);
                    if !matches!(reg_spec.kind, RegKind::General(RegBase::Ka))
                        || reg_spec.level != Level::Base
                    {
                        panic!("Syntax Error: 'return' only supports %ka as a register result");
                    }
                }
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
                let name = match &self.current_token {
                    Token::Identifier(i) => i.clone(),
                    _ => panic!(
                        "Syntax Error: Le verbe 'nama' exige un nom de variable (ex: nama age = 10)"
                    ),
                };
                self.advance(); // Consomme le nom de la variable

                // 2. On s'assure qu'il y a bien le symbole '='
                self.expect_token(Token::Equals);

                // 3. On capture ce qu'il y a après le '=' (un nombre, une phrase, etc.)
                let value = self.parse_expression();

                Instruction::Nama { name, value }
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
                let dest_spec = parse_general_register(&destination);
                self.advance(); // Consomme le registre

                self.expect_token(Token::Comma); // Consomme la virgule

                let value = self.parse_expression(); // Capture la valeur à soustraire
                if dest_spec.level <= Level::High {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "kheb",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { ra, apophis } => {
                            ensure_helix_fits(
                                "kheb",
                                &destination,
                                dest_spec.level,
                                *ra as u128,
                                *apophis as u128,
                            );
                        }
                        Expression::Number(n) => {
                            ensure_number_fits("kheb", &destination, dest_spec.level, *n);
                        }
                        _ => {}
                    }
                } else if dest_spec.level == Level::Extreme {
                    match &value {
                        Expression::Register(src) => {
                            let src_spec = parse_general_register(src);
                            ensure_same_level(
                                "kheb",
                                &destination,
                                dest_spec.level,
                                src,
                                src_spec.level,
                            );
                        }
                        Expression::Helix { .. } => {}
                        _ => panic!(
                            "Kheb for 128-bit registers only accepts Helix literals or registers."
                        ),
                    }
                } else {
                    panic!(
                        "Kheb does not yet support registers beyond Extreme: %{} ({})",
                        destination, dest_spec.level
                    );
                }
                Instruction::Kheb { destination, value }
            }
            // (On ajoutera 'wdj', 'sema', etc. ici plus tard)
            _ => panic!("Syntax Error: Unknown instruction {:?}", self.current_token),
        }
    }
}
