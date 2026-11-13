//! Generalized default tests (M4-3, statement A6, spec § 2.4.1): (D-abs)
//! absorption of edges towards D^def over any sub-domain R ⊆ Val(ℓ), and
//! (D-trans) transparent node (A = ∅ or all targets ≡ D → D). The D^def
//! stays designated by the controller (`default_payload`), never by the
//! form. The § 2.3 system (R-default on δ(ℓ) alone) remains the default.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::reduction::{Reduction, reduce_dabs};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// Seed of the LCG for property tests (same constant as the other suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplier of the LCG for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// Increment of the LCG for property tests.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Cardinality of the mock: 4 values per level.
const CARDINALITE_MOCK: u64 = 4;
/// Test level.
const LEVEL_TEST: u32 = 1;
/// Test anchor.
const ANCRE_TEST: Anchor = Anchor(0);

/// Node 0 plays the role of the default diagram D^def in the property
/// tests.
const NODE_DEFAUT: NodeId = NodeId(0);

struct MockKeys {
    with_default: bool,
}

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITE_MOCK
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        self.with_default.then_some(0)
    }
}

struct MockController {
    keys: MockKeys,
    absorption: bool,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(13);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
    fn absorption_generalized(&self) -> bool {
        self.absorption
    }
}

/// Pool with generalized default enabled (D-abs/D-trans) and default key 0.
fn arena_dabs() -> Arena<MockController> {
    Arena::new(
        MockController {
            keys: MockKeys { with_default: true },
            absorption: true,
        },
        Box::new(NoOpObserver),
    )
}

/// Pool with the § 2.3 system alone (absorption not enabled).
fn arena_23() -> Arena<MockController> {
    Arena::new(
        MockController {
            keys: MockKeys { with_default: true },
            absorption: false,
        },
        Box::new(NoOpObserver),
    )
}

/// M4-3 (D-abs): any edge towards D^def is absorbed, over any
/// sub-domain — not only the edge δ(ℓ) = 0. Here keys 0, 2 and 3
/// lead to ⊥ = D^def: they disappear, leaving {1 ↦ ⊤}.
#[test]
fn test_dabs_absorption_all_under_domain() {
    let mut pool = arena_dabs();
    let false_node = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let true_node = pool.intern(ANCRE_TEST, Form::Terminal(true));
    let node = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([
                (0, false_node),
                (1, true_node),
                (2, false_node),
                (3, false_node),
            ]),
        },
    );
    assert_eq!(
        pool.resolve(node),
        Ok(&Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(1, true_node)]),
        }),
        "D-abs: the edges 0, 2, 3 towards D^def are absorbed"
    );
}

/// M4-3: without D-abs (§ 2.3 system, `absorption_generalized` false),
/// only the edge δ(ℓ) = 0 towards D^def is absorbed — the edge 2 ↦ ⊥ remains.
#[test]
fn test_without_dabs_rdefault_only() {
    let mut pool = arena_23();
    let false_node = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let true_node = pool.intern(ANCRE_TEST, Form::Terminal(true));
    let node = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(0, false_node), (1, true_node), (2, false_node)]),
        },
    );
    assert_eq!(
        pool.resolve(node),
        Ok(&Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(1, true_node), (2, false_node)]),
        }),
        "R-default: only the edge δ(ℓ) = 0 is absorbed"
    );
}

/// M4-3 (D-trans): A = ∅ after absorption → collapse to D^def. A node
/// whose only edge (key 3, outside δ) leads to D^def is transparent: it
/// is D^def itself.
#[test]
fn test_dtrans_form_empty_collapse() {
    let mut pool = arena_dabs();
    let false_node = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let node = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(3, false_node)]),
        },
    );
    assert_eq!(node, false_node, "D-trans: ν(ℓ, {{3 ↦ D^def}}) = D^def");
    assert_eq!(pool.resolve(node), Ok(&Form::Terminal(false)));
}

/// M4-3 (D-trans) "all targets ≡ D": a single explicit edge on δ
/// collapse towards its target; two explicit edges towards the same target outside
/// coverage do not collapse (presence, confluence § 2.3).
#[test]
fn test_dtrans_coverage() {
    let mut pool = arena_dabs();
    let true_node = pool.intern(ANCRE_TEST, Form::Terminal(true));
    // Only the explicit δ = 0 edge, target ⊤: covers (absent = D^def ≠ ⊤,
    // but the coverage rule allows the collapse on the δ edge alone).
    let only_delta = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(0, true_node)]),
        },
    );
    assert_eq!(
        only_delta, true_node,
        "δ edge alone: collapse to its target"
    );
    // Two explicit edges to ⊤, outside coverage: no collapse.
    let two_edges = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(1, true_node), (2, true_node)]),
        },
    );
    assert_eq!(
        pool.resolve(two_edges),
        Ok(&Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(1, true_node), (2, true_node)]),
        }),
        "outside coverage: the two edges remain (presence)"
    );
}

/// M4-3: without a default key (δ(ℓ) = None), D-abs does not apply —
/// the absent edges would not be worth D^def.
#[test]
fn test_without_key_default_not_dabs() {
    let mut pool = Arena::new(
        MockController {
            keys: MockKeys {
                with_default: false,
            },
            absorption: true,
        },
        Box::new(NoOpObserver),
    );
    let false_node = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let node = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(1, false_node), (2, false_node)]),
        },
    );
    assert_eq!(
        pool.resolve(node),
        Ok(&Form::Node {
            level: Level(LEVEL_TEST),
            edges: BTreeMap::from([(1, false_node), (2, false_node)]),
        }),
        "no δ(ℓ): no absorption"
    );
}

/// M4-3 (D-trans): a literally empty form (A = ∅) with a designated
/// default collapses to D^def, even without any edge to designate it.
#[test]
fn test_dtrans_form_empty_literal() {
    let is_default = |node: NodeId| node == NODE_DEFAUT;
    let result = reduce_dabs(
        BTreeMap::new(),
        Some(0),
        &is_default,
        CARDINALITE_MOCK,
        Some(NODE_DEFAUT),
    );
    assert_eq!(result, Reduction::Collapse(NODE_DEFAUT));
    // Without a designated D^def: the empty form remains an empty form.
    let result = reduce_dabs(
        BTreeMap::new(),
        Some(0),
        &is_default,
        CARDINALITE_MOCK,
        None,
    );
    assert_eq!(result, Reduction::Form(BTreeMap::new()));
}

/// M4-3 (property tier, LCG): confluence of the generalized default —
/// `reduce_dabs ∘ reduce_dabs = reduce_dabs` (idempotence), no resulting
/// form keeps an edge towards D^def (when δ exists), and a
/// collapse can only target D^def or a covered target.
#[test]
fn test_property_dabs_confluence() {
    let is_default = |node: NodeId| node == NODE_DEFAUT;
    let mut graine = GRAINE_LCG;
    let mut next = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _ in 0..1000 {
        let count_edges = next() % 6;
        let edges: BTreeMap<u64, NodeId> = (0..count_edges)
            .map(|_| {
                let key = next() % CARDINALITE_MOCK;
                let target = NodeId((next() % 3) as u32);
                (key, target)
            })
            .collect();
        let key_default = if next() % 2 == 0 { Some(0) } else { None };

        let default = key_default.map(|_| NODE_DEFAUT);
        let first = reduce_dabs(
            edges.clone(),
            key_default,
            &is_default,
            CARDINALITE_MOCK,
            default,
        );
        // Idempotence: a reduced form is a fixpoint (a Collapse
        // targets a node, not an edge form — nothing to re-reduce).
        if let Reduction::Form(edges_reduced) = &first {
            let second = reduce_dabs(
                edges_reduced.clone(),
                key_default,
                &is_default,
                CARDINALITE_MOCK,
                default,
            );
            assert_eq!(first.clone(), second, "idempotence of reduce_dabs");
        }

        // D-abs: when δ(ℓ) exists, no edge towards D^def remains.
        if key_default.is_some()
            && let Reduction::Form(a) = &first
        {
            assert!(
                a.values().all(|c| !is_default(*c)),
                "no edge towards D^def after D-abs"
            );
        }
        // Inactivity without δ(ℓ): never a collapse on an empty form, even
        // with a designated D^def (the absent edges would not be worth D^def).
        if key_default.is_none()
            && edges.is_empty()
            && let Reduction::Collapse(_) = &first
        {
            panic!("collapse on empty form without a default key");
        }
        // D-trans / R-collapse: a collapse happens only if the coverage
        // is satisfied (same rules as `reduce`) or if A = ∅ after
        // absorption (then the target is D^def).
        if let Reduction::Collapse(target) = &first {
            // Same edges after D-abs as in `reduce_dabs`.
            let edges_filters: BTreeMap<u64, NodeId> = match key_default {
                Some(_) => edges
                    .iter()
                    .filter(|(_, c)| !is_default(**c))
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                None => edges.clone(),
            };
            let all_default = !edges.is_empty() && edges.values().all(|c| is_default(*c));
            let covers = match key_default {
                None => edges_filters.len() as u64 >= CARDINALITE_MOCK,
                Some(key) => {
                    is_default(*target)
                        || all_default
                        || (edges_filters.len() == 1 && edges_filters.contains_key(&key))
                }
            };
            assert!(covers, "lawful collapse: target={target:?}");
        }
    }
}
