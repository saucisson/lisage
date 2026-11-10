//! PBT de confluence des réductions § 2.3 (M1-2, énoncé C1) :
//! `reduce∘reduce = reduce` (idempotence) et commutation des quotients
//! (R-collapse et R-default commutent, quel que soit l'ordre).

use std::collections::BTreeMap;

use lisage_algebra::interface::NodeId;
use lisage_algebra::reduction::{Reduction, reduire, reduire_inverse};
use proptest::prelude::*;

/// Le nœud 0 joue le rôle du diagramme de défaut D_δ dans les tests.
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
    // val(a) : 0 et 1 mènent au même sous-DD ⇒ R-collapse (exemple spec § 2.3).
    let x = NodeId(5);
    let arcs = BTreeMap::from([(0, x), (1, x)]);
    let resultat = reduire(arcs, None, &est_defaut, 2);
    assert_eq!(resultat, Reduction::Collapse(x));
}

#[test]
fn test_r_default_arc_absent_vers_defaut() {
    // présence : absent ↦ ⊥ (D_δ) est supprimé, present ↦ X reste.
    let x = NodeId(5);
    let arcs = BTreeMap::from([(0, NOEUD_DEFAUT), (1, x)]);
    let resultat = reduire(arcs, Some(0), &est_defaut, 2);
    assert_eq!(resultat, Reduction::Forme(BTreeMap::from([(1, x)])));
}

#[test]
fn test_pas_de_collapse_si_cible_non_defaut() {
    // {present ↦ X} avec défaut implicite ⊥ et X ≠ ⊥ : pas de collapse.
    let x = NodeId(5);
    let arcs = BTreeMap::from([(1, x)]);
    let resultat = reduire(arcs, Some(0), &est_defaut, 2);
    assert_eq!(resultat, Reduction::Forme(BTreeMap::from([(1, x)])));
}

#[test]
fn test_collapse_si_tout_vaut_le_defaut() {
    // {absent ↦ ⊥, present ↦ ⊥} ⇒ collapse en ⊥.
    let arcs = BTreeMap::from([(0, NOEUD_DEFAUT), (1, NOEUD_DEFAUT)]);
    let resultat = reduire(arcs, Some(0), &est_defaut, 2);
    assert_eq!(resultat, Reduction::Collapse(NOEUD_DEFAUT));
}

proptest! {
    /// Confluence : `reduce∘reduce = reduce` — la forme réduite est un point
    /// fixe de la réduction.
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

    /// Commutation des quotients : R-collapse puis R-default == R-default
    /// puis R-collapse.
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
        prop_assert_eq!(direct, inverse, "les deux ordres de réduction commutent");
    }
}
