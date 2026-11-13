//! Tests of the classic BDD configuration (M1-8, C6): compilation by
//! truth table, node counting, evaluation.

use lisage_algebra::interface::Anchor;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compile_bdd, count_nodes, evaluer};
use lisage_core::observer::NoOpObserver;

type PoolBdd = Arena<BddController>;

fn pool(number_variables: u32) -> PoolBdd {
    Arena::new(BddController::new(number_variables), Box::new(NoOpObserver))
}

/// Truth table of the constant ⊥.
fn table_constant(number_variables: u32, value: bool) -> Vec<bool> {
    vec![value; 1usize << number_variables]
}

/// Truth table of the XOR of two variables.
fn table_xor2() -> Vec<bool> {
    (0..4u64).map(|a| (a & 1) != ((a >> 1) & 1)).collect()
}

#[test]
fn test_bdd_constants_one_only_terminal() {
    let mut pool = pool(3);
    let anchor = Anchor(0);
    let keys = BddKeys::new(3);
    let root_false = compile_bdd(&mut pool, anchor, &keys, &table_constant(3, false));
    let root_true = compile_bdd(&mut pool, anchor, &keys, &table_constant(3, true));
    // R-collapse: no variable, a single terminal visitd.
    assert_eq!(count_nodes(&pool, root_false), 1);
    assert_eq!(count_nodes(&pool, root_true), 1);
    assert!(!evaluer(&pool, root_false, 0b101).unwrap());
    assert!(evaluer(&pool, root_true, 0b101).unwrap());
}

#[test]
fn test_bdd_xor2_count_and_evaluated() {
    let mut pool = pool(2);
    let anchor = Anchor(0);
    let keys = BddKeys::new(2);
    let table = table_xor2();
    let root = compile_bdd(&mut pool, anchor, &keys, &table);
    // XOR(x0, x1): root x0 + two distinct x1 nodes + ⊥/⊤ = 5 nodes.
    assert_eq!(count_nodes(&pool, root), 5);
    for a in 0..4u64 {
        assert_eq!(
            evaluer(&pool, root, a).unwrap(),
            table[a as usize],
            "affectation {a}"
        );
    }
}

#[test]
fn test_bdd_projection_collapse() {
    let mut pool = pool(2);
    let anchor = Anchor(0);
    let keys = BddKeys::new(2);
    // Projection onto x0: the two potential branches of the root x1
    // are constant, R-collapse reduces them — root x0 → (⊥, ⊤), 3 nodes.
    let table = (0..4u64).map(|a| a & 1 == 1).collect::<Vec<bool>>();
    let root = compile_bdd(&mut pool, anchor, &keys, &table);
    assert_eq!(count_nodes(&pool, root), 3);
}
