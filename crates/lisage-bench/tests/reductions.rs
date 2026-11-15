//! PBT of the confluence of the reductions § 2.3 (M1-2, statement C1):
//! `reduce∘reduce = reduce` (idempotence) and commutation of the quotients
//! (R-collapse and R-default commute, whatever the order).

use std::collections::BTreeMap;

use lisage_algebra::interface::NodeId;
use lisage_algebra::reduction::{Reduction, reduce, reduce_inverse};
use proptest::prelude::*;

/// Node 0 plays the role of the default diagram D_δ in the tests.
const NODE_DEFAUT: NodeId = NodeId(0);

fn est_defaut(node: NodeId) -> bool {
    node == NODE_DEFAUT
}

fn vers_arcs(paires: Vec<(u8, u32)>) -> BTreeMap<u64, NodeId> {
    paires
        .into_iter()
        .map(|(key, cible)| (u64::from(key), NodeId(cible)))
        .collect()
}

#[test]
fn test_collapse_level_value_exemple_spec() {
    // val(a): 0 and 1 lead to the same sub-DD ⇒ R-collapse (spec § 2.3 example).
    let x = NodeId(5);
    let arcs = BTreeMap::from([(0, x), (1, x)]);
    let result = reduce(arcs, None, &est_defaut, 2);
    assert_eq!(result, Reduction::Collapse(x));
}

#[test]
fn test_r_default_arc_absent_vers_defaut() {
    // presence: absent ↦ ⊥ (D_δ) is removed, present ↦ X stays.
    let x = NodeId(5);
    let arcs = BTreeMap::from([(0, NODE_DEFAUT), (1, x)]);
    let result = reduce(arcs, Some(0), &est_defaut, 2);
    assert_eq!(result, Reduction::Form(BTreeMap::from([(1, x)])));
}

#[test]
fn test_pas_de_collapse_si_cible_non_defaut() {
    // {present ↦ X} with implicit default ⊥ and X ≠ ⊥: no collapse.
    let x = NodeId(5);
    let arcs = BTreeMap::from([(1, x)]);
    let result = reduce(arcs, Some(0), &est_defaut, 2);
    assert_eq!(result, Reduction::Form(BTreeMap::from([(1, x)])));
}

#[test]
fn test_collapse_si_tout_vaut_le_defaut() {
    // {absent ↦ ⊥, present ↦ ⊥} ⇒ collapse to ⊥.
    let arcs = BTreeMap::from([(0, NODE_DEFAUT), (1, NODE_DEFAUT)]);
    let result = reduce(arcs, Some(0), &est_defaut, 2);
    assert_eq!(result, Reduction::Collapse(NODE_DEFAUT));
}

proptest! {
    /// Confluence: `reduce∘reduce = reduce` — the reduced form is a
    /// fixpoint of the reduction.
    #[test]
    fn test_confluence_idempotence(
        paires in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        key_defaut in proptest::option::of(0u8..4),
        cardinality in 1u64..5,
    ) {
        let arcs = vers_arcs(paires);
        let key = key_defaut.map(u64::from);
        let first = reduce(arcs.clone(), key, &est_defaut, cardinality);
        if let Reduction::Form(arcs_reduceds) = &first {
            let seconde = reduce(arcs_reduceds.clone(), key, &est_defaut, cardinality);
            prop_assert_eq!(seconde, first, "reduce∘reduce = reduce");
        }
    }

    /// Commutation of the quotients: R-collapse then R-default == R-default
    /// then R-collapse.
    #[test]
    fn test_confluence_commutation(
        paires in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        key_defaut in proptest::option::of(0u8..4),
        cardinality in 1u64..5,
    ) {
        let arcs = vers_arcs(paires);
        let key = key_defaut.map(u64::from);
        let direct = reduce(arcs.clone(), key, &est_defaut, cardinality);
        let inverse = reduce_inverse(arcs, key, &est_defaut, cardinality);
        prop_assert_eq!(direct, inverse, "the two reduction orders commute");
    }
}
