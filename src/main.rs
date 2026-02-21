mod ast;
mod elf;
mod emitter;
mod lexer;
mod parser;

use crate::ast::Instruction;
use crate::elf::Sarcophage;
use crate::emitter::Emitter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use clap::{Arg, ArgAction, Command, value_parser};
use std::fs;
use std::path::Path;

fn cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::new("file")
                .action(ArgAction::Set)
                .value_parser(value_parser!(String))
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .required(true)
                .index(2)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("boot")
                .help("Generates a 512-byte raw disk image (Bootloader) instead of an ELF")
                .action(ArgAction::Set)
                .value_parser(value_parser!(bool))
                .required(false)
                .index(3),
        )
        .arg(
            Arg::new("kbd")
                .help("Defines the keyboard layout (eg: azerty, qwerty)")
                .value_parser(value_parser!(String))
                .required(false)
                .default_value("qwerty")
                .index(4)
                .action(ArgAction::Set),
        )
}

// Le Tisserand : Il parcourt les instructions et remplace les "dema" par le vrai code
pub fn tisser_tablettes(
    instructions_brutes: Vec<Instruction>,
    dossier_courant: &Path,
) -> Vec<Instruction> {
    let mut instructions_finales = Vec::new();

    for instruction in instructions_brutes {
        match instruction {
            Instruction::Smen { .. } => instructions_finales.push(instruction),
            Instruction::Dema { chemin } => {
                // 1. On trouve le chemin absolu du nouveau fichier
                let mut chemin_complet = dossier_courant.join(&chemin);
                if chemin_complet.extension().is_none() {
                    chemin_complet.set_extension("maat");
                }
                // 2. On lit le parchemin
                let code_inclus = fs::read_to_string(&chemin_complet).unwrap_or_else(|_| {
                    panic!("The Scribe could not find the tablet: {:?}", chemin_complet)
                });

                // 3. On relance les Yeux et l'Esprit sur ce nouveau texte
                let lexer = Lexer::new(&code_inclus);
                let mut parser = Parser::new(lexer);

                let mut sous_instructions = Vec::new();
                while parser.not_eof() {
                    sous_instructions.push(parser.parse_instruction());
                }

                // 4. RÉCURSION : On tisse ce nouveau fichier au cas où IL contienne aussi des 'dema' !
                let dossier_parent = chemin_complet.parent().unwrap_or(Path::new(""));
                let sous_instructions_tissees = tisser_tablettes(sous_instructions, dossier_parent);

                // 5. On fusionne les instructions tissées dans notre ligne temporelle principale
                instructions_finales.extend(sous_instructions_tissees);
            }
            // Si c'est une instruction normale, ont la garde intacte
            autre => instructions_finales.push(autre),
        }
    }
    instructions_finales
}

fn main() {
    let matches = cli().get_matches();

    // On utilise if let imbriqués (plus stable sur toutes les versions de Rust)
    if let Some(file) = matches.get_one::<String>("file") {
        if let Some(o) = matches.get_one::<String>("output") {
            let kbd_layout = matches
                .get_one::<String>("kbd")
                .expect("failed to get kbd")
                .clone();

            // --- LA CORRECTION EST ICI ---
            // 1. Le Scribe lit le parchemin principal
            let code_source = fs::read_to_string(file)
                .expect("Erreur fatale : Le Scribe n'a pas pu lire le fichier source principal.");

            // 2. Les Yeux (Lexer) et l'Esprit (Parser) analysent le texte
            let lexer = Lexer::new(&code_source);
            let mut parser = Parser::new(lexer);

            // 3. On remplit le vecteur avec les vraies instructions du fichier
            let mut instructions = Vec::new();
            while parser.not_eof() {
                instructions.push(parser.parse_instruction());
            }
            // On récupère le dossier du fichier principal pour gérer les chemins relatifs
            let chemin_fichier_principal = Path::new(file);
            let dossier_principal = chemin_fichier_principal.parent().unwrap_or(Path::new(""));

            // On aplatit l'arbre syntaxique en résolvant toutes les inclusions
            let instructions_fusionnees = tisser_tablettes(instructions, dossier_principal);

            // 4. Émetteur (Le Marteau)
            // On lui donne maintenant les instructions fusionnées, pures de tout 'dema'
            let emetteur = Emitter::new(instructions_fusionnees, kbd_layout);
            let code_machine = emetteur.generer_binaire(matches.get_flag("boot"));

            let binaire_final = if matches.get_flag("boot") {
                code_machine
            } else {
                Sarcophage::emballer(&code_machine)
            };

            // 6. Écriture sur le disque dur
            fs::write(o, binaire_final).expect("Erreur lors de l'écriture du fichier");

            #[cfg(unix)]
            {
                if !matches.get_flag("boot") {
                    // On ne met les droits d'exécution que pour les ELF, un disque brut s'en fiche
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(o, fs::Permissions::from_mode(0o777))
                        .expect("failed to set execute permissions");
                }
            }
        }
    }
}
