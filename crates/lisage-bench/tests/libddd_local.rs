//! Local differential against real libDDD (LIP6) — BENCH-1, issue #277,
//! ADR-0102 pattern: identical reproduction against an independent
//! external witness, outside CI.
//!
//! **Outside CI**: this test is `#[ignore]` by default. It requires:
//! - the Docker image `lisage-libddd:latest` (see `docker/libddd/Dockerfile`);
//! - Docker available on the machine.
//!
//! To run it locally:
//! ```sh
//! docker build -t lisage-libddd:latest docker/libddd
//! cargo nextest run -p lisage-bench --test libddd_local --run-ignored all
//! ```

use lisage_algebra::interface::Anchor;
use lisage_bench::witness_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;
use std::process::Command;

/// Calls the `libddd_measure` binary in the Docker container and returns
/// the parsed JSON line (node count in the lisage convention: distinct
/// reachable nodes, terminals included — libDDD has no complement edges).
fn libddd_measure(number_variables: u32, table: &[bool]) -> Option<(u64, u64)> {
    let mut input = format!("{}\n", number_variables);
    for &value in table {
        input.push_str(if value { "1 " } else { "0 " });
    }
    input.push('\n');

    let docker_bin = std::env::var("DOCKER_BIN").unwrap_or_else(|_| "/usr/bin/docker".to_string());

    // Passes the input via a shell. `printf '%b'` interprets the \n.
    let script = format!(
        "printf '%b' {} | {} run --rm -i lisage-libddd:latest libddd_measure",
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
            "libddd_measure failed (code {:?}): stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let nodes = extract_json_u64(&line, "\"noeuds\":")?;
    let verifies = extract_json_u64(&line, "\"verifie\":")?;
    Some((nodes, verifies))
}

/// Extracts an unsigned integer value after a JSON key in a flat line.
fn extract_json_u64(line: &str, key: &str) -> Option<u64> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    rest[..end].trim().parse().ok()
}

/// Escapes a string for the POSIX shell.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Local C6: the node count of the classic BDD (lisage arena) equals
    /// the one computed by real libDDD (Docker container) on random
    /// truth tables, and the cross-check value (satisfying assignments)
    /// matches the table.
    #[test]
    #[ignore = "outside CI — requires the Docker image lisage-libddd:latest"]
    fn test_c6_local_arena_vs_libddd(
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
        let verifies_arena = table.iter().filter(|value| **value).count() as u64;

        let (nodes_libddd, verifies_libddd) = libddd_measure(number_variables, &table)
            .expect("call to the lisage-libddd Docker container failed");

        prop_assert_eq!(
            nodes_arena,
            nodes_libddd,
            "local C6: identical reproduction of the node count against real libDDD"
        );
        prop_assert_eq!(verifies_arena, verifies_libddd);
    }

    /// Local C6: the node count of the independent mini-BDD witness equals
    /// the one computed by real libDDD on random truth tables.
    #[test]
    #[ignore = "outside CI — requires the Docker image lisage-libddd:latest"]
    fn test_c6_local_witness_vs_libddd(
        number_variables in 2u32..=8,
        table in prop::collection::vec(any::<bool>(), 4..=256),
    ) {
        let size = 1usize << number_variables;
        let table: Vec<bool> = table.into_iter().cycle().take(size).collect();

        let mut witness = MiniBdd::new();
        let root_witness = witness.compiler_table(number_variables, &table);
        let nodes_witness = witness.count_nodes(root_witness);

        let (nodes_libddd, _) = libddd_measure(number_variables, &table)
            .expect("call to the lisage-libddd Docker container failed");

        prop_assert_eq!(
            nodes_witness,
            nodes_libddd,
            "local C6: identical reproduction of the witness node count against real libDDD"
        );
    }
}

/// Variable-order pin (BENCH-1): on the asymmetric table
/// `[1,0,0,1,1,0,0,0]` over 3 variables, the lisage order (variable 0
/// at the root) gives 6 nodes; the reversed order gives 7 (verified by
/// exhaustive enumeration over all 2^8 tables of n = 3). The libDDD
/// witness must reproduce 6 — this pins that `libddd_measure` builds with
/// variable l = bit l of the index, variable 0 at the root, like lisage.
#[test]
#[ignore = "outside CI — requires the Docker image lisage-libddd:latest"]
fn test_c6_local_order_pinned_asymmetric() {
    let table = [true, false, false, true, true, false, false, false];

    let mut witness = MiniBdd::new();
    let root_witness = witness.compiler_table(3, &table);
    assert_eq!(
        witness.count_nodes(root_witness),
        6,
        "lisage order: 6 nodes (reversed order would give 7)"
    );

    let (nodes_libddd, verifies_libddd) =
        libddd_measure(3, &table).expect("call to the lisage-libddd Docker container failed");
    assert_eq!(
        nodes_libddd, 6,
        "libDDD must build the same variable order (reversed would give 7)"
    );
    assert_eq!(
        verifies_libddd, 3,
        "three satisfying assignments in the table"
    );
}
