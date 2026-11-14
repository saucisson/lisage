//! Tests of the A/B/C triptych on PNML models (M5-2, statements C3/C4):
//! identical cardinal on the three paths (explicit falsification), path C
//! with runtime order (valid π adopted, invalid π rejected), typed errors.

use lisage_bench::pnml::{PnmlDocument, ReseauPetri};
use lisage_bench::triptyque::{ErreurTriptyque, voie_a, voie_b, voie_c};
use lisage_lang::value::Atom;
use lisage_plan::ordre::StrategieAval;

/// Loop: p0 (2 tokens), t : p0 → p0. Reachable space = 1 marking.
const PNML_BOUCL: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <transition id="t"/>
  <arc id="a1" source="p0" target="t"/>
  <arc id="a2" source="t" target="p0"/>
</net></pnml>"#;

/// Transfer: p0 (1 token) → t → p1. Reachable space = 2 markings.
const PNML_TRANSFERT: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>1</text></initialMarking></place>
  <place id="p1"/>
  <transition id="t"/>
  <arc id="a1" source="p0" target="t"/>
  <arc id="a2" source="t" target="p1"/>
</net></pnml>"#;

/// Choice: p0 (1 token), t1 → p1, t2 → p2. Reachable space = 3 markings.
const PNML_CHOIX: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>1</text></initialMarking></place>
  <place id="p1"/>
  <place id="p2"/>
  <transition id="t1"/>
  <transition id="t2"/>
  <arc id="a1" source="p0" target="t1"/>
  <arc id="a2" source="t1" target="p1"/>
  <arc id="a3" source="p0" target="t2"/>
  <arc id="a4" source="t2" target="p2"/>
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
  <arc id="a1" source="p0" target="t1"/>
  <arc id="a2" source="t1" target="p1"/>
  <arc id="a3" source="p1" target="t2"/>
  <arc id="a4" source="t2" target="p2"/>
  <arc id="a5" source="p2" target="t3"/>
  <arc id="a6" source="t3" target="p0"/>
</net></pnml>"#;

/// Net with no place at all.
const PNML_SANS_PLACE: &str = r#"<pnml><net><transition id="t"/></net></pnml>"#;

/// Parses a test PNML model.
fn reseau(pnml: &str) -> ReseauPetri {
    ReseauPetri::depuis_pnml(&PnmlDocument(pnml.to_string())).unwrap()
}

/// Canonical order of the place keys of a net with `n` places.
fn ordre_canonique(n: usize) -> Vec<Atom> {
    (0..n).map(|index| Atom(index as u64)).collect()
}

/// Reversed order of the place keys of a net with `n` places.
fn ordre_inverse(n: usize) -> Vec<Atom> {
    (0..n).rev().map(|index| Atom(index as u64)).collect()
}

/// C3/C4 falsification: identical cardinal on paths A, B and C
/// (canonical order and reversed order), for each toy model.
#[test]
fn test_cardinal_identique_sur_les_trois_voies() {
    let attendus = [
        (PNML_BOUCL, 1),
        (PNML_TRANSFERT, 2),
        (PNML_CHOIX, 3),
        (PNML_CYCLE2, 6),
    ];
    for (pnml, cardinal_attendu) in attendus {
        let reseau = reseau(pnml);
        let mesure_a = voie_a(&reseau);
        let mesure_b = voie_b(&reseau).unwrap();
        let resultat_c_canonique =
            voie_c(&reseau, &ordre_canonique(reseau.nombre_places())).unwrap();
        let resultat_c_inverse = voie_c(&reseau, &ordre_inverse(reseau.nombre_places())).unwrap();
        let mesure_c_canonique = resultat_c_canonique.mesure;
        let mesure_c_inverse = resultat_c_inverse.mesure;
        assert_eq!(mesure_a.cardinal, cardinal_attendu, "path A on {pnml}");
        assert_eq!(mesure_b.cardinal, cardinal_attendu, "path B on {pnml}");
        assert_eq!(
            mesure_c_canonique.cardinal, cardinal_attendu,
            "canonical path C on {pnml}"
        );
        assert_eq!(
            mesure_c_inverse.cardinal, cardinal_attendu,
            "reversed path C on {pnml}"
        );
    }
}

/// Path A is explicit set-theoretic (0 diagram nodes); paths
/// B/C build a non-empty DD.
#[test]
fn test_voie_a_sans_dd_voies_b_c_avec_dd() {
    let reseau = reseau(PNML_CHOIX);
    assert_eq!(voie_a(&reseau).noeuds_vivants, 0);
    assert!(voie_b(&reseau).unwrap().noeuds_vivants > 0);
    assert!(
        voie_c(&reseau, &ordre_inverse(reseau.nombre_places()))
            .unwrap()
            .mesure
            .noeuds_vivants
            > 0
    );
}

/// Path C with canonical order == path B (same order, same measurements).
#[test]
fn test_voie_c_canonique_egale_voie_b() {
    let reseau = reseau(PNML_CYCLE2);
    let mesure_b = voie_b(&reseau).unwrap();
    let resultat_c = voie_c(&reseau, &ordre_canonique(reseau.nombre_places())).unwrap();
    assert_eq!(mesure_b, resultat_c.mesure);
    assert_eq!(resultat_c.strategie, StrategieAval::OrdreRuntime);
}

/// Invalid π (missing, duplicated, or foreign key): the planner falls back
/// to the canonical order (RepliCanonique) and the cardinal stays exact.
#[test]
fn test_voie_c_ordre_invalide_repli_canonique() {
    let reseau = reseau(PNML_CHOIX);
    let cle_manquante = vec![Atom(0), Atom(1)];
    let cle_double = vec![Atom(0), Atom(0), Atom(2)];
    let cle_etrangere = vec![Atom(0), Atom(1), Atom(7)];
    for ordre in [cle_manquante, cle_double, cle_etrangere] {
        let resultat = voie_c(&reseau, &ordre).unwrap();
        assert_eq!(resultat.strategie, StrategieAval::RepliCanonique);
        assert_eq!(resultat.mesure.cardinal, 3);
    }
}

/// Valid π (reversed order): the planner adopts the runtime order as-is.
#[test]
fn test_voie_c_ordre_runtime_adopte() {
    let reseau = reseau(PNML_CYCLE2);
    let resultat = voie_c(&reseau, &ordre_inverse(reseau.nombre_places())).unwrap();
    assert_eq!(resultat.strategie, StrategieAval::OrdreRuntime);
    assert_eq!(resultat.mesure.cardinal, 6);
}

/// Net without a place: typed error on paths B and C.
#[test]
fn test_sans_place_rejete() {
    let reseau = reseau(PNML_SANS_PLACE);
    assert_eq!(voie_b(&reseau).unwrap_err(), ErreurTriptyque::SansPlace);
    assert_eq!(
        voie_c(&reseau, &[]).unwrap_err(),
        ErreurTriptyque::SansPlace
    );
}

/// Determinism: two runs give the same measurement.
#[test]
fn test_determinisme() {
    let reseau = reseau(PNML_CYCLE2);
    assert_eq!(voie_b(&reseau).unwrap(), voie_b(&reseau).unwrap());
    let ordre = ordre_inverse(reseau.nombre_places());
    assert_eq!(
        voie_c(&reseau, &ordre).unwrap(),
        voie_c(&reseau, &ordre).unwrap()
    );
}
