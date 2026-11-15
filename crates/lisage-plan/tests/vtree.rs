//! Tests of the vtree as a first-class order (M5-3, C6): canonical
//! order = flat traversal, left-before-right constraints checked
//! at `Order` construction, valid π adopted, violating π rejected.

use lisage_algebra::vtree::{ErreurVtree, Vtree};
use lisage_lang::Atom;
use lisage_plan::order::{ErreurOrder, Order};

/// Test vtree: (x0, x1) on the left, (x2, x3) on the right.
fn vtree_test() -> Vtree {
    Vtree::node(
        Vtree::node(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
        Vtree::node(Vtree::feuille(Atom(2)), Vtree::feuille(Atom(3))),
    )
}

/// Canonical order of the vtree = flat left-before-right traversal.
#[test]
fn test_order_canonique_vtree() {
    let order = Order::canonique(&vtree_test());
    assert_eq!(order.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// A vtree induces a total order (left-before-right constraint at each
/// internal node): the flat order is the only valid permutation, adopted.
#[test]
fn test_pi_valide_adopte() {
    let vtree = vtree_test();
    let order = Order::new(&vtree, vtree.order_plat()).unwrap();
    assert_eq!(order.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// Another vtree shape induces another canonical order: (x2, (x0, x1))
/// gives flat order [2, 0, 1], and π = [0, 1, 2] violates constraint 2 < 0.
#[test]
fn test_other_vtree_other_order() {
    let vtree = Vtree::node(
        Vtree::feuille(Atom(2)),
        Vtree::node(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(1))),
    );
    let order = Order::canonique(&vtree);
    assert_eq!(order.sequence(), &[Atom(2), Atom(0), Atom(1)]);
    let result = Order::new(&vtree, vec![Atom(0), Atom(1), Atom(2)]);
    assert_eq!(
        result.unwrap_err(),
        ErreurOrder::ContrainteViolee {
            avant: Atom(2),
            apres: Atom(0),
        }
    );
}

/// π placing a right variable before a left variable:
/// vtree constraint violated, rejected with the offending pair.
#[test]
fn test_pi_violant_vtree_rejete() {
    let result = Order::new(&vtree_test(), vec![Atom(2), Atom(0), Atom(1), Atom(3)]);
    assert_eq!(
        result.unwrap_err(),
        ErreurOrder::ContrainteViolee {
            avant: Atom(0),
            apres: Atom(2),
        }
    );
}

/// π that is not a permutation of the vtree's variables: rejected.
#[test]
fn test_pi_pas_une_permutation_rejete() {
    let result = Order::new(&vtree_test(), vec![Atom(0), Atom(1), Atom(2), Atom(9)]);
    assert_eq!(result.unwrap_err(), ErreurOrder::PasUnePermutation);
}

/// Invalid vtree: duplicate variable, typed error.
#[test]
fn test_vtree_variable_en_double_rejete() {
    let vtree = Vtree::node(Vtree::feuille(Atom(0)), Vtree::feuille(Atom(0)));
    assert_eq!(
        vtree.valider().unwrap_err(),
        ErreurVtree::VariableEnDouble(Atom(0))
    );
}

/// Generates a random vtree over `number_variables` variables, by
/// deterministic LCG (same constants as the other property tests).
fn vtree_aleatoire(seed: &mut u64, number_variables: u64) -> Vtree {
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    fn build(seed: &mut u64, variables: &[Atom]) -> Vtree {
        if variables.len() == 1 {
            return Vtree::feuille(variables[0]);
        }
        let point_coupe = (next(seed) % (variables.len() as u64 - 1)) as usize + 1;
        Vtree::node(
            build(seed, &variables[..point_coupe]),
            build(seed, &variables[point_coupe..]),
        )
    }
    fn next(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        *seed
    }
    let variables: Vec<Atom> = (0..number_variables).map(Atom).collect();
    build(seed, &variables)
}

/// Structurally checks (by direct tree traversal, independent of
/// `contraintes_order`) that `sequence` places every left variable before
/// every right variable at each internal node.
fn verifier_gauche_avant_droite(vtree: &Vtree, sequence: &[Atom]) {
    if let Vtree::Node { gauche, droite } = vtree {
        for variable_gauche in gauche.order_plat() {
            for variable_droite in droite.order_plat() {
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
fn test_propriete_order_canonique_satisfait_contraintes() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const NUMBER_CAS: u64 = 50;
    const BOUND_VARIABLES: u64 = 8;
    let mut seed = SEED_LCG;
    for _ in 0..NUMBER_CAS {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let number_variables = 2 + seed % (BOUND_VARIABLES - 1);
        let vtree = vtree_aleatoire(&mut seed, number_variables);
        assert!(vtree.valider().is_ok());
        let order = Order::canonique(&vtree);
        let sequence = order.sequence();
        let mut triee = sequence.to_vec();
        triee.sort();
        let mut expected_f: Vec<Atom> = (0..number_variables).map(Atom).collect();
        expected_f.sort();
        assert_eq!(triee, expected_f);
        verifier_gauche_avant_droite(&vtree, sequence);
    }
}
