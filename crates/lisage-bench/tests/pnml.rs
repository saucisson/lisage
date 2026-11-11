//! Tests du parseur PNML et de la conversion vers `lang/` (M5-2) :
//! parsing d'un modèle jouet, marquage initial, sémantique pas-à-pas,
//! espace atteignable calculé par l'interprète de référence (voie A),
//! erreurs typées, et propriétés (ADR-0012) sur réseaux générés.

use std::collections::BTreeSet;

use lisage_bench::pnml::{
    PlaceId, PnmlDocument, PnmlError, ReseauPetri, TransitionId, espace_atteignable,
    est_sensibilisee, marquage_depuis_value, marquage_initial, pas_semantique, tirer,
};
use lisage_lang::value::Atom;
use proptest::prelude::*;

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
/// p0 (2 jetons), t : p0 → p0. Espace atteignable = 1 marquage.
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
    // 3 marquages : initial, après t1, après t2.
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
    // Deux transitions reliées : arc mal orienté, rejeté au parsing.
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
    // Racine autre que <pnml>.
    let resultat = ReseauPetri::depuis_pnml(&PnmlDocument("<foo/>".to_string()));
    assert!(matches!(
        resultat,
        Err(PnmlError::ElementInattendu {
            attendu: "pnml",
            ..
        })
    ));
    // <pnml> sans <net>.
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
// Propriétés (ADR-0012) : réseaux générés, invariants sémantiques
// ---------------------------------------------------------------------------

/// Génère un document PNML aléatoire : `nombre_places` places (marquage
/// initial 0..=2), `nombre_transitions` transitions, chaque transition
/// reliée à une place entrante et une place sortante (éventuellement
/// la même).
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
    /// L'espace atteignable est un point fixe : aucun successeur hors de
    /// l'espace, et il contient le marquage initial.
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

    /// Tirer une transition sensibilisée préserve le nombre total de
    /// jetons quand chaque place entrante a aussi un arc sortant
    /// (réseau en boucle) — invariant du tir.
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
                // Le marquage résultant a une entrée par place.
                prop_assert_eq!(suivant.len(), nombre_places);
                // Invariant documenté : le tir déplace un jeton (poids 1
                // en entrée et en sortie), le total est préservé.
                let total_apres: u64 = suivant.values().map(|a| a.0).sum();
                prop_assert_eq!(total_apres, total_avant);
            }
        }
    }
}
