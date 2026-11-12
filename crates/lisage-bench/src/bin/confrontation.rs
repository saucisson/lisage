//! Point d'entrée du runner lisage de la confrontation (ADR-0102) : une
//! charge, une mesure, une ligne JSON sur stdout. Les répétitions (processus
//! neufs) et l'agrégation sont portées par l'orchestrateur
//! `python -m scripts bench-confrontation`, pas par ce binaire : la RSS
//! pic est une mesure du processus entier, elle ne se répète pas in-process.

use std::process::ExitCode;

use lisage_bench::confrontation::{Charge, executer, serialiser_ligne};

/// Code de sortie : usage incorrect.
const CODE_USAGE: u8 = 2;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((nom_charge, parametres)) = arguments.split_first() else {
        eprintln!("usage: confrontation <charge> [paramètres…]");
        eprintln!(
            "charges: alea-lcg <n> | philosophes <n> <chemin> | fenetre <taille> <nb_bases> [absolu|relatif]"
        );
        return ExitCode::from(CODE_USAGE);
    };
    let Some(charge) = Charge::depuis_nom(nom_charge) else {
        eprintln!("charge inconnue: {nom_charge}");
        eprintln!(
            "charges: alea-lcg <n> | philosophes <n> <chemin> | fenetre <taille> <nb_bases> [absolu|relatif]"
        );
        return ExitCode::from(CODE_USAGE);
    };
    match executer(&charge, parametres, None) {
        Ok(mesure) => {
            println!("{}", serialiser_ligne(&mesure));
            ExitCode::SUCCESS
        }
        Err(erreur) => {
            eprintln!("confrontation: {erreur}");
            ExitCode::FAILURE
        }
    }
}
