//! Differential of the BDD witnesses (M1-8, statement C6 — identical
//! reproduction): the classic BDD configuration of the arena
//! (`lisage_core::bdd`) and the independent mini-BDD witness
//! (`lisage_bench::temoin_bdd`) must produce the **same node count**
//! and the **same semantics** on any random Boolean function.

use lisage_algebra::interface::Anchor;
use lisage_bench::temoin_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, compter_noeuds, evaluer};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

proptest! {
    /// C6: on a random truth table, the node count of the classic BDD
    /// (arena) equals that of the mini-BDD witness, and the evaluation
    /// of both agrees with the table on all 2^n assignments.
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
            "C6: identical reproduction of the node count"
        );
        for a in 0..(1u64 << nombre_variables) {
            let attendu = table[a as usize];
            prop_assert_eq!(evaluer(&pool, racine, a).unwrap(), attendu, "arena, assignment {}", a);
            prop_assert_eq!(temoin.evaluer(racine_temoin, a), attendu, "witness, assignment {}", a);
        }
    }
}
