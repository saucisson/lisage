//! Tests of the PNML parser and the conversion to `lang/` (M5-2):
//! parsing of a toy model, initial marking, step semantics,
//! reachable space computed by the reference interpreter (path A),
//! typed errors, and properties (ADR-0012) on generated nets.

use std::collections::BTreeSet;

use lisage_bench::pnml::{
    NetworkPetri, PlaceId, PnmlDocument, PnmlError, TransitionId, est_sensibilisee,
    marquage_depuis_value, marquage_initial, pas_semantique, space_atteignable, tirer,
};
use lisage_lang::value::Atom;
use proptest::prelude::*;

/// Toy model: a source place with one token, two transitions
/// to two sink places. Reachable space = 3 markings.
const PNML_JOUET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pnml xmlns="http://www.pnml.org/version-2009/grammar/pnml">
  <net id="net1">
    <place id="p0">
      <name><text>source</text></name>
      <initialMarking><text>1</text></initialMarking>
    </place>
    <place id="p1"><name><text>puits_a</text></name></place>
    <place id="p2"><name><text>puits_b</text></name></place>
    <transition id="t1"><name><text>aller_a</text></name></transition>
    <transition id="t2"><name><text>aller_b</text></name></transition>
    <arc id="a1" source="p0" target="t1"/>
    <arc id="a2" source="t1" target="p1"/>
    <arc id="a3" source="p0" target="t2"/>
    <arc id="a4" source="t2" target="p2"/>
  </net>
</pnml>"#;

/// Model with two tokens on one place, one loop transition:
/// p0 (2 tokens), t : p0 → p0. Reachable space = 1 marking.
const PNML_BOUCL: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <transition id="t"/>
  <arc id="a1" source="p0" target="t"/>
  <arc id="a2" source="t" target="p0"/>
</net></pnml>"#;

#[test]
fn test_parse_modele_joue() {
    let network = NetworkPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    assert_eq!(network.number_places(), 3);
    assert_eq!(network.number_transitions(), 2);
    assert_eq!(network.arcs.len(), 4);
    assert_eq!(network.places[0].identifiant, PlaceId("p0".to_string()));
    assert_eq!(network.places[0].nom.as_deref(), Some("source"));
    assert_eq!(network.places[0].marquage_initial, 1);
    assert_eq!(network.places[1].marquage_initial, 0);
    assert_eq!(
        network.transitions[0].identifiant,
        TransitionId("t1".to_string())
    );
    assert_eq!(network.transitions[0].nom.as_deref(), Some("aller_a"));
}

#[test]
fn test_marquage_initial_vers_value() {
    let network = NetworkPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let value = marquage_initial(&network);
    let marquage = marquage_depuis_value(&value).unwrap();
    assert_eq!(marquage.get(&Atom(0)), Some(&Atom(1)));
    assert_eq!(marquage.get(&Atom(1)), Some(&Atom(0)));
    assert_eq!(marquage.get(&Atom(2)), Some(&Atom(0)));
}

#[test]
fn test_sensibilisation_et_tir() {
    let network = NetworkPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let initial = marquage_depuis_value(&marquage_initial(&network)).unwrap();
    let t1 = TransitionId("t1".to_string());
    let t2 = TransitionId("t2".to_string());
    assert!(est_sensibilisee(&network, &t1, &initial));
    assert!(est_sensibilisee(&network, &t2, &initial));
    let apres_t1 = tirer(&network, &t1, &initial);
    assert_eq!(apres_t1.get(&Atom(0)), Some(&Atom(0)));
    assert_eq!(apres_t1.get(&Atom(1)), Some(&Atom(1)));
    assert!(!est_sensibilisee(&network, &t1, &apres_t1));
    assert!(!est_sensibilisee(&network, &t2, &apres_t1));
}

#[test]
fn test_space_atteignable_voie_a() {
    let network = NetworkPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let space = space_atteignable(&network);
    // 3 markings: initial, after t1, after t2.
    assert_eq!(space.len(), 3);
    let initial = marquage_initial(&network);
    assert!(space.contains(&initial));
}

#[test]
fn test_pas_semantique_depuis_set() {
    let network = NetworkPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let initial = BTreeSet::from([marquage_initial(&network)]);
    let successeurs = pas_semantique(&network, &initial);
    assert_eq!(successeurs.len(), 2);
}

#[test]
fn test_boucle_space_stable() {
    let network = NetworkPetri::depuis_pnml(&PnmlDocument(PNML_BOUCL.to_string())).unwrap();
    let space = space_atteignable(&network);
    assert_eq!(space.len(), 1);
}

#[test]
fn test_error_xml_mal_form() {
    let result =
        NetworkPetri::depuis_pnml(&PnmlDocument("<pnml><net><place id=\"p0\">".to_string()));
    assert!(matches!(result, Err(PnmlError::XmlMalformed { .. })));
}

#[test]
fn test_error_attribut_manquant() {
    let result = NetworkPetri::depuis_pnml(&PnmlDocument(
        "<pnml><net><place/></net></pnml>".to_string(),
    ));
    assert!(matches!(
        result,
        Err(PnmlError::AttributManquant {
            element: "place",
            attribut: "id"
        })
    ));
}

#[test]
fn test_error_arc_vers_inconnu() {
    let document = "<pnml><net><place id=\"p0\"/><transition id=\"t1\"/><arc source=\"p0\" target=\"t9\"/></net></pnml>";
    let result = NetworkPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(result, Err(PnmlError::ArcVersInconnu { .. })));
}

#[test]
fn test_error_arc_mal_oriente() {
    // Two linked transitions: badly oriented arc, rejected at parsing.
    let document = "<pnml><net><transition id=\"t1\"/><transition id=\"t2\"/><arc source=\"t1\" target=\"t2\"/></net></pnml>";
    let result = NetworkPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(result, Err(PnmlError::ArcMalOriente { .. })));
}

#[test]
fn test_error_poids_non_supporte() {
    let document = "<pnml><net><place id=\"p0\"/><transition id=\"t1\"/><arc source=\"p0\" target=\"t1\" weight=\"2\"/></net></pnml>";
    let result = NetworkPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(result, Err(PnmlError::PoidsNonSupporte { .. })));
}

#[test]
fn test_error_element_inexpected() {
    // Root other than <pnml>.
    let result = NetworkPetri::depuis_pnml(&PnmlDocument("<foo/>".to_string()));
    assert!(matches!(
        result,
        Err(PnmlError::ElementInexpected {
            expected: "pnml",
            ..
        })
    ));
    // <pnml> without <net>.
    let result = NetworkPetri::depuis_pnml(&PnmlDocument("<pnml/>".to_string()));
    assert!(matches!(
        result,
        Err(PnmlError::ElementInexpected {
            expected: "net",
            ..
        })
    ));
}

#[test]
fn test_error_number_invalide() {
    let document = "<pnml><net><place id=\"p0\"><initialMarking><text>abc</text></initialMarking></place></net></pnml>";
    let result = NetworkPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(
        result,
        Err(PnmlError::NumberInvalide {
            element: "initialMarking",
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// Properties (ADR-0012): generated nets, semantic invariants
// ---------------------------------------------------------------------------

/// Generates a random PNML document: `number_places` places (initial
/// marking 0..=2), `number_transitions` transitions, each transition
/// linked to an input place and an output place (possibly
/// the same).
fn network_aleatoire(
    number_places: usize,
    number_transitions: usize,
    marquages: Vec<u64>,
) -> String {
    let mut document = String::from("<pnml><net>");
    for (index, marquage) in marquages.iter().enumerate() {
        document.push_str(&format!(
            "<place id=\"p{index}\"><initialMarking><text>{marquage}</text></initialMarking></place>"
        ));
    }
    for index in 0..number_transitions {
        document.push_str(&format!("<transition id=\"t{index}\"/>"));
    }
    for index in 0..number_transitions {
        let input = index % number_places.max(1);
        let output = (index + 1) % number_places.max(1);
        document.push_str(&format!(
            "<arc source=\"p{input}\" target=\"t{index}\"/><arc source=\"t{index}\" target=\"p{output}\"/>"
        ));
    }
    document.push_str("</net></pnml>");
    document
}

proptest! {
    /// The reachable space is a fixpoint: no successor outside the
    /// space, and it contains the initial marking.
    #[test]
    fn propriete_space_atteignable_point_fixe(
        number_places in 1usize..=4,
        number_transitions in 1usize..=4,
        marquages in proptest::collection::vec(0u64..=2, 1..=4),
    ) {
        let marquages = {
            let mut m = marquages;
            m.resize(number_places, 0);
            m.truncate(number_places);
            m
        };
        let document = network_aleatoire(number_places, number_transitions, marquages);
        let network = NetworkPetri::depuis_pnml(&PnmlDocument(document.to_string())).unwrap();
        let space = space_atteignable(&network);
        prop_assert!(space.contains(&marquage_initial(&network)));
        let successeurs = pas_semantique(&network, &space);
        prop_assert!(successeurs.is_subset(&space));
    }

    /// Firing an enabled transition preserves the total number of
    /// tokens when each input place also has an outgoing arc
    /// (loop net) — firing invariant.
    #[test]
    fn propriete_tir_sensibilise_produit_successeur_valide(
        number_places in 1usize..=4,
        number_transitions in 1usize..=4,
        marquages in proptest::collection::vec(1u64..=3, 1..=4),
    ) {
        let marquages = {
            let mut m = marquages;
            m.resize(number_places, 1);
            m.truncate(number_places);
            m
        };
        let document = network_aleatoire(number_places, number_transitions, marquages);
        let network = NetworkPetri::depuis_pnml(&PnmlDocument(document.to_string())).unwrap();
        let initial = marquage_depuis_value(&marquage_initial(&network)).unwrap();
        let total_avant: u64 = initial.values().map(|a| a.0).sum();
        for transition in &network.transitions {
            if est_sensibilisee(&network, &transition.identifiant, &initial) {
                let next = tirer(&network, &transition.identifiant, &initial);
                // The resulting marking has one entry per place.
                prop_assert_eq!(next.len(), number_places);
                // Documented invariant: firing moves one token (weight 1
                // in input and in output), the total is preserved.
                let total_apres: u64 = next.values().map(|a| a.0).sum();
                prop_assert_eq!(total_apres, total_avant);
            }
        }
    }
}
