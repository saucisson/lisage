//! Differential of the BDD witnesses (M1-8, statement C6 — exact
//! reproduction): the classic BDD configuration of the arena
//! (`lisage_core::bdd`) and the independent mini-BDD witness
//! (`lisage_bench::witness_bdd`) must produce the **same node count**
//! and the **same semantics** on any random Boolean function.

use lisage_algebra::interface::Anchor;
use lisage_bench::witness_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compile_bdd, count_nodes, evaluer};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

proptest! {
    /// C6: on a random truth table, the node count of the classic BDD
    /// (arena) equals that of the mini-BDD witness, and the evaluation of
    /// both agrees with the table on all 2^n assignments.
    #[test]
    fn test_c6_reproduction_identical_arena_vs_witness(
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

        let mut witness = MiniBdd::new();
        let root_witness = witness.compile_table(number_variables, &table);
        let nodes_witness = witness.count_nodes(root_witness);

        prop_assert_eq!(
            nodes_arena, nodes_witness,
            "C6: exact reproduction of the node count"
        );
        for a in 0..(1u64 << number_variables) {
            let expected = table[a as usize];
            prop_assert_eq!(evaluer(&pool, root, a).unwrap(), expected, "arena, assignment {}", a);
            prop_assert_eq!(witness.evaluer(root_witness, a), expected, "witness, assignment {}", a);
        }
    }
}
