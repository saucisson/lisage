//! Tests for the vtree as a first-class order (M5-3, C6): canonical order
//! = flat traversal, left-before-right constraints checked at `Order`
//! construction, valid π adopted, violating π rejected.

use lisage_algebra::vtree::{ErrorVtree, Vtree};
use lisage_lang::Atom;
use lisage_plan::order::{ErrorOrder, Order};

/// Test vtree: (x0, x1) on the left, (x2, x3) on the right.
fn vtree_test() -> Vtree {
    Vtree::node(
        Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
        Vtree::node(Vtree::leaf(Atom(2)), Vtree::leaf(Atom(3))),
    )
}

/// Canonical order of the vtree = flat left-before-right traversal.
#[test]
fn test_order_canonical_vtree() {
    let order = Order::canonical(&vtree_test());
    assert_eq!(order.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// A vtree induces a total order (left-before-right constraint at each
/// internal node): the flat order is the only valid permutation, adopted.
#[test]
fn test_pi_valid_adopted() {
    let vtree = vtree_test();
    let order = Order::new(&vtree, vtree.order_flat()).unwrap();
    assert_eq!(order.sequence(), &[Atom(0), Atom(1), Atom(2), Atom(3)]);
}

/// Another vtree shape induces another canonical order: (x2, (x0, x1))
/// gives the flat order [2, 0, 1], and π = [0, 1, 2] violates the constraint 2 < 0.
#[test]
fn test_other_vtree_other_order() {
    let vtree = Vtree::node(
        Vtree::leaf(Atom(2)),
        Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(1))),
    );
    let order = Order::canonical(&vtree);
    assert_eq!(order.sequence(), &[Atom(2), Atom(0), Atom(1)]);
    let result = Order::new(&vtree, vec![Atom(0), Atom(1), Atom(2)]);
    assert_eq!(
        result.unwrap_err(),
        ErrorOrder::ConstraintViolated {
            before: Atom(2),
            after: Atom(0),
        }
    );
}

/// π placing a right variable before a left variable: vtree constraint
/// violated, rejected with the offending pair.
#[test]
fn test_pi_violating_vtree_rejected() {
    let result = Order::new(&vtree_test(), vec![Atom(2), Atom(0), Atom(1), Atom(3)]);
    assert_eq!(
        result.unwrap_err(),
        ErrorOrder::ConstraintViolated {
            before: Atom(0),
            after: Atom(2),
        }
    );
}

/// π that is not a permutation of the vtree's variables: rejected.
#[test]
fn test_pi_not_one_permutation_rejected() {
    let result = Order::new(&vtree_test(), vec![Atom(0), Atom(1), Atom(2), Atom(9)]);
    assert_eq!(result.unwrap_err(), ErrorOrder::NotOnePermutation);
}

/// Invalid vtree: duplicate variable, typed error.
#[test]
fn test_vtree_variable_in_double_rejected() {
    let vtree = Vtree::node(Vtree::leaf(Atom(0)), Vtree::leaf(Atom(0)));
    assert_eq!(
        vtree.validate().unwrap_err(),
        ErrorVtree::VariableInDouble(Atom(0))
    );
}

/// Generates a random vtree over `number_variables` variables, via
/// deterministic LCG (same constants as the other property tests).
fn vtree_random(graine: &mut u64, number_variables: u64) -> Vtree {
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    fn build(graine: &mut u64, variables: &[Atom]) -> Vtree {
        if variables.len() == 1 {
            return Vtree::leaf(variables[0]);
        }
        let point_cut = (next(graine) % (variables.len() as u64 - 1)) as usize + 1;
        Vtree::node(
            build(graine, &variables[..point_cut]),
            build(graine, &variables[point_cut..]),
        )
    }
    fn next(graine: &mut u64) -> u64 {
        *graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        *graine
    }
    let variables: Vec<Atom> = (0..number_variables).map(Atom).collect();
    build(graine, &variables)
}

/// Checks structurally (by direct tree traversal, independent of
/// `constraints_order`) that `sequence` places every left variable before
/// every right variable at each internal node.
fn verify_left_before_right(vtree: &Vtree, sequence: &[Atom]) {
    if let Vtree::Node { left, right } = vtree {
        for variable_left in left.order_flat() {
            for variable_right in right.order_flat() {
                let position_left = sequence.iter().position(|a| *a == variable_left).unwrap();
                let position_right = sequence.iter().position(|a| *a == variable_right).unwrap();
                assert!(position_left < position_right);
            }
        }
        verify_left_before_right(left, sequence);
        verify_left_before_right(right, sequence);
    }
}

/// Property (LCG-generated cases): for every vtree, the canonical order is
/// a permutation of the variables and satisfies left-before-right at each
/// node, checked by direct tree traversal.
#[test]
fn test_property_order_canonical_satisfies_constraints() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const NUMBER_CAS: u64 = 50;
    const BOUND_VARIABLES: u64 = 8;
    let mut graine = GRAINE_LCG;
    for _ in 0..NUMBER_CAS {
        graine = graine
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let number_variables = 2 + graine % (BOUND_VARIABLES - 1);
        let vtree = vtree_random(&mut graine, number_variables);
        assert!(vtree.validate().is_ok());
        let order = Order::canonical(&vtree);
        let sequence = order.sequence();
        let mut sorted = sequence.to_vec();
        sorted.sort();
        let mut attendues: Vec<Atom> = (0..number_variables).map(Atom).collect();
        attendues.sort();
        assert_eq!(sorted, attendues);
        verify_left_before_right(&vtree, sequence);
    }
}
