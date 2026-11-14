//! PBT of the confluence of the reductions § 2.3 (M1-2, statement C1):
//! `reduce∘reduce = reduce` (idempotence) and commutation of the quotients
//! (R-collapse and R-default commute, whatever the order).

use std::collections::BTreeMap;

use lisage_algebra::interface::NodeId;
use lisage_algebra::reduction::{Reduction, reduire, reduire_inverse};
use proptest::prelude::*;

/// Node 0 plays the role of the default diagram D_δ in the tests.
const NOEUD_DEFAUT: NodeId = NodeId(0);

fn est_defaut(node: NodeId) -> bool {
    node == NOEUD_DEFAUT
}

fn vers_arcs(paires: Vec<(u8, u32)>) -> BTreeMap<u64, NodeId> {
    paires
        .into_iter()
        .map(|(cle, cible)| (u64::from(cle), NodeId(cible)))
        .collect()
}

#[test]
fn test_collapse_niveau_valeur_exemple_spec() {
    // val(a): 0 and 1 lead to the same sub-DD ⇒ R-collapse (spec § 2.3 example).
    let x = NodeId(5);
    let arcs = BTreeMap::from([(0, x), (1, x)]);
    let resultat = reduire(arcs, None, &est_defaut, 2);
    assert_eq!(resultat, Reduction::Collapse(x));
}

#[test]
fn test_r_default_arc_absent_vers_defaut() {
    // presence: absent ↦ ⊥ (D_δ) is removed, present ↦ X stays.
    let x = NodeId(5);
    let arcs = BTreeMap::from([(0, NOEUD_DEFAUT), (1, x)]);
    let resultat = reduire(arcs, Some(0), &est_defaut, 2);
    assert_eq!(resultat, Reduction::Forme(BTreeMap::from([(1, x)])));
}

#[test]
fn test_pas_de_collapse_si_cible_non_defaut() {
    // {present ↦ X} with implicit default ⊥ and X ≠ ⊥: no collapse.
    let x = NodeId(5);
    let arcs = BTreeMap::from([(1, x)]);
    let resultat = reduire(arcs, Some(0), &est_defaut, 2);
    assert_eq!(resultat, Reduction::Forme(BTreeMap::from([(1, x)])));
}

#[test]
fn test_collapse_si_tout_vaut_le_defaut() {
    // {absent ↦ ⊥, present ↦ ⊥} ⇒ collapse to ⊥.
    let arcs = BTreeMap::from([(0, NOEUD_DEFAUT), (1, NOEUD_DEFAUT)]);
    let resultat = reduire(arcs, Some(0), &est_defaut, 2);
    assert_eq!(resultat, Reduction::Collapse(NOEUD_DEFAUT));
}

proptest! {
    /// Confluence: `reduce∘reduce = reduce` — the reduced form is a
    /// fixpoint of the reduction.
    #[test]
    fn test_confluence_idempotence(
        paires in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        cle_defaut in proptest::option::of(0u8..4),
        cardinalite in 1u64..5,
    ) {
        let arcs = vers_arcs(paires);
        let cle = cle_defaut.map(u64::from);
        let premiere = reduire(arcs.clone(), cle, &est_defaut, cardinalite);
        if let Reduction::Forme(arcs_reduits) = &premiere {
            let seconde = reduire(arcs_reduits.clone(), cle, &est_defaut, cardinalite);
            prop_assert_eq!(seconde, premiere, "reduce∘reduce = reduce");
        }
    }

    /// Commutation of the quotients: R-collapse then R-default == R-default
    /// then R-collapse.
    #[test]
    fn test_confluence_commutation(
        paires in prop::collection::vec((0u8..4, 0u32..6), 0..6),
        cle_defaut in proptest::option::of(0u8..4),
        cardinalite in 1u64..5,
    ) {
        let arcs = vers_arcs(paires);
        let cle = cle_defaut.map(u64::from);
        let direct = reduire(arcs.clone(), cle, &est_defaut, cardinalite);
        let inverse = reduire_inverse(arcs, cle, &est_defaut, cardinalite);
        prop_assert_eq!(direct, inverse, "the two reduction orders commute");
    }
}
