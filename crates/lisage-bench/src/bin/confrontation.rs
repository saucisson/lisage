//! Entry point of the lisage confrontation runner (ADR-0102): one
//! workload, one measurement, one JSON line on stdout. The repetitions
//! (fresh processes) and the aggregation are carried by the orchestrator
//! `python -m scripts bench-confrontation`, not by this binary: the peak
//! RSS is a whole-process measurement, it does not repeat in-process.

use std::process::ExitCode;

use lisage_bench::confrontation::{Workload, executer, serialiser_ligne};

/// Code de output : usage incorrect.
const CODE_USAGE: u8 = 2;

/// Workload usage line, written as-is on stderr.
const USAGE_WORKLOADS: &str =
    "workloads: alea-lcg <n> | philosophers <n> <path> | window <size> <nb_bases> [absolu|relatif]";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((nom_workload, parametres)) = arguments.split_first() else {
        eprintln!("usage: confrontation <workload> [parameters…]");
        eprintln!("{USAGE_WORKLOADS}");
        return ExitCode::from(CODE_USAGE);
    };
    let Some(workload) = Workload::depuis_nom(nom_workload) else {
        eprintln!("unknown workload: {nom_workload}");
        eprintln!("{USAGE_WORKLOADS}");
        return ExitCode::from(CODE_USAGE);
    };
    match executer(&workload, parametres, None) {
        Ok(measure) => {
            println!("{}", serialiser_ligne(&measure));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("confrontation: {error}");
            ExitCode::FAILURE
        }
    }
}
