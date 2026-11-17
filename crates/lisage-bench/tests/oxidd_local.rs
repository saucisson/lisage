//! Local differential against real OxiDD (M7-2 — identical reproduction
//! against an independent external witness, ADR-0102 pattern).
//!
//! **Outside CI**: this test is `#[ignore]` by default. It requires:
//! - the Docker image `lisage-oxidd:latest` (see `docker/oxidd/Dockerfile`);
//! - Docker available on the machine.
//!
//! To run it locally:
//! ```sh
//! docker build -t lisage-oxidd:latest docker/oxidd
//! cargo nextest run -p lisage-bench --test oxidd_local --run-ignored all
//! ```

use lisage_algebra::interface::Anchor;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;
use std::process::Command;

/// Calls the `oxidd_count` binary in the Docker container and returns the
/// OxiDD node count for the given truth table.
fn oxidd_count_nodes(number_variables: u32, table: &[bool]) -> Option<usize> {
    let mut input = format!("{}\n", number_variables);
    for &value in table {
        input.push_str(if value { "1 " } else { "0 " });
    }
    input.push('\n');

    let docker_bin = std::env::var("DOCKER_BIN").unwrap_or_else(|_| "/usr/bin/docker".to_string());

    // Passes the input via a shell. `printf '%b'` interprets the \n.
    let script = format!(
        "printf '%b' {} | {} run --rm -i lisage-oxidd:latest oxidd_count",
        shell_quote(&input),
        shell_quote(&docker_bin),
    );
    let result = Command::new("/bin/sh").args(["-c", &script]).output();

    let output = match result {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to spawn /bin/sh -c: {}", e);
            return None;
        }
    };

    if !output.status.success() {
        eprintln!(
            "oxidd_count failed (code {:?}): stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Escapes a string for the POSIX shell.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Local M7-2: the node count of the classic BDD (lisage arena) equals
    /// the one computed by real OxiDD (Docker container) on random
    /// truth tables.
    #[test]
    #[ignore = "outside CI — requires the Docker image lisage-oxidd:latest"]
    fn test_m7_2_local_arena_vs_oxidd(
        number_variables in 2u32..=8,
        table in prop::collection::vec(any::<bool>(), 4..=256),
    ) {
        let size = 1usize << number_variables;
        let table: Vec<bool> = table.into_iter().cycle().take(size).collect();

        let keys = BddKeys::new(number_variables);
        let mut pool = Arena::new(BddController::new(number_variables), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let root = compiler_bdd(&mut pool, anchor, &keys, &table);
        let nodes_arena = count_nodes(&pool, root);

        let nodes_oxidd = oxidd_count_nodes(number_variables, &table)
            .expect("call to the lisage-oxidd Docker container failed");

        prop_assert_eq!(
            nodes_arena,
            nodes_oxidd as u64,
            "local M7-2: identical reproduction of the node count against real OxiDD"
        );
    }
}
