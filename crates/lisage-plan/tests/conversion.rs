//! M3-2 tests (claim C3): reorder = convert — `Conversion<T>`
//! compiled into adjacent transpositions at predictable cost (inversions),
//! and typed contract extended to encodings via `Royaume<T, O, E, V>`.

use lisage_lang::Atom;
use lisage_plan::conversion::{Conversion, Encodage, ErrorConversion, MarqueEncodage, Royaume};
use lisage_plan::order::{DomainAtomes, MarqueOrder, Order, TypeOrder};

/// Mark carrying the canonical order over an atom domain.
struct MarqueCanonique;

impl MarqueOrder<DomainAtomes> for MarqueCanonique {
    fn order(descripteur: &DomainAtomes) -> Order<DomainAtomes> {
        Order::canonique(descripteur)
    }
}

/// Mark carrying the reversed order over an atom domain.
struct MarqueInverse;

impl MarqueOrder<DomainAtomes> for MarqueInverse {
    fn order(descripteur: &DomainAtomes) -> Order<DomainAtomes> {
        let mut pi = descripteur.keys_canoniques();
        pi.reverse();
        Order::new(descripteur, pi).expect("the reversed order is a valid permutation")
    }
}

/// Encoding mark: universe sweep.
struct MarqueBalayage;

impl MarqueEncodage for MarqueBalayage {
    fn encodage() -> Encodage {
        Encodage::BalayageUnivers
    }
}

/// Encoding mark: enumerative compilation.
struct MarqueEnumeration;

impl MarqueEncodage for MarqueEnumeration {
    fn encodage() -> Encodage {
        Encodage::Enumeration
    }
}

/// Number of inversions between two sequences (Kendall tau distance),
/// computed independently of `Conversion`.
fn inversions<T: TypeOrder>(source: &Order<T>, cible: &Order<T>) -> usize {
    let sequence = cible.sequence();
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
fn test_conversion_identite_cout_nul() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let order = Order::canonique(&domain);
    let conversion = Conversion::new(&order, &order.clone()).unwrap();
    assert!(conversion.est_identite());
    assert_eq!(conversion.cout(), 0);
    assert!(conversion.transpositions().is_empty());
}

/// M3-2: reorder = convert — the conversion is compiled into
/// adjacent transpositions whose count is the predictable cost.
#[test]
fn test_conversion_transpositions_adjacentes() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let source = Order::canonique(&domain);
    let cible = Order::new(&domain, vec![Atom(2), Atom(1), Atom(0)]).unwrap();
    let conversion = Conversion::new(&source, &cible).unwrap();
    // Total inversion of 3 elements: 3 inversions, 3 transpositions.
    assert_eq!(conversion.cout(), 3);
    assert_eq!(conversion.transpositions().len(), 3);
    for (i, j) in conversion.transpositions() {
        assert_eq!(*j, i + 1, "adjacent transposition");
    }
    // Applying the transpositions to the target gives back the source.
    let mut sequence = cible.sequence().to_vec();
    for (i, j) in conversion.transpositions() {
        sequence.swap(*i, *j);
    }
    assert_eq!(sequence, source.sequence());
}

/// M3-2: two orders of different domains are inconvertible.
#[test]
fn test_conversion_orders_inconvertibles() {
    let domain_a = DomainAtomes(vec![Atom(0), Atom(1)]);
    let domain_b = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let order_a = Order::canonique(&domain_a);
    let order_b = Order::canonique(&domain_b);
    assert_eq!(
        Conversion::new(&order_a, &order_b),
        Err(ErrorConversion::OrdersInconvertibles)
    );
}

/// M3-2: typed contract extended to encodings — the realm exposes its order
/// and its encoding, and `reordonner` requires a conversion compatible with
/// the source and target marks.
#[test]
fn test_royaume_reordonner_contrat_type() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let source = MarqueCanonique::order(&domain);
    let cible = MarqueInverse::order(&domain);
    let conversion = Conversion::new(&source, &cible).unwrap();

    let royaume = Royaume::<DomainAtomes, MarqueCanonique, MarqueBalayage, u32>::new(7);
    assert_eq!(*royaume.value(), 7);
    assert_eq!(
        Royaume::<DomainAtomes, MarqueCanonique, MarqueBalayage, u32>::encodage(),
        Encodage::BalayageUnivers
    );
    assert_eq!(
        Royaume::<DomainAtomes, MarqueCanonique, MarqueEnumeration, u32>::encodage(),
        Encodage::Enumeration
    );

    // Compatible conversion: the value moves into the target realm.
    let converti = royaume
        .reordonner::<MarqueInverse>(&domain, &conversion)
        .unwrap();
    assert_eq!(*converti.value(), 7);
    assert_eq!(conversion.cout(), 3, "predictable cost exposed");

    // Incompatible conversion (wrong source): typed error.
    let mauvaise = Conversion::new(&cible, &source).unwrap();
    assert_eq!(
        royaume
            .reordonner::<MarqueInverse>(&domain, &mauvaise)
            .err(),
        Some(ErrorConversion::RoyaumeIncompatible)
    );
}

/// Property (deterministic LCG): for any source/target orders over the
/// same domain, the conversion cost equals the independently computed
/// inversion count, and the transpositions applied to the target give back
/// the source.
#[test]
fn test_propriete_cout_egal_inversions_et_transpositions_exactes() {
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed >> 33
    };

    for _cas in 0..50 {
        let size = 1 + (next() % 8) as usize;
        let domain = DomainAtomes((0..size).map(|i| Atom(i as u64)).collect());
        let source = Order::canonique(&domain);
        // Target: random permutation (deterministic Fisher-Yates).
        let mut pi: Vec<Atom> = (0..size).map(|i| Atom(i as u64)).collect();
        for i in (1..size).rev() {
            let j = (next() as usize) % (i + 1);
            pi.swap(i, j);
        }
        let cible = Order::new(&domain, pi).unwrap();

        let conversion = Conversion::new(&source, &cible).unwrap();
        assert_eq!(
            conversion.cout(),
            inversions(&source, &cible),
            "cost = inversions (Kendall tau)"
        );
        let mut sequence = cible.sequence().to_vec();
        for (i, j) in conversion.transpositions() {
            sequence.swap(*i, *j);
        }
        assert_eq!(sequence, source.sequence(), "exact transpositions");
        assert_eq!(conversion.est_identite(), source == cible);
    }
}
