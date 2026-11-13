//! Tests of the A/B/C triptych on PNML models (M5-2, statements C3/C4):
//! identical cardinality on the three lanes (explicit falsification), lane C
//! at runtime order (valid π adopted, invalid π rejected), typed errors.

use lisage_bench::pnml::{NetworkPetri, PnmlDocument};
use lisage_bench::triptych::{ErrorTriptych, lane_a, lane_b, lane_c};
use lisage_lang::value::Atom;
use lisage_plan::order::StrategieDownstream;

/// Loop: p0 (2 tokens), t: p0 → p0. Reachable space = 1 marking.
const PNML_BOUCL: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <transition id="t"/>
  <edge id="a1" source="p0" target="t"/>
  <edge id="a2" source="t" target="p0"/>
</net></pnml>"#;

/// Transfer: p0 (1 token) → t → p1. Reachable space = 2 markings.
const PNML_TRANSFERT: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>1</text></initialMarking></place>
  <place id="p1"/>
  <transition id="t"/>
  <edge id="a1" source="p0" target="t"/>
  <edge id="a2" source="t" target="p1"/>
</net></pnml>"#;

/// Choice: p0 (1 token), t1 → p1, t2 → p2. Reachable space = 3 markings.
const PNML_CHOIX: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>1</text></initialMarking></place>
  <place id="p1"/>
  <place id="p2"/>
  <transition id="t1"/>
  <transition id="t2"/>
  <edge id="a1" source="p0" target="t1"/>
  <edge id="a2" source="t1" target="p1"/>
  <edge id="a3" source="p0" target="t2"/>
  <edge id="a4" source="t2" target="p2"/>
</net></pnml>"#;

/// Cycle of 3 places, 2 tokens: all compositions of 2 into 3 parts,
/// reachable space = 6 markings.
const PNML_CYCLE2: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <place id="p1"/>
  <place id="p2"/>
  <transition id="t1"/>
  <transition id="t2"/>
  <transition id="t3"/>
  <edge id="a1" source="p0" target="t1"/>
  <edge id="a2" source="t1" target="p1"/>
  <edge id="a3" source="p1" target="t2"/>
  <edge id="a4" source="t2" target="p2"/>
  <edge id="a5" source="p2" target="t3"/>
  <edge id="a6" source="t3" target="p0"/>
</net></pnml>"#;

/// Network with no place.
const PNML_SANS_PLACE: &str = r#"<pnml><net><transition id="t"/></net></pnml>"#;

/// Parses a test PNML model.
fn network(pnml: &str) -> NetworkPetri {
    NetworkPetri::since_pnml(&PnmlDocument(pnml.to_string())).unwrap()
}

/// Canonical order of the place keys of a network with `n` places.
fn order_canonical(n: usize) -> Vec<Atom> {
    (0..n).map(|index| Atom(index as u64)).collect()
}

/// Reversed order of the place keys of a network with `n` places.
fn order_inverse(n: usize) -> Vec<Atom> {
    (0..n).rev().map(|index| Atom(index as u64)).collect()
}

/// C3/C4 falsification: identical cardinality on lanes A, B and C
/// (canonical order and reversed order), for each toy model.
#[test]
fn test_cardinal_identical_on_the_three_lanes() {
    let attendus = [
        (PNML_BOUCL, 1),
        (PNML_TRANSFERT, 2),
        (PNML_CHOIX, 3),
        (PNML_CYCLE2, 6),
    ];
    for (pnml, cardinal_expected) in attendus {
        let network = network(pnml);
        let measure_a = lane_a(&network);
        let measure_b = lane_b(&network).unwrap();
        let result_c_canonical =
            lane_c(&network, &order_canonical(network.number_places())).unwrap();
        let result_c_inverse = lane_c(&network, &order_inverse(network.number_places())).unwrap();
        let measure_c_canonical = result_c_canonical.measure;
        let measure_c_inverse = result_c_inverse.measure;
        assert_eq!(measure_a.cardinal, cardinal_expected, "lane A on {pnml}");
        assert_eq!(measure_b.cardinal, cardinal_expected, "lane B on {pnml}");
        assert_eq!(
            measure_c_canonical.cardinal, cardinal_expected,
            "lane C canonical on {pnml}"
        );
        assert_eq!(
            measure_c_inverse.cardinal, cardinal_expected,
            "lane C reversed on {pnml}"
        );
    }
}

/// Lane A is explicit set-theoretic (0 diagram nodes); lanes B/C build a
/// not-empty DD.
#[test]
fn test_lane_a_without_dd_lanes_b_c_with_dd() {
    let network = network(PNML_CHOIX);
    assert_eq!(lane_a(&network).nodes_live, 0);
    assert!(lane_b(&network).unwrap().nodes_live > 0);
    assert!(
        lane_c(&network, &order_inverse(network.number_places()))
            .unwrap()
            .measure
            .nodes_live
            > 0
    );
}

/// Lane C at canonical order == lane B (same order, same measurements).
#[test]
fn test_lane_c_canonical_equal_lane_b() {
    let network = network(PNML_CYCLE2);
    let measure_b = lane_b(&network).unwrap();
    let result_c = lane_c(&network, &order_canonical(network.number_places())).unwrap();
    assert_eq!(measure_b, result_c.measure);
    assert_eq!(result_c.strategie, StrategieDownstream::OrderRuntime);
}

/// Invalid π (missing, duplicate, or foreign key): the planner falls back
/// to the canonical order (FoldCanonical) and the cardinality stays exact.
#[test]
fn test_lane_c_order_invalid_fold_canonical() {
    let network = network(PNML_CHOIX);
    let key_missing = vec![Atom(0), Atom(1)];
    let key_double = vec![Atom(0), Atom(0), Atom(2)];
    let key_foreign = vec![Atom(0), Atom(1), Atom(7)];
    for order in [key_missing, key_double, key_foreign] {
        let result = lane_c(&network, &order).unwrap();
        assert_eq!(result.strategie, StrategieDownstream::FoldCanonical);
        assert_eq!(result.measure.cardinal, 3);
    }
}

/// Valid π (reversed order): the planner adopts the runtime order as-is.
#[test]
fn test_lane_c_order_runtime_adopted() {
    let network = network(PNML_CYCLE2);
    let result = lane_c(&network, &order_inverse(network.number_places())).unwrap();
    assert_eq!(result.strategie, StrategieDownstream::OrderRuntime);
    assert_eq!(result.measure.cardinal, 6);
}

/// Network without a place: typed error on lanes B and C.
#[test]
fn test_without_place_rejected() {
    let network = network(PNML_SANS_PLACE);
    assert_eq!(lane_b(&network).unwrap_err(), ErrorTriptych::WithoutPlace);
    assert_eq!(
        lane_c(&network, &[]).unwrap_err(),
        ErrorTriptych::WithoutPlace
    );
}

/// Determinism: two runs give the same measurement.
#[test]
fn test_determinism() {
    let network = network(PNML_CYCLE2);
    assert_eq!(lane_b(&network).unwrap(), lane_b(&network).unwrap());
    let order = order_inverse(network.number_places());
    assert_eq!(
        lane_c(&network, &order).unwrap(),
        lane_c(&network, &order).unwrap()
    );
}
