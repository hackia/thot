mod ast;
mod elf;
mod emitter;
mod lexer;
mod parser;
mod register;

use crate::ast::Instruction;
use crate::elf::Sarcophagus;
use crate::emitter::Emitter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use clap::{Arg, Command, value_parser};
use crossterm::execute;
use crossterm::style::{Print, Stylize};
use crossterm::terminal::size;
use std::fs;
use std::path::Path;

fn cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(Arg::new("maat").required(true))
        .arg(Arg::new("output").required(true).index(2))
        .arg(
            Arg::new("boot")
                .required(false)
                .index(3)
                .value_parser(value_parser!(bool))
                .default_value("false"),
        )
}

// Le Tisserand : Il parcourt les instructions et remplace les "dema" par le vrai code
pub fn tiss_tablet(
    instructions_brutes: Vec<Instruction>,
    dossier_courant: &Path,
    m: &Path,
    tablets: &mut Vec<String>,
) -> Vec<Instruction> {
    let mut instructions_finales = Vec::new();
    for instruction in instructions_brutes {
        match instruction {
            Instruction::Smen { .. } => instructions_finales.push(instruction),
            Instruction::Dema { path } => {
                // 1. On trouve le chemin absolu du nouveau fichier
                let mut chemin_complet = dossier_courant.join(&path);
                if chemin_complet.extension().is_none() {
                    chemin_complet.set_extension("maat");
                }
                if chemin_complet == m {
                    panic!("A tablet cannot include itself");
                } else {
                    if tablets.contains(&chemin_complet.to_str().expect("").to_string()) {
                        panic!("A tablet cannot include itself");
                    }
                }
                tablets.push(chemin_complet.to_str().unwrap().to_string());
                ok_tablet(
                    chemin_complet
                        .file_name()
                        .expect("")
                        .to_str()
                        .expect("")
                        .replace(".maat", ""),
                );
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
                let sous_instructions_tissees =
                    tiss_tablet(sous_instructions, dossier_parent, m, tablets);

                // 5. On fusionne les instructions tissées dans notre ligne temporelle principale
                instructions_finales.extend(sous_instructions_tissees);
            }
            // Si c'est une instruction normale, ont la garde intacte
            autre => instructions_finales.push(autre),
        }
    }
    instructions_finales
}

fn ok_tablet(tablet: String) {
    let (w, _) = size().expect("Failed to get size");
    let description = "Tablet has been compiled successfully";
    let x = "* ".to_string();
    let cr = " [ ".to_string();
    let cl = " ] ".to_string();
    let status = "ok".to_string();
    let y = tablet.to_string();
    let padding = w
        - y.chars().count() as u16
        - cr.chars().count() as u16
        - cl.chars().count() as u16
        - status.chars().count() as u16
        - description.chars().count() as u16
        - x.chars().count() as u16
        - 5;
    execute!(
        std::io::stdout(),
        Print(x.green().bold()),
        Print(format!(
            "The tablet {} has been compiled successfully",
            y.green().bold()
        )),
        Print(" ".repeat(padding as usize)),
        Print(cr.white().bold()),
        Print(status.green().bold()),
        Print(cl.white().bold()),
        Print("\n")
    )
    .unwrap();
}

fn main() {
    let matches = cli().get_matches();
    let mut tablets = Vec::new();

    // On utilise if let imbriqués (plus stable sur toutes les versions de Rust)
    if let Some(file) = matches.get_one::<String>("maat")
        && let Some(out) = matches.get_one::<String>("output")
    {
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
        let instructions_fusionnees =
            tiss_tablet(instructions, dossier_principal, Path::new(file.as_str()),&mut tablets);

        let bin = Emitter::new()
            .add_instruction(instructions_fusionnees.clone())
            .set_kbd_layout(String::from("qwerty"))
            .generer_binaire(true);

        let binary = if matches.get_flag("boot") {
            bin
        } else {
            Sarcophagus::packaging(&bin)
        };
        fs::write(out, binary).expect("Failed to write bin");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(out, fs::Permissions::from_mode(0o755))
                .expect("Failed to set permissions");
        }
        ok_tablet(file.replace(".maat", ""));
    }
}
