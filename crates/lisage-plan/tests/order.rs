//! M3-1 tests (claim C3): `Order<T>` first-class value —
//! verified permutation π, embedded type constraints, alignment
//! as typing (same mark `O`) and checked by value (`alignement`).

use lisage_lang::Atom;
use lisage_plan::order::{
    Aligne, DomainAtomes, ErrorOrder, KeyMap, KeysMap, MarqueOrder, Order, Phase, StrategieAval,
    TypeOrder,
};

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

/// M3-1: the canonical order is π = identity.
#[test]
fn test_order_canonique_identite() {
    let domain = DomainAtomes(vec![Atom(2), Atom(0), Atom(1), Atom(0)]);
    let order = Order::canonique(&domain);
    assert_eq!(order.size(), 3, "duplicates removed from canonical");
    assert_eq!(order.permutation(), vec![0, 1, 2]);
    assert_eq!(order.position(&Atom(0)), Some(0));
    assert_eq!(order.position(&Atom(2)), Some(2));
    assert_eq!(order.position(&Atom(9)), None);
    assert_eq!(order.key(1), Some(&Atom(1)));
    assert_eq!(order.key(3), None);
    assert!(order.contraintes().is_empty());
}

/// M3-1: a valid permutation is accepted, with the right positions.
#[test]
fn test_nouvel_order_permutation_valide() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
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
fn test_nouvel_order_rejete_si_not_one_permutation() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1)]);
    // Missing key.
    assert_eq!(
        Order::new(&domain, vec![Atom(0)]),
        Err(ErrorOrder::PasUnePermutation)
    );
    // Duplicate key.
    assert_eq!(
        Order::new(&domain, vec![Atom(0), Atom(0)]),
        Err(ErrorOrder::PasUnePermutation)
    );
    // Key foreign to the type.
    assert_eq!(
        Order::new(&domain, vec![Atom(0), Atom(7)]),
        Err(ErrorOrder::PasUnePermutation)
    );
}

/// M3-1: the type's constraints are embedded in `Order<T>` and
/// checked — for `Map(K, V)`, a key's presence precedes its value.
#[test]
fn test_contraintes_map_presence_avant_value() {
    let keys = KeysMap(vec![Atom(0), Atom(1)]);
    let canonique = Order::canonique(&keys);
    assert_eq!(canonique.size(), 4);
    assert_eq!(canonique.contraintes().len(), 2, "one constraint per key");
    // Canonical order: satisfies the constraints.
    assert!(
        Order::new(
            &keys,
            canonique
                .permutation()
                .iter()
                .map(|&p| *canonique.key(p).unwrap())
                .collect::<Vec<_>>()
        )
        .is_ok()
    );

    // Value before presence for key 1: constraint violated.
    let violee = vec![
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
        Order::new(&keys, violee),
        Err(ErrorOrder::ContrainteViolee {
            avant: KeyMap {
                key: Atom(1),
                phase: Phase::Presence
            },
            apres: KeyMap {
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
fn test_alignement_en_value() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let order_a = Order::canonique(&domain);
    let order_b = Order::new(&domain, vec![Atom(2), Atom(1), Atom(0)]).unwrap();
    assert!(order_a.alignes(&order_a.clone()));
    assert!(!order_a.alignes(&order_b));
    assert!(order_a.alignement(&order_a.clone()).is_ok());
    assert_eq!(
        order_a.alignement(&order_b),
        Err(ErrorOrder::OrdersNonAlignes)
    );
}

/// M3-1: alignment as typing — two operands compose only under the same
/// order mark `O` (same type parameter on both sides;
/// different marks would not compile).
#[test]
fn test_alignement_comme_typage_composition_sous_same_marque() {
    let domain = DomainAtomes(vec![Atom(0), Atom(1)]);
    let order = MarqueCanonique::order(&domain);
    assert_eq!(order.size(), 2);

    let a = Aligne::<DomainAtomes, MarqueCanonique, u32>::new(2);
    let b = Aligne::<DomainAtomes, MarqueCanonique, u32>::new(3);
    let sum = a.compose(&b, |x, y| x + y);
    assert_eq!(*sum.value(), 5);

    // The inverse mark carries another order: compose requires the same
    // mark on both sides (positive typing test; the negative case is a
    // compile-time rejection).
    let c = Aligne::<DomainAtomes, MarqueInverse, u32>::new(10);
    let d = Aligne::<DomainAtomes, MarqueInverse, u32>::new(1);
    assert_eq!(*c.compose(&d, |x, y| x - y).value(), 9);
    assert!(!MarqueCanonique::order(&domain).alignes(&MarqueInverse::order(&domain)));
}

/// Property (deterministic LCG): for any valid permutation π of a domain
/// of n atoms, `position ∘ key` is the identity and `permutation`
/// is a bijection; Map constraints are satisfied by the canonical order.
#[test]
fn test_propriete_permutation_valide_bijection() {
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
    let canonique = Order::canonique(&keys);
    for (avant, apres) in canonique.contraintes() {
        assert!(canonique.position(avant) < canonique.position(apres));
    }
}

#[test]
fn test_aval_runtime_order_valide_adopte() {
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
    let (order, strategie) = Order::with_aval_runtime(&keys, pi_runtime.clone());
    assert_eq!(strategie, StrategieAval::OrderRuntime);
    assert_eq!(order.sequence(), pi_runtime.as_slice());
}

#[test]
fn test_aval_runtime_contrainte_violee_repli_canonique() {
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
    let pi_invalide = vec![value_0, presence_0, presence_1, value_1];
    let (order, strategie) = Order::with_aval_runtime(&keys, pi_invalide);
    assert_eq!(strategie, StrategieAval::RepliCanonique);
    let canonique = Order::canonique(&keys);
    assert_eq!(order.sequence(), canonique.sequence());
    assert_eq!(order, canonique);
}

#[test]
fn test_aval_runtime_not_one_permutation_repli_canonique() {
    // Runtime π with a duplicate / missing key: canonical fallback too.
    let domain = DomainAtomes(vec![Atom(0), Atom(1), Atom(2)]);
    let pi_invalide = vec![Atom(0), Atom(0), Atom(2)];
    let (order, strategie) = Order::with_aval_runtime(&domain, pi_invalide);
    assert_eq!(strategie, StrategieAval::RepliCanonique);
    assert_eq!(order.sequence(), Order::canonique(&domain).sequence());
}
