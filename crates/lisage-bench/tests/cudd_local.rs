//! Local differential against real CUDD (M1-8, statement C6 — identical
//! reproduction against an independent external witness).
//!
//! **Outside CI**: this test is `#[ignore]` by default. It requires:
//! - the Docker image `lisage-cudd:latest` (see `docker/cudd/Dockerfile`);
//! - Docker available on the machine.
//!
//! To run it locally:
//! ```sh
//! docker build -t lisage-cudd:latest docker/cudd
//! cargo nextest run -p lisage-bench --test cudd_local --run-ignored all
//! ```

use lisage_algebra::interface::Anchor;
use lisage_bench::temoin_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;
use std::process::Command;

/// Calls the `cudd_count` binary in the Docker container and returns the
/// CUDD node count for the given truth table, in the lisage convention
/// (distinct (node, complement-bit) pairs, terminals included).
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

    /// Local C6: the node count of the classic BDD (lisage arena) equals
    /// the one computed by real CUDD (Docker container) on random
    /// truth tables.
    #[test]
    #[ignore = "outside CI — requires the Docker image lisage-cudd:latest"]
    fn test_c6_local_arene_vs_cudd(
        number_variables in 2u32..=8,
        table in prop::collection::vec(any::<bool>(), 4..=256),
    ) {
        let size = 1usize << number_variables;
        let table: Vec<bool> = table.into_iter().cycle().take(size).collect();

        let keys = BddKeys::new(number_variables);
        let mut pool = Arena::new(BddController::new(number_variables), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let root = compiler_bdd(&mut pool, anchor, &keys, &table);
        let nodes_arene = count_nodes(&pool, root);

        let nodes_cudd = cudd_count_nodes(number_variables, &table)
            .expect("call to the lisage-cudd Docker container failed");

        prop_assert_eq!(
            nodes_arene,
            nodes_cudd as u64,
            "local C6: identical reproduction of the node count against real CUDD"
        );
    }

    /// Local C6 (MEM-6): the node count of the independent mini-BDD
    /// witness — whose published binary node is `BinaryNode<LEVEL, LOW,
    /// HIGH>` with the CUDD-style complement bit — equals the one
    /// computed by real CUDD (Docker container) on random truth tables.
    /// `cudd_count` counts distinct (node, complement-bit) pairs,
    /// terminals included: the lisage convention, so the counts are
    /// directly comparable.
    #[test]
    #[ignore = "outside CI — requires the Docker image lisage-cudd:latest"]
    fn test_c6_local_witness_vs_cudd(
        number_variables in 2u32..=8,
        table in prop::collection::vec(any::<bool>(), 4..=256),
    ) {
        let size = 1usize << number_variables;
        let table: Vec<bool> = table.into_iter().cycle().take(size).collect();

        let mut witness = MiniBdd::new();
        let root_witness = witness.compiler_table(number_variables, &table);
        let nodes_witness = witness.count_nodes(root_witness);

        let nodes_cudd = cudd_count_nodes(number_variables, &table)
            .expect("call to the lisage-cudd Docker container failed");

        prop_assert_eq!(
            nodes_witness,
            nodes_cudd as u64,
            "local C6: identical reproduction of the witness node count against real CUDD"
        );
    }
}

/// Variable-order pin (MEM-6): on the asymmetric table
/// `x0 AND (x1 OR x2)` over 3 variables, the lisage order (variable 0
/// at the root) gives 5 nodes (3 internal + 2 terminals); the reversed
/// order would give 6. The CUDD witness must reproduce 5 — this pins
/// that `cudd_count` builds with variable l = bit l of the index,
/// variable 0 at the root, like lisage.
#[test]
#[ignore = "outside CI — requires the Docker image lisage-cudd:latest"]
fn test_c6_local_order_pinned_asymmetric() {
    let table = [false, false, false, true, false, true, false, true];

    let mut witness = MiniBdd::new();
    let root_witness = witness.compiler_table(3, &table);
    assert_eq!(
        witness.count_nodes(root_witness),
        5,
        "lisage order: 3 internal + 2 terminals"
    );

    let nodes_cudd =
        cudd_count_nodes(3, &table).expect("call to the lisage-cudd Docker container failed");
    assert_eq!(
        nodes_cudd, 5,
        "CUDD must build the same variable order (reversed would give 6)"
    );
}
