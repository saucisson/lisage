//! Tests of the PNML parser and the conversion to `lang/` (M5-2):
//! parsing of a toy model, initial marking, step semantics,
//! reachable space computed by the reference interpreter (lane A),
//! typed errors, and properties (ADR-0012) on generated nets.

use std::collections::BTreeSet;

use lisage_bench::pnml::{
    NetworkPetri, PlaceId, PnmlDocument, PnmlError, TransitionId, fire, is_sensitive,
    marking_initial, marking_since_value, not_semantic, space_reachable,
};
use lisage_lang::value::Atom;
use proptest::prelude::*;

/// Toy model: a source place with one token, two transitions
/// toward two sink places. Reachable space = 3 markings.
const PNML_JOUET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pnml xmlns="http://www.pnml.org/version-2009/grammar/pnml">
  <net id="net1">
    <place id="p0">
      <name><text>source</text></name>
      <initialMarking><text>1</text></initialMarking>
    </place>
    <place id="p1"><name><text>sink_a</text></name></place>
    <place id="p2"><name><text>sink_b</text></name></place>
    <transition id="t1"><name><text>go_to</text></name></transition>
    <transition id="t2"><name><text>go_to_b</text></name></transition>
    <edge id="a1" source="p0" target="t1"/>
    <edge id="a2" source="t1" target="p1"/>
    <edge id="a3" source="p0" target="t2"/>
    <edge id="a4" source="t2" target="p2"/>
  </net>
</pnml>"#;

/// Model with two tokens on one place, one loop transition:
/// p0 (2 tokens), t: p0 → p0. Reachable space = 1 marking.
const PNML_BOUCL: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <transition id="t"/>
  <edge id="a1" source="p0" target="t"/>
  <edge id="a2" source="t" target="p0"/>
</net></pnml>"#;

#[test]
fn test_parse_model_plays() {
    let network = NetworkPetri::since_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    assert_eq!(network.number_places(), 3);
    assert_eq!(network.number_transitions(), 2);
    assert_eq!(network.edges.len(), 4);
    assert_eq!(network.places[0].identifier, PlaceId("p0".to_string()));
    assert_eq!(network.places[0].name.as_deref(), Some("source"));
    assert_eq!(network.places[0].marking_initial, 1);
    assert_eq!(network.places[1].marking_initial, 0);
    assert_eq!(
        network.transitions[0].identifier,
        TransitionId("t1".to_string())
    );
    assert_eq!(network.transitions[0].name.as_deref(), Some("go_to"));
}

#[test]
fn test_marking_initial_toward_value() {
    let network = NetworkPetri::since_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let value = marking_initial(&network);
    let marking = marking_since_value(&value).unwrap();
    assert_eq!(marking.get(&Atom(0)), Some(&Atom(1)));
    assert_eq!(marking.get(&Atom(1)), Some(&Atom(0)));
    assert_eq!(marking.get(&Atom(2)), Some(&Atom(0)));
}

#[test]
fn test_sensitivity_and_draw() {
    let network = NetworkPetri::since_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let initial = marking_since_value(&marking_initial(&network)).unwrap();
    let t1 = TransitionId("t1".to_string());
    let t2 = TransitionId("t2".to_string());
    assert!(is_sensitive(&network, &t1, &initial));
    assert!(is_sensitive(&network, &t2, &initial));
    let after_t1 = fire(&network, &t1, &initial);
    assert_eq!(after_t1.get(&Atom(0)), Some(&Atom(0)));
    assert_eq!(after_t1.get(&Atom(1)), Some(&Atom(1)));
    assert!(!is_sensitive(&network, &t1, &after_t1));
    assert!(!is_sensitive(&network, &t2, &after_t1));
}

#[test]
fn test_space_reachable_lane_a() {
    let network = NetworkPetri::since_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let space = space_reachable(&network);
    // 3 markings: initial, after t1, after t2.
    assert_eq!(space.len(), 3);
    let initial = marking_initial(&network);
    assert!(space.contains(&initial));
}

#[test]
fn test_not_semantic_since_set() {
    let network = NetworkPetri::since_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let initial = BTreeSet::from([marking_initial(&network)]);
    let successors = not_semantic(&network, &initial);
    assert_eq!(successors.len(), 2);
}

#[test]
fn test_loop_space_stable() {
    let network = NetworkPetri::since_pnml(&PnmlDocument(PNML_BOUCL.to_string())).unwrap();
    let space = space_reachable(&network);
    assert_eq!(space.len(), 1);
}

#[test]
fn test_error_xml_bad_form() {
    let result =
        NetworkPetri::since_pnml(&PnmlDocument("<pnml><net><place id=\"p0\">".to_string()));
    assert!(matches!(result, Err(PnmlError::XmlBadForm { .. })));
}

#[test]
fn test_error_attribute_missing() {
    let result = NetworkPetri::since_pnml(&PnmlDocument(
        "<pnml><net><place/></net></pnml>".to_string(),
    ));
    assert!(matches!(
        result,
        Err(PnmlError::AttributeMissing {
            element: "place",
            attribute: "id"
        })
    ));
}

#[test]
fn test_error_edge_toward_unknown() {
    let document = "<pnml><net><place id=\"p0\"/><transition id=\"t1\"/><edge source=\"p0\" target=\"t9\"/></net></pnml>";
    let result = NetworkPetri::since_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(result, Err(PnmlError::EdgeTowardUnknown { .. })));
}

#[test]
fn test_error_edge_bad_oriented() {
    // Two linked transitions: badly oriented edge, rejected at parsing.
    let document = "<pnml><net><transition id=\"t1\"/><transition id=\"t2\"/><edge source=\"t1\" target=\"t2\"/></net></pnml>";
    let result = NetworkPetri::since_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(result, Err(PnmlError::EdgeBadOriented { .. })));
}

#[test]
fn test_error_weight_not_supporte() {
    let document = "<pnml><net><place id=\"p0\"/><transition id=\"t1\"/><edge source=\"p0\" target=\"t1\" weight=\"2\"/></net></pnml>";
    let result = NetworkPetri::since_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(result, Err(PnmlError::WeightNotSupporte { .. })));
}

#[test]
fn test_error_element_unexpected() {
    // Root other than <pnml>.
    let result = NetworkPetri::since_pnml(&PnmlDocument("<foo/>".to_string()));
    assert!(matches!(
        result,
        Err(PnmlError::ElementUnexpected {
            expected: "pnml",
            ..
        })
    ));
    // <pnml> without <net>.
    let result = NetworkPetri::since_pnml(&PnmlDocument("<pnml/>".to_string()));
    assert!(matches!(
        result,
        Err(PnmlError::ElementUnexpected {
            expected: "net",
            ..
        })
    ));
}

#[test]
fn test_error_number_invalid() {
    let document = "<pnml><net><place id=\"p0\"><initialMarking><text>abc</text></initialMarking></place></net></pnml>";
    let result = NetworkPetri::since_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(
        result,
        Err(PnmlError::NumberInvalid {
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
/// linked to an incoming place and an outgoing place (possibly
/// the same).
fn network_random(number_places: usize, number_transitions: usize, markings: Vec<u64>) -> String {
    let mut document = String::from("<pnml><net>");
    for (index, marking) in markings.iter().enumerate() {
        document.push_str(&format!(
            "<place id=\"p{index}\"><initialMarking><text>{marking}</text></initialMarking></place>"
        ));
    }
    for index in 0..number_transitions {
        document.push_str(&format!("<transition id=\"t{index}\"/>"));
    }
    for index in 0..number_transitions {
        let input = index % number_places.max(1);
        let output = (index + 1) % number_places.max(1);
        document.push_str(&format!(
            "<edge source=\"p{input}\" target=\"t{index}\"/><edge source=\"t{index}\" target=\"p{output}\"/>"
        ));
    }
    document.push_str("</net></pnml>");
    document
}

proptest! {
    /// The reachable space is a fixpoint: no successor outside the
    /// space, and it contains the initial marking.
    #[test]
    fn property_space_reachable_point_fixe(
        number_places in 1usize..=4,
        number_transitions in 1usize..=4,
        markings in proptest::collection::vec(0u64..=2, 1..=4),
    ) {
        let markings = {
            let mut m = markings;
            m.resize(number_places, 0);
            m.truncate(number_places);
            m
        };
        let document = network_random(number_places, number_transitions, markings);
        let network = NetworkPetri::since_pnml(&PnmlDocument(document.to_string())).unwrap();
        let space = space_reachable(&network);
        prop_assert!(space.contains(&marking_initial(&network)));
        let successors = not_semantic(&network, &space);
        prop_assert!(successors.is_subset(&space));
    }

    /// Firing an enabled transition preserves the total number of
    /// tokens when each incoming place also has an outgoing edge
    /// (loop net) — firing invariant.
    #[test]
    fn property_draw_sensitive_product_successor_valid(
        number_places in 1usize..=4,
        number_transitions in 1usize..=4,
        markings in proptest::collection::vec(1u64..=3, 1..=4),
    ) {
        let markings = {
            let mut m = markings;
            m.resize(number_places, 1);
            m.truncate(number_places);
            m
        };
        let document = network_random(number_places, number_transitions, markings);
        let network = NetworkPetri::since_pnml(&PnmlDocument(document.to_string())).unwrap();
        let initial = marking_since_value(&marking_initial(&network)).unwrap();
        let total_before: u64 = initial.values().map(|a| a.0).sum();
        for transition in &network.transitions {
            if is_sensitive(&network, &transition.identifier, &initial) {
                let next = fire(&network, &transition.identifier, &initial);
                // The resulting marking has one entry per place.
                prop_assert_eq!(next.len(), number_places);
                // Documented invariant: firing moves one token (weight 1
                // in and out), the total is preserved.
                let total_after: u64 = next.values().map(|a| a.0).sum();
                prop_assert_eq!(total_after, total_before);
            }
        }
    }
}
