//! Differential of the BDD witnesses (M1-8, statement C6 — identical
//! reproduction): the classic BDD configuration of the arena
//! (`lisage_core::bdd`) and the independent mini-BDD witness
//! (`lisage_bench::temoin_bdd`) must produce the **same node count**
//! and the **same semantics** on any random Boolean function.

use lisage_algebra::interface::Anchor;
use lisage_bench::temoin_bdd::{BinaryNode, MiniBdd, TERMINAL_FAUX, TERMINAL_VRAI};
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes, evaluate};
use lisage_core::observer::NoOpObserver;
use proptest::prelude::*;

/// MEM-6: the published binary node of the witness — static size
/// pinned, typed view of a fixed function (XOR over 2 variables), and
/// the C6 count (3 internal nodes + 2 terminals = 5).
#[test]
fn test_published_binary_node() {
    // XOR over 2 variables: terminals 0/1, then internal nodes built in
    // order — index 2: (level 1, low = false, high = true),
    // index 3: (level 1, low = true, high = false), index 4: root
    // (level 0, low = 2, high = 3).
    let table = [false, true, true, false];
    let mut witness = MiniBdd::new();
    let root = witness.compiler_table(2, &table);
    assert_eq!(root, 4);

    // Static size of the published binary node: one byte, the
    // complement bit — level/low/high are const generics.
    assert_eq!(BinaryNode::<1, TERMINAL_FAUX, TERMINAL_VRAI>::SIZE, 1);

    // Typed view: the runtime fields must match the const generics.
    let node_level1 = witness
        .binary_node::<1, TERMINAL_FAUX, TERMINAL_VRAI>(2)
        .expect("node 2 is (level 1, low = false, high = true)");
    let node_root = witness
        .binary_node::<0, 2, 3>(root)
        .expect("the root is (level 0, low = 2, high = 3)");
    assert!(
        witness
            .binary_node::<1, TERMINAL_VRAI, TERMINAL_FAUX>(2)
            .is_none()
    );
    assert_eq!(BinaryNode::<0, 2, 3>::level(), 0);
    assert_eq!(BinaryNode::<0, 2, 3>::low(), 2);
    assert_eq!(BinaryNode::<0, 2, 3>::high(), 3);
    assert!(!node_level1.is_complemented());
    assert!(node_root.negate().is_complemented());
    assert!(!node_root.negate().negate().is_complemented());

    // C6 count on the fixed function: 3 internal nodes + 2 terminals.
    assert_eq!(witness.count_nodes(root), 5);
}

proptest! {
    /// C6: on a random truth table, the node count of the classic BDD
    /// (arena) equals that of the mini-BDD witness, and the evaluation
    /// of both agrees with the table on all 2^n assignments.
    #[test]
    fn test_c6_reproduction_identique_arene_vs_temoin(
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

        let mut temoin = MiniBdd::new();
        let root_temoin = temoin.compiler_table(number_variables, &table);
        let nodes_temoin = temoin.count_nodes(root_temoin);

        prop_assert_eq!(
            nodes_arene, nodes_temoin,
            "C6: identical reproduction of the node count"
        );
        for a in 0..(1u64 << number_variables) {
            let expected = table[a as usize];
            prop_assert_eq!(evaluate(&pool, root, a).unwrap(), expected, "arena, assignment {}", a);
            prop_assert_eq!(temoin.evaluate(root_temoin, a), expected, "witness, assignment {}", a);
        }
    }
}
