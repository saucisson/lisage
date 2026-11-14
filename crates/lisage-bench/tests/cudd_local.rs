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
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, compter_noeuds};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;
use std::process::Command;

/// Calls the `cudd_count` binary in the Docker container and returns the
/// CUDD node count for the given truth table.
fn cudd_compter_noeuds(nombre_variables: u32, table: &[bool]) -> Option<usize> {
    let mut entree = format!("{}\n", nombre_variables);
    for &valeur in table {
        entree.push_str(if valeur { "1 " } else { "0 " });
    }
    entree.push('\n');

    let docker_bin = std::env::var("DOCKER_BIN").unwrap_or_else(|_| "/usr/bin/docker".to_string());

    // Passes the input via a shell. `printf '%b'` interprets the \n.
    let script = format!(
        "printf '%b' {} | {} run --rm -i lisage-cudd:latest cudd_count",
        shell_quote(&entree),
        shell_quote(&docker_bin),
    );
    let resultat = Command::new("/bin/sh").args(["-c", &script]).output();

    let sortie = match resultat {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to spawn /bin/sh -c: {}", e);
            return None;
        }
    };

    if !sortie.status.success() {
        eprintln!(
            "cudd_count failed (code {:?}): stdout={} stderr={}",
            sortie.status.code(),
            String::from_utf8_lossy(&sortie.stdout),
            String::from_utf8_lossy(&sortie.stderr)
        );
        return None;
    }
    String::from_utf8_lossy(&sortie.stdout).trim().parse().ok()
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
        nombre_variables in 2u32..=8,
        table in prop::collection::vec(any::<bool>(), 4..=256),
    ) {
        let taille = 1usize << nombre_variables;
        let table: Vec<bool> = table.into_iter().cycle().take(taille).collect();

        let keys = BddKeys::new(nombre_variables);
        let mut pool = Arena::new(BddController::new(nombre_variables), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let racine = compiler_bdd(&mut pool, anchor, &keys, &table);
        let noeuds_arene = compter_noeuds(&pool, racine);

        let noeuds_cudd = cudd_compter_noeuds(nombre_variables, &table)
            .expect("call to the lisage-cudd Docker container failed");

        prop_assert_eq!(
            noeuds_arene,
            noeuds_cudd as u64,
            "local C6: identical reproduction of the node count against real CUDD"
        );
    }
}
