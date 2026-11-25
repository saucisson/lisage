//! M3-1 tests (claim C3): `Order<T>` first-class value —
//! verified permutation π, embedded type constraints, alignment
//! as typing (same mark `O`) and checked by value (`alignment`).

use lisage_lang::Atom;
use lisage_plan::order::{
    Aligned, AtomDomain, DownstreamStrategy, ErrorOrder, KeyMap, KeysMap, Order, OrderMark, Phase,
    TypeOrder,
};

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

/// M3-1: the canonical order is π = identity.
#[test]
fn test_order_canonical_identity() {
    let domain = AtomDomain(vec![Atom(2), Atom(0), Atom(1), Atom(0)]);
    let order = Order::canonical(&domain);
    assert_eq!(order.size(), 3, "duplicates removed from canonical");
    assert_eq!(order.permutation(), vec![0, 1, 2]);
    assert_eq!(order.position(&Atom(0)), Some(0));
    assert_eq!(order.position(&Atom(2)), Some(2));
    assert_eq!(order.position(&Atom(9)), None);
    assert_eq!(order.key(1), Some(&Atom(1)));
    assert_eq!(order.key(3), None);
    assert!(order.constraints().is_empty());
}

/// M3-1: a valid permutation is accepted, with the right positions.
#[test]
fn test_nouvel_order_permutation_valid() {
    let domain = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
    let order = Order::new(&domain, vec![Atom(2), Atom(0), Atom(1)]).unwrap();
    assert_eq!(
        order.permutation(),
        vec![1, 2, 0],
        "π: canonical → position"
    );
    assert_eq!(order.position(&Atom(2)), Some(0));
    assert_eq!(order.key(0), Some(&Atom(2)));
}

/// M3-1: π that is not a permutation is rejected (typed error).
#[test]
fn test_nouvel_order_rejected_if_not_one_permutation() {
    let domain = AtomDomain(vec![Atom(0), Atom(1)]);
    // Missing key.
    assert_eq!(
        Order::new(&domain, vec![Atom(0)]),
        Err(ErrorOrder::NotAPermutation)
    );
    // Duplicate key.
    assert_eq!(
        Order::new(&domain, vec![Atom(0), Atom(0)]),
        Err(ErrorOrder::NotAPermutation)
    );
    // Key foreign to the type.
    assert_eq!(
        Order::new(&domain, vec![Atom(0), Atom(7)]),
        Err(ErrorOrder::NotAPermutation)
    );
}

/// M3-1: the type's constraints are embedded in `Order<T>` and
/// checked — for `Map(K, V)`, a key's presence precedes its value.
#[test]
fn test_constraints_map_presence_before_value() {
    let keys = KeysMap(vec![Atom(0), Atom(1)]);
    let canonical = Order::canonical(&keys);
    assert_eq!(canonical.size(), 4);
    assert_eq!(canonical.constraints().len(), 2, "one constraint per key");
    // Canonical order: satisfies the constraints.
    assert!(
        Order::new(
            &keys,
            canonical
                .permutation()
                .iter()
                .map(|&p| *canonical.key(p).unwrap())
                .collect::<Vec<_>>()
        )
        .is_ok()
    );

    // Value before presence for key 1: constraint violated.
    let violated = vec![
        KeyMap {
            key: Atom(0),
            phase: Phase::Presence,
        },
        KeyMap {
            key: Atom(0),
            phase: Phase::Value,
        },
        KeyMap {
            key: Atom(1),
            phase: Phase::Value,
        },
        KeyMap {
            key: Atom(1),
            phase: Phase::Presence,
        },
    ];
    assert_eq!(
        Order::new(&keys, violated),
        Err(ErrorOrder::ConstraintViolated {
            before: KeyMap {
                key: Atom(1),
                phase: Phase::Presence
            },
            after: KeyMap {
                key: Atom(1),
                phase: Phase::Value
            },
        })
    );

    // Presence and value of the same key swapped in total disorder:
    // any permutation that inverts a (presence, value) pair is rejected.
    let inverse = Order::new(
        &keys,
        vec![
            KeyMap {
                key: Atom(1),
                phase: Phase::Presence,
            },
            KeyMap {
                key: Atom(1),
                phase: Phase::Value,
            },
            KeyMap {
                key: Atom(0),
                phase: Phase::Presence,
            },
            KeyMap {
                key: Atom(0),
                phase: Phase::Value,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        inverse.position(&KeyMap {
            key: Atom(1),
            phase: Phase::Presence
        }),
        Some(0)
    );
}

/// M3-1: alignment by value — two orders of the same type are aligned iff same π.
#[test]
fn test_value_alignment() {
    let domain = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
    let order_a = Order::canonical(&domain);
    let order_b = Order::new(&domain, vec![Atom(2), Atom(1), Atom(0)]).unwrap();
    assert!(order_a.aligned(&order_a.clone()));
    assert!(!order_a.aligned(&order_b));
    assert!(order_a.alignment(&order_a.clone()).is_ok());
    assert_eq!(
        order_a.alignment(&order_b),
        Err(ErrorOrder::OrdersNonAligned)
    );
}

/// M3-1: alignment as typing — two operands compose only under the same
/// order mark `O` (same type parameter on both sides;
/// different marks would not compile).
#[test]
fn test_alignment_as_typing_composition_sub_same_mark() {
    let domain = AtomDomain(vec![Atom(0), Atom(1)]);
    let order = CanonicalMark::order(&domain);
    assert_eq!(order.size(), 2);

    let a = Aligned::<AtomDomain, CanonicalMark, u32>::new(2);
    let b = Aligned::<AtomDomain, CanonicalMark, u32>::new(3);
    let sum = a.compose(&b, |x, y| x + y);
    assert_eq!(*sum.value(), 5);

    // The inverse mark carries another order: compose requires the same
    // mark on both sides (positive typing test; the negative case is a
    // compile-time rejection).
    let c = Aligned::<AtomDomain, MarkInverse, u32>::new(10);
    let d = Aligned::<AtomDomain, MarkInverse, u32>::new(1);
    assert_eq!(*c.compose(&d, |x, y| x - y).value(), 9);
    assert!(!CanonicalMark::order(&domain).aligned(&MarkInverse::order(&domain)));
}

/// Property (deterministic LCG): for any valid permutation π of a domain
/// of n atoms, `position ∘ key` is the identity and `permutation`
/// is a bijection; Map constraints are satisfied by the canonical order.
#[test]
fn test_property_permutation_valid_bijection() {
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
        // Random permutation (deterministic Fisher-Yates shuffle).
        let mut pi: Vec<Atom> = (0..size).map(|i| Atom(i as u64)).collect();
        for i in (1..size).rev() {
            let j = (next() as usize) % (i + 1);
            pi.swap(i, j);
        }
        let order = Order::new(&domain, pi.clone()).expect("valid permutation");
        assert_eq!(order.size(), size);
        let mut positions: Vec<usize> =
            (0..size).map(|i| order.position(&pi[i]).unwrap()).collect();
        positions.sort();
        assert_eq!(positions, (0..size).collect::<Vec<_>>(), "π bijective");
        for (index, key) in pi.iter().enumerate() {
            assert_eq!(order.key(order.position(key).unwrap()), Some(key));
            let _ = index;
        }
        let mut permutation = order.permutation();
        permutation.sort();
        assert_eq!(permutation, (0..size).collect::<Vec<_>>());
    }

    // Map constraints: the canonical order always satisfies them.
    let keys = KeysMap((0..5).map(Atom).collect());
    let canonical = Order::canonical(&keys);
    for (before, after) in canonical.constraints() {
        assert!(canonical.position(before) < canonical.position(after));
    }
}

#[test]
fn test_downstream_runtime_order_valid_adopted() {
    // Valid runtime π (permutation respecting presence < value):
    // OrderRuntime strategy, sequence = provided π.
    let keys = KeysMap(vec![Atom(0), Atom(1)]);
    let presence_0 = KeyMap {
        key: Atom(0),
        phase: Phase::Presence,
    };
    let value_0 = KeyMap {
        key: Atom(0),
        phase: Phase::Value,
    };
    let presence_1 = KeyMap {
        key: Atom(1),
        phase: Phase::Presence,
    };
    let value_1 = KeyMap {
        key: Atom(1),
        phase: Phase::Value,
    };
    let pi_runtime = vec![presence_1, value_1, presence_0, value_0];
    let (order, strategy) = Order::with_downstream_runtime(&keys, pi_runtime.clone());
    assert_eq!(strategy, DownstreamStrategy::OrderRuntime);
    assert_eq!(order.sequence(), pi_runtime.as_slice());
}

#[test]
fn test_downstream_runtime_constraint_violated_fallback_canonical() {
    // Runtime π that violates presence < value for key 0:
    // fallback to the canonical order (degraded reordering as fallback).
    let keys = KeysMap(vec![Atom(0), Atom(1)]);
    let presence_0 = KeyMap {
        key: Atom(0),
        phase: Phase::Presence,
    };
    let value_0 = KeyMap {
        key: Atom(0),
        phase: Phase::Value,
    };
    let presence_1 = KeyMap {
        key: Atom(1),
        phase: Phase::Presence,
    };
    let value_1 = KeyMap {
        key: Atom(1),
        phase: Phase::Value,
    };
    let pi_invalid = vec![value_0, presence_0, presence_1, value_1];
    let (order, strategy) = Order::with_downstream_runtime(&keys, pi_invalid);
    assert_eq!(strategy, DownstreamStrategy::CanonicalFallback);
    let canonical = Order::canonical(&keys);
    assert_eq!(order.sequence(), canonical.sequence());
    assert_eq!(order, canonical);
}

#[test]
fn test_downstream_runtime_not_one_permutation_fallback_canonical() {
    // Runtime π with a duplicate / missing key: canonical fallback too.
    let domain = AtomDomain(vec![Atom(0), Atom(1), Atom(2)]);
    let pi_invalid = vec![Atom(0), Atom(0), Atom(2)];
    let (order, strategy) = Order::with_downstream_runtime(&domain, pi_invalid);
    assert_eq!(strategy, DownstreamStrategy::CanonicalFallback);
    assert_eq!(order.sequence(), Order::canonical(&domain).sequence());
}
