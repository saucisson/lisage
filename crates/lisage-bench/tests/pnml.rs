//! Tests of the PNML parser and the conversion to `lang/` (M5-2):
//! parsing of a toy model, initial marking, step semantics,
//! reachable space computed by the reference interpreter (path A),
//! typed errors, and properties (ADR-0012) on generated nets.

use std::collections::BTreeSet;

use lisage_bench::pnml::{
    PlaceId, PnmlDocument, PnmlError, ReseauPetri, TransitionId, espace_atteignable,
    est_sensibilisee, marquage_depuis_value, marquage_initial, pas_semantique, tirer,
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
    let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    assert_eq!(reseau.nombre_places(), 3);
    assert_eq!(reseau.nombre_transitions(), 2);
    assert_eq!(reseau.arcs.len(), 4);
    assert_eq!(reseau.places[0].identifiant, PlaceId("p0".to_string()));
    assert_eq!(reseau.places[0].nom.as_deref(), Some("source"));
    assert_eq!(reseau.places[0].marquage_initial, 1);
    assert_eq!(reseau.places[1].marquage_initial, 0);
    assert_eq!(
        reseau.transitions[0].identifiant,
        TransitionId("t1".to_string())
    );
    assert_eq!(reseau.transitions[0].nom.as_deref(), Some("aller_a"));
}

#[test]
fn test_marquage_initial_vers_value() {
    let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let valeur = marquage_initial(&reseau);
    let marquage = marquage_depuis_value(&valeur).unwrap();
    assert_eq!(marquage.get(&Atom(0)), Some(&Atom(1)));
    assert_eq!(marquage.get(&Atom(1)), Some(&Atom(0)));
    assert_eq!(marquage.get(&Atom(2)), Some(&Atom(0)));
}

#[test]
fn test_sensibilisation_et_tir() {
    let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let initial = marquage_depuis_value(&marquage_initial(&reseau)).unwrap();
    let t1 = TransitionId("t1".to_string());
    let t2 = TransitionId("t2".to_string());
    assert!(est_sensibilisee(&reseau, &t1, &initial));
    assert!(est_sensibilisee(&reseau, &t2, &initial));
    let apres_t1 = tirer(&reseau, &t1, &initial);
    assert_eq!(apres_t1.get(&Atom(0)), Some(&Atom(0)));
    assert_eq!(apres_t1.get(&Atom(1)), Some(&Atom(1)));
    assert!(!est_sensibilisee(&reseau, &t1, &apres_t1));
    assert!(!est_sensibilisee(&reseau, &t2, &apres_t1));
}

#[test]
fn test_espace_atteignable_voie_a() {
    let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let espace = espace_atteignable(&reseau);
    // 3 markings: initial, after t1, after t2.
    assert_eq!(espace.len(), 3);
    let initial = marquage_initial(&reseau);
    assert!(espace.contains(&initial));
}

#[test]
fn test_pas_semantique_depuis_ensemble() {
    let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(PNML_JOUET.to_string())).unwrap();
    let initial = BTreeSet::from([marquage_initial(&reseau)]);
    let successeurs = pas_semantique(&reseau, &initial);
    assert_eq!(successeurs.len(), 2);
}

#[test]
fn test_boucle_espace_stable() {
    let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(PNML_BOUCL.to_string())).unwrap();
    let espace = espace_atteignable(&reseau);
    assert_eq!(espace.len(), 1);
}

#[test]
fn test_erreur_xml_mal_forme() {
    let resultat =
        ReseauPetri::depuis_pnml(&PnmlDocument("<pnml><net><place id=\"p0\">".to_string()));
    assert!(matches!(resultat, Err(PnmlError::XmlMalForme { .. })));
}

#[test]
fn test_erreur_attribut_manquant() {
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument(
        "<pnml><net><place/></net></pnml>".to_string(),
    ));
    assert!(matches!(
        resultat,
        Err(PnmlError::AttributManquant {
            element: "place",
            attribut: "id"
        })
    ));
}

#[test]
fn test_erreur_arc_vers_inconnu() {
    let document = "<pnml><net><place id=\"p0\"/><transition id=\"t1\"/><arc source=\"p0\" target=\"t9\"/></net></pnml>";
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(resultat, Err(PnmlError::ArcVersInconnu { .. })));
}

#[test]
fn test_erreur_arc_mal_oriente() {
    // Two linked transitions: badly oriented arc, rejected at parsing.
    let document = "<pnml><net><transition id=\"t1\"/><transition id=\"t2\"/><arc source=\"t1\" target=\"t2\"/></net></pnml>";
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(resultat, Err(PnmlError::ArcMalOriente { .. })));
}

#[test]
fn test_erreur_poids_non_supporte() {
    let document = "<pnml><net><place id=\"p0\"/><transition id=\"t1\"/><arc source=\"p0\" target=\"t1\" weight=\"2\"/></net></pnml>";
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(resultat, Err(PnmlError::PoidsNonSupporte { .. })));
}

#[test]
fn test_erreur_element_inattendu() {
    // Root other than <pnml>.
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument("<foo/>".to_string()));
    assert!(matches!(
        resultat,
        Err(PnmlError::ElementInattendu {
            attendu: "pnml",
            ..
        })
    ));
    // <pnml> without <net>.
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument("<pnml/>".to_string()));
    assert!(matches!(
        resultat,
        Err(PnmlError::ElementInattendu { attendu: "net", .. })
    ));
}

#[test]
fn test_erreur_nombre_invalide() {
    let document = "<pnml><net><place id=\"p0\"><initialMarking><text>abc</text></initialMarking></place></net></pnml>";
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument(document.to_string()));
    assert!(matches!(
        resultat,
        Err(PnmlError::NombreInvalide {
            element: "initialMarking",
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// Properties (ADR-0012): generated nets, semantic invariants
// ---------------------------------------------------------------------------

/// Generates a random PNML document: `nombre_places` places (initial
/// marking 0..=2), `nombre_transitions` transitions, each transition
/// linked to an input place and an output place (possibly
/// the same).
fn reseau_aleatoire(
    nombre_places: usize,
    nombre_transitions: usize,
    marquages: Vec<u64>,
) -> String {
    let mut document = String::from("<pnml><net>");
    for (index, marquage) in marquages.iter().enumerate() {
        document.push_str(&format!(
            "<place id=\"p{index}\"><initialMarking><text>{marquage}</text></initialMarking></place>"
        ));
    }
    for index in 0..nombre_transitions {
        document.push_str(&format!("<transition id=\"t{index}\"/>"));
    }
    for index in 0..nombre_transitions {
        let entree = index % nombre_places.max(1);
        let sortie = (index + 1) % nombre_places.max(1);
        document.push_str(&format!(
            "<arc source=\"p{entree}\" target=\"t{index}\"/><arc source=\"t{index}\" target=\"p{sortie}\"/>"
        ));
    }
    document.push_str("</net></pnml>");
    document
}

proptest! {
    /// The reachable space is a fixpoint: no successor outside the
    /// space, and it contains the initial marking.
    #[test]
    fn propriete_espace_atteignable_point_fixe(
        nombre_places in 1usize..=4,
        nombre_transitions in 1usize..=4,
        marquages in proptest::collection::vec(0u64..=2, 1..=4),
    ) {
        let marquages = {
            let mut m = marquages;
            m.resize(nombre_places, 0);
            m.truncate(nombre_places);
            m
        };
        let document = reseau_aleatoire(nombre_places, nombre_transitions, marquages);
        let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(document.to_string())).unwrap();
        let espace = espace_atteignable(&reseau);
        prop_assert!(espace.contains(&marquage_initial(&reseau)));
        let successeurs = pas_semantique(&reseau, &espace);
        prop_assert!(successeurs.is_subset(&espace));
    }

    /// Firing an enabled transition preserves the total number of
    /// tokens when each input place also has an outgoing arc
    /// (loop net) — firing invariant.
    #[test]
    fn propriete_tir_sensibilise_produit_successeur_valide(
        nombre_places in 1usize..=4,
        nombre_transitions in 1usize..=4,
        marquages in proptest::collection::vec(1u64..=3, 1..=4),
    ) {
        let marquages = {
            let mut m = marquages;
            m.resize(nombre_places, 1);
            m.truncate(nombre_places);
            m
        };
        let document = reseau_aleatoire(nombre_places, nombre_transitions, marquages);
        let reseau = ReseauPetri::depuis_pnml(&PnmlDocument(document.to_string())).unwrap();
        let initial = marquage_depuis_value(&marquage_initial(&reseau)).unwrap();
        let total_avant: u64 = initial.values().map(|a| a.0).sum();
        for transition in &reseau.transitions {
            if est_sensibilisee(&reseau, &transition.identifiant, &initial) {
                let suivant = tirer(&reseau, &transition.identifiant, &initial);
                // The resulting marking has one entry per place.
                prop_assert_eq!(suivant.len(), nombre_places);
                // Documented invariant: firing moves one token (weight 1
                // in input and in output), the total is preserved.
                let total_apres: u64 = suivant.values().map(|a| a.0).sum();
                prop_assert_eq!(total_apres, total_avant);
            }
        }
    }
}
