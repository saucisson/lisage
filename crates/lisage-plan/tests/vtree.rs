//! Tests of the vtree as a first-class order (M5-3, C6): canonical
//! order = flat traversal, left-before-right constraints checked
//! at `Order` construction, valid π adopted, violating π rejected.

use lisage_algebra::vtree::{ErreurVtree, Vtree};
use lisage_lang::Atom;
use lisage_plan::ordre::{ErreurOrdre, Order};

/// Test vtree: (x0, x1) on the left, (x2, x3) on the right.
fn vtree_test() -> Vtree {
    Vtree::noeud(
        Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
        Vtree::noeud(Vtree::feuille(Atom(2)), Vtree::feuille(Atom(3))),
    )
}

/// Canonical order of the vtree = flat left-before-right traversal.
#[test]
fn test_ordre_canonique_vtree() {
    let ordre = Order::canonique(&vtree_test());
    assert_eq!(ordre.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// A vtree induces a total order (left-before-right constraint at each
/// internal node): the flat order is the only valid permutation, adopted.
#[test]
fn test_pi_valide_adopte() {
    let vtree = vtree_test();
    let ordre = Order::nouveau(&vtree, vtree.ordre_plat()).unwrap();
    assert_eq!(ordre.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// Another vtree shape induces another canonical order: (x2, (x0, x1))
/// gives flat order [2, 0, 1], and π = [0, 1, 2] violates constraint 2 < 0.
#[test]
fn test_autre_vtree_autre_ordre() {
    let vtree = Vtree::noeud(
        Vtree::feuille(Atom(2)),
        Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
    );
    let ordre = Order::canonique(&vtree);
    assert_eq!(ordre.sequence(), &[Atom(2), Atom(0), Atom(1)]);
    let resultat = Order::nouveau(&vtree, vec![Atom(0), Atom(1), Atom(2)]);
    assert_eq!(
        resultat.unwrap_err(),
        ErreurOrdre::ContrainteViolee {
            avant: Atom(2),
            apres: Atom(0),
        }
    );
}

/// π placing a right variable before a left variable:
/// vtree constraint violated, rejected with the offending pair.
#[test]
fn test_pi_violant_vtree_rejete() {
    let resultat = Order::nouveau(&vtree_test(), vec![Atom(2), Atom(0), Atom(1), Atom(3)]);
    assert_eq!(
        resultat.unwrap_err(),
        ErreurOrdre::ContrainteViolee {
            avant: Atom(0),
            apres: Atom(2),
        }
    );
}

/// π that is not a permutation of the vtree's variables: rejected.
#[test]
fn test_pi_pas_une_permutation_rejete() {
    let resultat = Order::nouveau(&vtree_test(), vec![Atom(0), Atom(1), Atom(2), Atom(9)]);
    assert_eq!(resultat.unwrap_err(), ErreurOrdre::PasUnePermutation);
}

/// Invalid vtree: duplicate variable, typed error.
#[test]
fn test_vtree_variable_en_double_rejete() {
    let vtree = Vtree::noeud(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(0)));
    assert_eq!(
        vtree.valider().unwrap_err(),
        ErreurVtree::VariableEnDouble(Atom(0))
    );
}

/// Generates a random vtree over `nombre_variables` variables, by
/// deterministic LCG (same constants as the other property tests).
fn vtree_aleatoire(graine: &mut u64, nombre_variables: u64) -> Vtree {
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    fn construire(graine: &mut u64, variables: &[Atom]) -> Vtree {
        if variables.len() == 1 {
            return Vtree::feuille(variables[0]);
        }
        let point_coupe = (suivant(graine) % (variables.len() as u64 - 1)) as usize + 1;
        Vtree::noeud(
            construire(graine, &variables[..point_coupe]),
            construire(graine, &variables[point_coupe..]),
        )
    }
    fn suivant(graine: &mut u64) -> u64 {
        *graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        *graine
    }
    let variables: Vec<Atom> = (0..nombre_variables).map(Atom).collect();
    construire(graine, &variables)
}

/// Structurally checks (by direct tree traversal, independent of
/// `contraintes_ordre`) that `sequence` places every left variable before
/// every right variable at each internal node.
fn verifier_gauche_avant_droite(vtree: &Vtree, sequence: &[Atom]) {
    if let Vtree::Noeud { gauche, droite } = vtree {
        for variable_gauche in gauche.ordre_plat() {
            for variable_droite in droite.ordre_plat() {
                let position_gauche = sequence.iter().position(|a| *a == variable_gauche).unwrap();
                let position_droite = sequence.iter().position(|a| *a == variable_droite).unwrap();
                assert!(position_gauche < position_droite);
            }
        }
        verifier_gauche_avant_droite(gauche, sequence);
        verifier_gauche_avant_droite(droite, sequence);
    }
}

/// Property (LCG-generated cases): for any vtree, the canonical order is
/// a permutation of the variables and satisfies left-before-right at each
/// node, verified by direct tree traversal.
#[test]
fn test_propriete_ordre_canonique_satisfait_contraintes() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const NOMBRE_CAS: u64 = 50;
    const BORNE_VARIABLES: u64 = 8;
    let mut graine = GRAINE_LCG;
    for _ in 0..NOMBRE_CAS {
        graine = graine
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let nombre_variables = 2 + graine % (BORNE_VARIABLES - 1);
        let vtree = vtree_aleatoire(&mut graine, nombre_variables);
        assert!(vtree.valider().is_ok());
        let ordre = Order::canonique(&vtree);
        let sequence = ordre.sequence();
        let mut triee = sequence.to_vec();
        triee.sort();
        let mut attendues: Vec<Atom> = (0..nombre_variables).map(Atom).collect();
        attendues.sort();
        assert_eq!(triee, attendues);
        verifier_gauche_avant_droite(&vtree, sequence);
    }
}
