//! Différentiel local contre CUDD réel (M1-8, énoncé C6 — reproduction à
//! l'identique contre un témoin externe indépendant).
//!
//! **Hors CI** : ce test est `#[ignore]` par défaut. Il exige :
//! - l'image Docker `lisage-cudd:latest` (voir `docker/cudd/Dockerfile`) ;
//! - Docker disponible sur la machine.
//!
//! Pour l'exécuter en local :
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

/// Appelle le binaire `cudd_count` dans le conteneur Docker et retourne le
/// nombre de nœuds CUDD pour la table de vérité donnée.
fn cudd_compter_noeuds(nombre_variables: u32, table: &[bool]) -> Option<usize> {
    let mut entree = format!("{}\n", nombre_variables);
    for &valeur in table {
        entree.push_str(if valeur { "1 " } else { "0 " });
    }
    entree.push('\n');

    let docker_bin = std::env::var("DOCKER_BIN").unwrap_or_else(|_| "/usr/bin/docker".to_string());

    // Passe l'entrée via un shell. `printf '%b'` interprète les \n.
    let script = format!(
        "printf '%b' {} | {} run --rm -i lisage-cudd:latest cudd_count",
        shell_quote(&entree),
        shell_quote(&docker_bin),
    );
    let resultat = Command::new("/bin/sh").args(["-c", &script]).output();

    let sortie = match resultat {
        Ok(s) => s,
        Err(e) => {
            eprintln!("échec du spawn de /bin/sh -c : {}", e);
            return None;
        }
    };

    if !sortie.status.success() {
        eprintln!(
            "cudd_count a échoué (code {:?}) : stdout={} stderr={}",
            sortie.status.code(),
            String::from_utf8_lossy(&sortie.stdout),
            String::from_utf8_lossy(&sortie.stderr)
        );
        return None;
    }
    String::from_utf8_lossy(&sortie.stdout).trim().parse().ok()
}

/// Échappe une chaîne pour le shell POSIX.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// C6 local : le nombre de nœuds du BDD classique (arène lisage) égale
    /// celui calculé par CUDD réel (conteneur Docker) sur des tables de
    /// vérité aléatoires.
    #[test]
    #[ignore = "hors CI — exige l'image Docker lisage-cudd:latest"]
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
            .expect("échec de l'appel au conteneur Docker lisage-cudd");

        prop_assert_eq!(
            noeuds_arene,
            noeuds_cudd as u64,
            "C6 local : reproduction à l'identique du nombre de nœuds contre CUDD réel"
        );
    }
}
