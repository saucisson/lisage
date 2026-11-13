//! Local differential against real CUDD (M1-8, statement C6 — exact
//! reproduction against an independent external witness).
//!
//! **Out of CI**: this test is `#[ignore]` by default. It requires:
//! - the Docker image `lisage-cudd:latest` (see `docker/cudd/Dockerfile`);
//! - Docker available on the machine.
//!
//! To run it locally:
//! ```sh
//! docker build -t lisage-cudd:latest docker/cudd
//! cargo nextest run -p lisage-bench --test cudd_local --run-ignored all
//! ```

use lisage_algebra::interface::Anchor;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compile_bdd, count_nodes};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;
use std::process::Command;

/// Calls the `cudd_count` binary in the Docker container and returns the
/// CUDD node count for the given truth table.
fn cudd_count_nodes(number_variables: u32, table: &[bool]) -> Option<usize> {
    let mut input = format!("{}\n", number_variables);
    for &value in table {
        input.push_str(if value { "1 " } else { "0 " });
    }
    input.push('\n');

    let docker_bin = std::env::var("DOCKER_BIN").unwrap_or_else(|_| "/usr/bin/docker".to_string());

    // Passes the input via a shell. `printf '%b'` interprets the \n.
    let script = format!(
        "printf '%b' {} | {} run --rm -i lisage-cudd:latest cudd_count",
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
            "cudd_count failed (code {:?}): stdout={} stderr={}",
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

    /// C6 local: the node count of the classic BDD (lisage arena) equals
    /// the one computed by real CUDD (Docker container) on random truth
    /// tables.
    #[test]
    #[ignore = "out of CI — requires the Docker image lisage-cudd:latest"]
    fn test_c6_local_arena_vs_cudd(
        number_variables in 2u32..=8,
        table in prop::collection::vec(any::<bool>(), 4..=256),
    ) {
        let size = 1usize << number_variables;
        let table: Vec<bool> = table.into_iter().cycle().take(size).collect();

        let keys = BddKeys::new(number_variables);
        let mut pool = Arena::new(BddController::new(number_variables), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let root = compile_bdd(&mut pool, anchor, &keys, &table);
        let nodes_arena = count_nodes(&pool, root);

        let nodes_cudd = cudd_count_nodes(number_variables, &table)
            .expect("call to the lisage-cudd Docker container failed");

        prop_assert_eq!(
            nodes_arena,
            nodes_cudd as u64,
            "C6 local: exact reproduction of the node count against real CUDD"
        );
    }
}
