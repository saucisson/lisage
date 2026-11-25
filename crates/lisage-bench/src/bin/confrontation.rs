//! Entry point of the lisage confrontation runner (ADR-0102): one
//! workload, one measurement, one JSON line on stdout. The repetitions
//! (fresh processes) and the aggregation are carried by the orchestrator
//! `python -m scripts bench-confrontation`, not by this binary: the peak
//! RSS is a whole-process measurement, it does not repeat in-process.

use std::process::ExitCode;

use lisage_bench::confrontation::{Workload, execute, serialize_line, serialize_load_from_file};

/// Exit code: incorrect usage.
const CODE_USAGE: u8 = 2;

/// Workload usage line, written as-is on stderr.
const USAGE_WORKLOADS: &str = "workloads: alea-lcg <n> | philosophers <n> <path> | window <size> <nb_bases> [absolu|relatif] | cnf <path> | pnml <path>";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    // `dump-load <workload> <path>`: serializes the load in the standard
    // stdin protocol of the external witnesses (orchestrator feed).
    if arguments.first().map(String::as_str) == Some("dump-charge") {
        let Some(name_workload) = arguments.get(1) else {
            eprintln!("usage: confrontation dump-charge <workload> <path>");
            return ExitCode::from(CODE_USAGE);
        };
        let Some(workload) = Workload::from_name(name_workload) else {
            eprintln!("unknown workload: {name_workload}");
            return ExitCode::from(CODE_USAGE);
        };
        let Some(path) = arguments.get(2) else {
            eprintln!("usage: confrontation dump-charge <workload> <path>");
            return ExitCode::from(CODE_USAGE);
        };
        return match serialize_load_from_file(&workload, path) {
            Ok(load) => {
                print!("{load}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("confrontation: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let Some((name_workload, parameters)) = arguments.split_first() else {
        eprintln!("usage: confrontation <workload> [parameters…]");
        eprintln!("{USAGE_WORKLOADS}");
        return ExitCode::from(CODE_USAGE);
    };
    let Some(workload) = Workload::from_name(name_workload) else {
        eprintln!("unknown workload: {name_workload}");
        eprintln!("{USAGE_WORKLOADS}");
        return ExitCode::from(CODE_USAGE);
    };
    match execute(&workload, parameters, None) {
        Ok(measure) => {
            println!("{}", serialize_line(&measure));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("confrontation: {error}");
            ExitCode::FAILURE
        }
    }
}
