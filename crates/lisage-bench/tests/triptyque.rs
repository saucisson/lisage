//! Tests du triptyque A/B/C sur modèles PNML (M5-2, énoncés C3/C4) :
//! cardinal identique sur les trois voies (falsification explicite), voie C
//! à ordre runtime (π valide adopté, π invalide rejeté), erreurs typées.

use lisage_bench::pnml::{PnmlDocument, ReseauPetri};
use lisage_bench::triptyque::{ErreurTriptyque, voie_a, voie_b, voie_c};
use lisage_lang::value::Atom;
use lisage_plan::ordre::StrategieAval;

/// Boucle : p0 (2 jetons), t : p0 → p0. Espace atteignable = 1 marquage.
const PNML_BOUCL: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>2</text></initialMarking></place>
  <transition id="t"/>
  <arc id="a1" source="p0" target="t"/>
  <arc id="a2" source="t" target="p0"/>
</net></pnml>"#;

/// Transfert : p0 (1 jeton) → t → p1. Espace atteignable = 2 marquages.
const PNML_TRANSFERT: &str = r#"<pnml><net>
  <place id="p0"><initialMarking><text>1</text></initialMarking></place>
  <place id="p1"/>
  <transition id="t"/>
  <arc id="a1" source="p0" target="t"/>
  <arc id="a2" source="t" target="p1"/>
</net></pnml>"#;

/// Choix : p0 (1 jeton), t1 → p1, t2 → p2. Espace atteignable = 3 marquages.
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

/// Cycle de 3 places, 2 jetons : toutes les compositions de 2 en 3 parts,
/// espace atteignable = 6 marquages.
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

/// Réseau sans aucune place.
const PNML_SANS_PLACE: &str = r#"<pnml><net><transition id="t"/></net></pnml>"#;

/// Parse un modèle PNML de test.
fn reseau(pnml: &str) -> ReseauPetri {
    ReseauPetri::depuis_pnml(&PnmlDocument(pnml.to_string())).unwrap()
}

/// Ordre canonique des clés de places d'un réseau à `n` places.
fn ordre_canonique(n: usize) -> Vec<Atom> {
    (0..n).map(|index| Atom(index as u64)).collect()
}

/// Ordre inversé des clés de places d'un réseau à `n` places.
fn ordre_inverse(n: usize) -> Vec<Atom> {
    (0..n).rev().map(|index| Atom(index as u64)).collect()
}

/// Falsification C3/C4 : cardinal identique sur les voies A, B et C
/// (ordre canonique et ordre inversé), pour chaque modèle jouet.
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
        assert_eq!(mesure_a.cardinal, cardinal_attendu, "voie A sur {pnml}");
        assert_eq!(mesure_b.cardinal, cardinal_attendu, "voie B sur {pnml}");
        assert_eq!(
            mesure_c_canonique.cardinal, cardinal_attendu,
            "voie C canonique sur {pnml}"
        );
        assert_eq!(
            mesure_c_inverse.cardinal, cardinal_attendu,
            "voie C inversé sur {pnml}"
        );
    }
}

/// La voie A est ensembliste explicite (0 nœud de diagramme) ; les voies
/// B/C construisent un DD non vide.
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

/// Voie C à ordre canonique == voie B (même ordre, mêmes mesures).
#[test]
fn test_voie_c_canonique_egale_voie_b() {
    let reseau = reseau(PNML_CYCLE2);
    let mesure_b = voie_b(&reseau).unwrap();
    let resultat_c = voie_c(&reseau, &ordre_canonique(reseau.nombre_places())).unwrap();
    assert_eq!(mesure_b, resultat_c.mesure);
    assert_eq!(resultat_c.strategie, StrategieAval::OrdreRuntime);
}

/// π invalide (clé manquante, en double, étrangère) : le planner se replie
/// sur l'ordre canonique (RepliCanonique) et le cardinal reste exact.
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

/// π valide (ordre inversé) : le planner adopte l'ordre runtime tel quel.
#[test]
fn test_voie_c_ordre_runtime_adopte() {
    let reseau = reseau(PNML_CYCLE2);
    let resultat = voie_c(&reseau, &ordre_inverse(reseau.nombre_places())).unwrap();
    assert_eq!(resultat.strategie, StrategieAval::OrdreRuntime);
    assert_eq!(resultat.mesure.cardinal, 6);
}

/// Réseau sans place : erreur typée sur les voies B et C.
#[test]
fn test_sans_place_rejete() {
    let reseau = reseau(PNML_SANS_PLACE);
    assert_eq!(voie_b(&reseau).unwrap_err(), ErreurTriptyque::SansPlace);
    assert_eq!(
        voie_c(&reseau, &[]).unwrap_err(),
        ErreurTriptyque::SansPlace
    );
}

/// Déterminisme : deux exécutions donnent la même mesure.
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
