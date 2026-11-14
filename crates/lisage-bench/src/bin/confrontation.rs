//! Entry point of the lisage confrontation runner (ADR-0102): one
//! workload, one measurement, one JSON line on stdout. The repetitions
//! (fresh processes) and the aggregation are carried by the orchestrator
//! `python -m scripts bench-confrontation`, not by this binary: the peak
//! RSS is a whole-process measurement, it does not repeat in-process.

use std::process::ExitCode;

use lisage_bench::confrontation::{Charge, executer, serialiser_ligne};

/// Code de sortie : usage incorrect.
const CODE_USAGE: u8 = 2;

/// Workload usage line, written as-is on stderr.
const USAGE_CHARGES: &str = "workloads: alea-lcg <n> | philosophes <n> <path> | fenetre <taille> <nb_bases> [absolu|relatif]";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((nom_charge, parametres)) = arguments.split_first() else {
        eprintln!("usage: confrontation <workload> [parameters…]");
        eprintln!("{USAGE_CHARGES}");
        return ExitCode::from(CODE_USAGE);
    };
    let Some(charge) = Charge::depuis_nom(nom_charge) else {
        eprintln!("unknown workload: {nom_charge}");
        eprintln!("{USAGE_CHARGES}");
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
