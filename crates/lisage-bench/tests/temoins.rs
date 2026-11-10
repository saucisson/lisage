//! Différentiel des témoins BDD (M1-8, énoncé C6 — reproduction à
//! l'identique) : la configuration BDD classique de l'arène
//! (`lisage_core::bdd`) et le mini-BDD témoin indépendant
//! (`lisage_bench::temoin_bdd`) doivent produire le **même nombre de nœuds**
//! et la **même sémantique** sur toute fonction booléenne aléatoire.

use lisage_algebra::interface::Anchor;
use lisage_bench::temoin_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, compter_noeuds, evaluer};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

proptest! {
    /// C6 : sur une table de vérité aléatoire, le nombre de nœuds du BDD
    /// classique (arène) égale celui du mini-BDD témoin, et l'évaluation
    /// des deux agrees avec la table sur les 2^n affectations.
    #[test]
    fn test_c6_reproduction_identique_arene_vs_temoin(
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

        let mut temoin = MiniBdd::new();
        let racine_temoin = temoin.compiler_table(nombre_variables, &table);
        let noeuds_temoin = temoin.compter_noeuds(racine_temoin);

        prop_assert_eq!(
            noeuds_arene, noeuds_temoin,
            "C6 : reproduction à l'identique du nombre de nœuds"
        );
        for a in 0..(1u64 << nombre_variables) {
            let attendu = table[a as usize];
            prop_assert_eq!(evaluer(&pool, racine, a).unwrap(), attendu, "arène, affectation {}", a);
            prop_assert_eq!(temoin.evaluer(racine_temoin, a), attendu, "témoin, affectation {}", a);
        }
    }
}
