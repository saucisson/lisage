//! PBT of the confluence of the reductions § 2.3 (M1-2, statement C1):
//! `reduce∘reduce = reduce` (idempotence) and commutation of the quotients
//! (R-collapse and R-default commute, whatever the order).

use std::collections::BTreeMap;

use lisage_algebra::interface::{ArcRun, NodeId};
use lisage_algebra::reduction::{Reduction, reduce, reduce_inverse};
use proptest::prelude::*;

/// Node 0 plays the role of the default diagram D_δ in the tests.
const NODE_DEFAULT: NodeId = NodeId(0);

fn is_default(node: NodeId) -> bool {
    node == NODE_DEFAULT
}

fn to_arcs(pairs: Vec<(u8, u32)>) -> ArcRun<u64> {
    pairs
        .into_iter()
        .map(|(key, target)| (u64::from(key), NodeId(target)))
        .collect()
}

#[test]
fn test_collapse_level_value_exemple_spec() {
    // val(a): 0 and 1 lead to the same sub-DD ⇒ R-collapse (spec § 2.3 example).
    let x = NodeId(5);
    let arcs = ArcRun::from_pairs([(0, x), (1, x)]);
    let result = reduce(arcs, None, &is_default, 2);
    assert_eq!(result, Reduction::Collapse(x));
}

#[test]
fn test_r_default_arc_absent_to_default() {
    // presence: absent ↦ ⊥ (D_δ) is removed, present ↦ X stays.
    let x = NodeId(5);
    let arcs = ArcRun::from_pairs([(0, NODE_DEFAULT), (1, x)]);
    let result = reduce(arcs, Some(0), &is_default, 2);
    assert_eq!(result, Reduction::Form(ArcRun::from_pairs([(1, x)])));
}

#[test]
fn test_no_collapse_if_target_not_default() {
    // {present ↦ X} with implicit default ⊥ and X ≠ ⊥: no collapse.
    let x = NodeId(5);
    let arcs = ArcRun::from_pairs([(1, x)]);
    let result = reduce(arcs, Some(0), &is_default, 2);
    assert_eq!(result, Reduction::Form(ArcRun::from_pairs([(1, x)])));
}

#[test]
fn test_collapse_if_all_equals_the_default() {
    // {absent ↦ ⊥, present ↦ ⊥} ⇒ collapse to ⊥.
    let arcs = ArcRun::from_pairs([(0, NODE_DEFAULT), (1, NODE_DEFAULT)]);
    let result = reduce(arcs, Some(0), &is_default, 2);
    assert_eq!(result, Reduction::Collapse(NODE_DEFAULT));
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
        let arcs = to_arcs(pairs);
        let key = key_default.map(u64::from);
        let first = reduce(arcs.clone(), key, &is_default, cardinality);
        if let Reduction::Form(arcs_reduceds) = &first {
            let second = reduce(arcs_reduceds.clone(), key, &is_default, cardinality);
            prop_assert_eq!(second, first, "reduce∘reduce = reduce");
        }
    }

    /// Last write wins: a duplicate key in the construction keeps the
    /// LAST target (MEM-5 semantics of the sorted run).
    #[test]
    fn test_last_write_wins(
        pairs in prop::collection::vec((0u8..4, 0u32..6), 0..8),
    ) {
        let run = to_arcs(pairs.clone());
        let mut expected: BTreeMap<u64, NodeId> = BTreeMap::new();
        for (key, target) in pairs {
            expected.insert(u64::from(key), NodeId(target));
        }
        // One entry per key: the run holds no duplicate (binary_search in
        // `get` relies on it).
        prop_assert_eq!(run.len(), expected.len());
        let rebuilt: BTreeMap<u64, NodeId> = run.iter().map(|(k, t)| (*k, *t)).collect();
        prop_assert_eq!(rebuilt, expected);
    }

    /// Commutation of the quotients: R-collapse then R-default == R-default
    /// then R-collapse.
    #[test]
    fn test_confluence_commutation(
        pairs in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        key_default in proptest::option::of(0u8..4),
        cardinality in 1u64..5,
    ) {
        let arcs = to_arcs(pairs);
        let key = key_default.map(u64::from);
        let direct = reduce(arcs.clone(), key, &is_default, cardinality);
        let inverse = reduce_inverse(arcs, key, &is_default, cardinality);
        prop_assert_eq!(direct, inverse, "the two reduction orders commute");
    }
}
