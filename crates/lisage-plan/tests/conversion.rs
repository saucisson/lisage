//! M3-2 tests (claim C3): reorder = convert — `Conversion<T>`
//! compiled into adjacent transpositions at predictable cost (inversions),
//! and typed contract extended to encodings via `Kingdom<T, O, E, V>`.

use lisage_lang::Atom;
use lisage_plan::conversion::{Conversion, Encoding, EncodingMark, ErrorConversion, Kingdom};
use lisage_plan::order::{AtomDomain, Order, OrderMark, TypeOrder};

/// Mark carrying the canonical order over an atom domain.
struct CanonicalMark;

impl OrderMark<AtomDomain> for CanonicalMark {
    fn order(descriptor: &AtomDomain) -> Order<AtomDomain> {
        Order::canonical(descriptor)
    }
}

/// Mark carrying the reversed order over an atom domain.
struct MarkInverse;

impl OrderMark<AtomDomain> for MarkInverse {
    fn order(descriptor: &AtomDomain) -> Order<AtomDomain> {
        let mut pi = descriptor.keys_canonical();
        pi.reverse();
        Order::new(descriptor, pi).expect("the reversed order is a valid permutation")
    }
}

/// Encoding mark: universe sweep.
struct SweepMark;

impl EncodingMark for SweepMark {
    fn encoding() -> Encoding {
        Encoding::UniverseSweep
    }
}

/// Encoding mark: enumerative compilation.
struct EnumerationMark;

impl EncodingMark for EnumerationMark {
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
fn test_conversion_identity_cost_zero() {
    let domain = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
    let order = Order::canonical(&domain);
    let conversion = Conversion::new(&order, &order.clone()).unwrap();
    assert!(conversion.is_identity());
    assert_eq!(conversion.cost(), 0);
    assert!(conversion.transpositions().is_empty());
}

/// M3-2: reorder = convert — the conversion is compiled into
/// adjacent transpositions whose count is the predictable cost.
#[test]
fn test_conversion_transpositions_adjacentes() {
    let domain = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
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

/// M3-2: two orders of different domains are inconvertible.
#[test]
fn test_conversion_orders_inconvertibles() {
    let domain_a = AtomDomain(vec![Atom(0), Atom(1)]);
    let domain_b = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
    let order_a = Order::canonical(&domain_a);
    let order_b = Order::canonical(&domain_b);
    assert_eq!(
        Conversion::new(&order_a, &order_b),
        Err(ErrorConversion::InconvertibleOrders)
    );
}

/// M3-2: typed contract extended to encodings — the realm exposes its order
/// and its encoding, and `reorder` requires a conversion compatible with
/// the source and target marks.
#[test]
fn test_kingdom_reorder_contract_type() {
    let domain = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
    let source = CanonicalMark::order(&domain);
    let target = MarkInverse::order(&domain);
    let conversion = Conversion::new(&source, &target).unwrap();

    let kingdom = Kingdom::<AtomDomain, CanonicalMark, SweepMark, u32>::new(7);
    assert_eq!(*kingdom.value(), 7);
    assert_eq!(
        Kingdom::<AtomDomain, CanonicalMark, SweepMark, u32>::encoding(),
        Encoding::UniverseSweep
    );
    assert_eq!(
        Kingdom::<AtomDomain, CanonicalMark, EnumerationMark, u32>::encoding(),
        Encoding::Enumeration
    );

    // Compatible conversion: the value moves into the target realm.
    let converti = kingdom
        .reorder::<MarkInverse>(&domain, &conversion)
        .unwrap();
    assert_eq!(*converti.value(), 7);
    assert_eq!(conversion.cost(), 3, "predictable cost exposed");

    // Incompatible conversion (wrong source): typed error.
    let mauvaise = Conversion::new(&target, &source).unwrap();
    assert_eq!(
        kingdom.reorder::<MarkInverse>(&domain, &mauvaise).err(),
        Some(ErrorConversion::KingdomIncompatible)
    );
}

/// Property (deterministic LCG): for any source/target orders over the
/// same domain, the conversion cost equals the independently computed
/// inversion count, and the transpositions applied to the target give back
/// the source.
#[test]
fn test_property_cost_equals_inversions_and_transpositions_exact() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const LCG_MULTIPLIER: u64 = 6364136223846793005;
    const LCG_INCREMENT: u64 = 1442695040888963407;

    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        seed >> 33
    };

    for _cas in 0..50 {
        let size = 1 + (next() % 8) as usize;
        let domain = AtomDomain((0..size).map(|i| Atom(i as u64)).collect());
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
