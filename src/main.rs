mod ast;
mod boot;
mod elf;
mod emitter;
mod lexer;
mod parser;

use crate::boot::Naos;
use crate::elf::Sarcophage;
use crate::emitter::Emitter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use clap::{Arg, ArgAction, Command, value_parser};
use std::fs;
use std::fs::read_to_string;

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

fn main() {
    let matches = cli().get_matches();
    if let Some(file) = matches.get_one::<String>("file")
        && let Some(o) = matches.get_one::<String>("output")
    {
        // 1. Notre code source Maât (Zep Tepi)
        // On met la valeur 42 dans %ka, on ajoute 1 avec sema, et on quitte !
        let code_source = read_to_string(file).expect("Unable to read file");
        // 2. Lexer (Les Yeux)
        let lexer = Lexer::new(code_source.as_str());
        // 3. Parser (L'Esprit)
        let mut parser = Parser::new(lexer);
        let kbd_layout = matches
            .get_one::<String>("kbd")
            .expect("failed to get kbd")
            .clone();

        // 4. Émetteur (Le Marteau)
        // On passe maintenant la disposition du clavier !
        let mut instructions = Vec::new();
        // On boucle pour lire toutes les lignes jusqu'à la fin (Eof)
        while parser.not_eof() {
            instructions.push(parser.parse_instruction());
        }
        let emetteur = Emitter::new(instructions, kbd_layout);
        let code_machine = emetteur.generer_binaire();
        let binaire_final = if matches.get_flag("boot") {
            Naos::emballer(&code_machine)
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
