//! PBT of confluence of the reductions § 2.3 (M1-2, statement C1):
//! `reduce∘reduce = reduce` (idempotence) and commutation of the quotients
//! (R-collapse and R-default commutes, whatever the order).

use std::collections::BTreeMap;

use lisage_algebra::interface::NodeId;
use lisage_algebra::reduction::{Reduction, reduce, reduce_inverse};
use proptest::prelude::*;

/// Node 0 plays the role of the default diagram D_δ in the tests.
const NODE_DEFAUT: NodeId = NodeId(0);

fn is_default(node: NodeId) -> bool {
    node == NODE_DEFAUT
}

fn toward_edges(pairs: Vec<(u8, u32)>) -> BTreeMap<u64, NodeId> {
    pairs
        .into_iter()
        .map(|(key, target)| (u64::from(key), NodeId(target)))
        .collect()
}

#[test]
fn test_collapse_level_value_example_spec() {
    // val(a): 0 and 1 lead to the same sub-DD ⇒ R-collapse (spec § 2.3 example).
    let x = NodeId(5);
    let edges = BTreeMap::from([(0, x), (1, x)]);
    let result = reduce(edges, None, &is_default, 2);
    assert_eq!(result, Reduction::Collapse(x));
}

#[test]
fn test_r_default_edge_absent_toward_default() {
    // presence: absent ↦ ⊥ (D_δ) is removed, present ↦ X remains.
    let x = NodeId(5);
    let edges = BTreeMap::from([(0, NODE_DEFAUT), (1, x)]);
    let result = reduce(edges, Some(0), &is_default, 2);
    assert_eq!(result, Reduction::Form(BTreeMap::from([(1, x)])));
}

#[test]
fn test_not_of_collapse_si_target_not_default() {
    // {present ↦ X} with implicit default ⊥ and X ≠ ⊥: no collapse.
    let x = NodeId(5);
    let edges = BTreeMap::from([(1, x)]);
    let result = reduce(edges, Some(0), &is_default, 2);
    assert_eq!(result, Reduction::Form(BTreeMap::from([(1, x)])));
}

#[test]
fn test_collapse_si_all_equals_the_default() {
    // {absent ↦ ⊥, present ↦ ⊥} ⇒ collapse to ⊥.
    let edges = BTreeMap::from([(0, NODE_DEFAUT), (1, NODE_DEFAUT)]);
    let result = reduce(edges, Some(0), &is_default, 2);
    assert_eq!(result, Reduction::Collapse(NODE_DEFAUT));
}

proptest! {
    /// Confluence: `reduce∘reduce = reduce` — the reduced form is a
    /// fixpoint of the reduction.
    #[test]
    fn test_confluence_idempotence(
        pairs in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        key_default in proptest::option::of(0u8..4),
        cardinality in 1u64..5,
    ) {
        let edges = toward_edges(pairs);
        let key = key_default.map(u64::from);
        let first = reduce(edges.clone(), key, &is_default, cardinality);
        if let Reduction::Form(edges_reduced) = &first {
            let second = reduce(edges_reduced.clone(), key, &is_default, cardinality);
            prop_assert_eq!(second, first, "reduce∘reduce = reduce");
        }
    }

    /// Commutation of the quotients: R-collapse then R-default == R-default
    /// then R-collapse.
    #[test]
    fn test_confluence_commutation(
        pairs in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        key_default in proptest::option::of(0u8..4),
        cardinality in 1u64..5,
    ) {
        let edges = toward_edges(pairs);
        let key = key_default.map(u64::from);
        let direct = reduce(edges.clone(), key, &is_default, cardinality);
        let inverse = reduce_inverse(edges, key, &is_default, cardinality);
        prop_assert_eq!(direct, inverse, "the two reduction orders commutes");
    }
}
