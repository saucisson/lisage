//! Tests for M3-2 (statement C3): reorder = convert — `Conversion<T>`
//! compiled into adjacent transpositions at predictable cost (inversions),
//! and the typed contract extended to encodings via `Kingdom<T, O, E, V>`.

use lisage_lang::Atom;
use lisage_plan::conversion::{Conversion, Encoding, ErrorConversion, Kingdom, MarkedEncoding};
use lisage_plan::order::{DomainAtoms, MarkedOrder, Order, TypeOrder};

/// Mark carrying the canonical order over a domain of atoms.
struct MarkedCanonical;

impl MarkedOrder<DomainAtoms> for MarkedCanonical {
    fn order(descripteur: &DomainAtoms) -> Order<DomainAtoms> {
        Order::canonical(descripteur)
    }
}

/// Mark carrying the reversed order over a domain of atoms.
struct MarkedInverse;

impl MarkedOrder<DomainAtoms> for MarkedInverse {
    fn order(descripteur: &DomainAtoms) -> Order<DomainAtoms> {
        let mut pi = descripteur.keys_canonical();
        pi.reverse();
        Order::new(descripteur, pi).expect("the reversed order is a valid permutation")
    }
}

/// Encoding mark: universe sweep.
struct MarkedSweep;

impl MarkedEncoding for MarkedSweep {
    fn encoding() -> Encoding {
        Encoding::SweepUnivers
    }
}

/// Encoding mark: enumerative compilation.
struct MarkedEnumeration;

impl MarkedEncoding for MarkedEnumeration {
    fn encoding() -> Encoding {
        Encoding::Enumeration
    }
}

/// Number of inversions between two sequences (Kendall tau distance),
/// computed independently of `Conversion`.
fn inversions<T: TypeOrder>(source: &Order<T>, target: &Order<T>) -> usize {
    let sequence = target.sequence();
    let mut count = 0;
    for i in 0..sequence.len() {
        for j in i + 1..sequence.len() {
            if source.position(&sequence[i]) > source.position(&sequence[j]) {
                count += 1;
            }
        }
    }
    count
}

/// M3-2: a conversion between aligned orders is the identity (zero cost).
#[test]
fn test_conversion_identity_cost_null() {
    let domain = DomainAtoms(vec![Atom(0), Atom(1), Atom(2)]);
    let order = Order::canonical(&domain);
    let conversion = Conversion::new(&order, &order.clone()).unwrap();
    assert!(conversion.is_identity());
    assert_eq!(conversion.cost(), 0);
    assert!(conversion.transpositions().is_empty());
}

/// M3-2: reorder = convert — the conversion is compiled into adjacent
/// transpositions whose number is the predictable cost.
#[test]
fn test_conversion_transpositions_adjacent() {
    let domain = DomainAtoms(vec![Atom(0), Atom(1), Atom(2)]);
    let source = Order::canonical(&domain);
    let target = Order::new(&domain, vec![Atom(2), Atom(1), Atom(0)]).unwrap();
    let conversion = Conversion::new(&source, &target).unwrap();
    // Total inversion of 3 elements: 3 inversions, 3 transpositions.
    assert_eq!(conversion.cost(), 3);
    assert_eq!(conversion.transpositions().len(), 3);
    for (i, j) in conversion.transpositions() {
        assert_eq!(*j, i + 1, "adjacent transposition");
    }
    // Applying the transpositions to the target gives back the source.
    let mut sequence = target.sequence().to_vec();
    for (i, j) in conversion.transpositions() {
        sequence.swap(*i, *j);
    }
    assert_eq!(sequence, source.sequence());
}

/// M3-2: two orders over different domains are unconvertible.
#[test]
fn test_conversion_orders_unconvertible() {
    let domain_a = DomainAtoms(vec![Atom(0), Atom(1)]);
    let domain_b = DomainAtoms(vec![Atom(0), Atom(1), Atom(2)]);
    let order_a = Order::canonical(&domain_a);
    let order_b = Order::canonical(&domain_b);
    assert_eq!(
        Conversion::new(&order_a, &order_b),
        Err(ErrorConversion::OrdersUnconvertible)
    );
}

/// M3-2: typed contract extended to encodings — the kingdom exposes its
/// order and its encoding, and `reorder` requires a conversion compatible
/// with the source and target marks.
#[test]
fn test_kingdom_reorder_contract_type() {
    let domain = DomainAtoms(vec![Atom(0), Atom(1), Atom(2)]);
    let source = MarkedCanonical::order(&domain);
    let target = MarkedInverse::order(&domain);
    let conversion = Conversion::new(&source, &target).unwrap();

    let kingdom = Kingdom::<DomainAtoms, MarkedCanonical, MarkedSweep, u32>::new(7);
    assert_eq!(*kingdom.value(), 7);
    assert_eq!(
        Kingdom::<DomainAtoms, MarkedCanonical, MarkedSweep, u32>::encoding(),
        Encoding::SweepUnivers
    );
    assert_eq!(
        Kingdom::<DomainAtoms, MarkedCanonical, MarkedEnumeration, u32>::encoding(),
        Encoding::Enumeration
    );

    // Compatible conversion: the value moves into the target kingdom.
    let converti = kingdom
        .reorder::<MarkedInverse>(&domain, &conversion)
        .unwrap();
    assert_eq!(*converti.value(), 7);
    assert_eq!(conversion.cost(), 3, "predictable cost exposed");

    // Incompatible conversion (wrong source): typed error.
    let mauvaise = Conversion::new(&target, &source).unwrap();
    assert_eq!(
        kingdom.reorder::<MarkedInverse>(&domain, &mauvaise).err(),
        Some(ErrorConversion::KingdomIncompatible)
    );
}

/// Property (deterministic LCG): for all source/target orders over the same
/// domain, the conversion cost equals the number of inversions computed
/// independently, and the transpositions applied to the target give back the
/// source.
#[test]
fn test_property_cost_equal_inversions_and_transpositions_exact() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut graine = GRAINE_LCG;
    let mut next = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _cas in 0..50 {
        let size = 1 + (next() % 8) as usize;
        let domain = DomainAtoms((0..size).map(|i| Atom(i as u64)).collect());
        let source = Order::canonical(&domain);
        // Target: random permutation (deterministic Fisher-Yates).
        let mut pi: Vec<Atom> = (0..size).map(|i| Atom(i as u64)).collect();
        for i in (1..size).rev() {
            let j = (next() as usize) % (i + 1);
            pi.swap(i, j);
        }
        let target = Order::new(&domain, pi).unwrap();

        let conversion = Conversion::new(&source, &target).unwrap();
        assert_eq!(
            conversion.cost(),
            inversions(&source, &target),
            "cost = inversions (Kendall tau)"
        );
        let mut sequence = target.sequence().to_vec();
        for (i, j) in conversion.transpositions() {
            sequence.swap(*i, *j);
        }
        assert_eq!(sequence, source.sequence(), "exact transpositions");
        assert_eq!(conversion.is_identity(), source == target);
    }
}
