//! Tests du parseur PNML et de la conversion vers `lang/` (M5-2) :
//! parsing d'un modèle jouet, marquage initial, sémantique pas-à-pas,
//! espace atteignable calculé par l'interprète de référence (voie A),
//! et erreurs typées.

use std::collections::BTreeSet;

use lisage_bench::pnml::{
    PnmlError, ReseauPetri, espace_atteignable, est_sensibilisee, marquage_depuis_value,
    marquage_initial, pas_semantique, tirer,
};
use lisage_lang::value::Atom;

/// Modèle jouet : une place source avec un jeton, deux transitions
/// vers deux places puits. Espace atteignable = 3 marquages.
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

/// Modèle à deux jetons sur une place, une transition en boucle :
/// p0 (2 jetons), t : p0 → p0. Espace atteignable = 1 marquage
/// (le jeton revient), mais la transition reste sensibilisée.
const PNML_BOUCL: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <transition id="t"/>
  <arc id="a1" source="p0" target="t"/>
  <arc id="a2" source="t" target="p0"/>
</net></pnml>"#;

#[test]
fn test_parse_modele_joue() {
    let reseau = ReseauPetri::depuis_pnml(PNML_JOUET).unwrap();
    assert_eq!(reseau.nb_places(), 3);
    assert_eq!(reseau.nb_transitions(), 2);
    assert_eq!(reseau.arcs.len(), 4);
    assert_eq!(reseau.places[0].identifiant, "p0");
    assert_eq!(reseau.places[0].nom.as_deref(), Some("source"));
    assert_eq!(reseau.places[0].marquage_initial, 1);
    assert_eq!(reseau.places[1].marquage_initial, 0);
    assert_eq!(reseau.transitions[0].nom.as_deref(), Some("aller_a"));
}

#[test]
fn test_marquage_initial_vers_value() {
    let reseau = ReseauPetri::depuis_pnml(PNML_JOUET).unwrap();
    let valeur = marquage_initial(&reseau);
    let marquage = marquage_depuis_value(&valeur).unwrap();
    assert_eq!(marquage.get(&Atom(0)), Some(&Atom(1)));
    assert_eq!(marquage.get(&Atom(1)), Some(&Atom(0)));
    assert_eq!(marquage.get(&Atom(2)), Some(&Atom(0)));
}

#[test]
fn test_sensibilisation_et_tir() {
    let reseau = ReseauPetri::depuis_pnml(PNML_JOUET).unwrap();
    let initial = marquage_depuis_value(&marquage_initial(&reseau)).unwrap();
    assert!(est_sensibilisee(&reseau, "t1", &initial));
    assert!(est_sensibilisee(&reseau, "t2", &initial));
    let apres_t1 = tirer(&reseau, "t1", &initial);
    assert_eq!(apres_t1.get(&Atom(0)), Some(&Atom(0)));
    assert_eq!(apres_t1.get(&Atom(1)), Some(&Atom(1)));
    assert!(!est_sensibilisee(&reseau, "t1", &apres_t1));
    assert!(!est_sensibilisee(&reseau, "t2", &apres_t1));
}

#[test]
fn test_espace_atteignable_voie_a() {
    let reseau = ReseauPetri::depuis_pnml(PNML_JOUET).unwrap();
    let espace = espace_atteignable(&reseau);
    // 3 marquages : initial, après t1, après t2.
    assert_eq!(espace.len(), 3);
    let initial = marquage_initial(&reseau);
    assert!(espace.contains(&initial));
}

#[test]
fn test_pas_semantique_depuis_ensemble() {
    let reseau = ReseauPetri::depuis_pnml(PNML_JOUET).unwrap();
    let initial = BTreeSet::from([marquage_initial(&reseau)]);
    let successeurs = pas_semantique(&reseau, &initial);
    assert_eq!(successeurs.len(), 2);
}

#[test]
fn test_boucle_espace_stable() {
    let reseau = ReseauPetri::depuis_pnml(PNML_BOUCL).unwrap();
    let espace = espace_atteignable(&reseau);
    assert_eq!(espace.len(), 1);
}

#[test]
fn test_erreur_xml_mal_forme() {
    let resultat = ReseauPetri::depuis_pnml("<pnml><net><place id=\"p0\">");
    assert!(matches!(resultat, Err(PnmlError::XmlMalForme { .. })));
}

#[test]
fn test_erreur_attribut_manquant() {
    let resultat = ReseauPetri::depuis_pnml("<pnml><net><place/></net></pnml>");
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
    let document = "<pnml><net><place id=\"p0\"/><arc source=\"p0\" target=\"t9\"/></net></pnml>";
    let resultat = ReseauPetri::depuis_pnml(document);
    assert!(matches!(resultat, Err(PnmlError::ArcVersInconnu { .. })));
}

#[test]
fn test_erreur_nombre_invalide() {
    let document = "<pnml><net><place id=\"p0\"><initialMarking><text>abc</text></initialMarking></place></net></pnml>";
    let resultat = ReseauPetri::depuis_pnml(document);
    assert!(matches!(
        resultat,
        Err(PnmlError::NombreInvalide {
            element: "initialMarking",
            ..
        })
    ));
}
