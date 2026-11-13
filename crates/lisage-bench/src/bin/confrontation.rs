//! Entry point of the lisage confrontation runner (ADR-0102): one
//! workload, one measurement, one JSON line on stdout. Repetitions (fresh
//! processes) and aggregation are handled by the orchestrator
//! `python -m scripts bench-confrontation`, not by this binary: peak RSS
//! is a whole-process measurement, it does not repeat in-process.

use std::process::ExitCode;

use lisage_bench::confrontation::{Workload, execute, serialize_line};

/// Exit code: incorrect usage.
const CODE_USAGE: u8 = 2;

/// Workload usage line, written as-is to stderr.
const USAGE_CHARGES: &str = "workloads: random-lcg <n> | philosophers <n> <path> | window <size> <base_count> [absolute|relative]";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((name_workload, parameters)) = arguments.split_first() else {
        eprintln!("usage: confrontation <workload> [parameters…]");
        eprintln!("{USAGE_CHARGES}");
        return ExitCode::from(CODE_USAGE);
    };
    let Some(workload) = Workload::since_name(name_workload) else {
        eprintln!("unknown workload: {name_workload}");
        eprintln!("{USAGE_CHARGES}");
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
