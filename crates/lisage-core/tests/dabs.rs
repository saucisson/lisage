//! Tests for the generalized default (M4-3, statement A6, spec § 2.4.1):
//! (D-abs) absorption of arcs toward D^def over any subdomain R ⊆ Val(ℓ),
//! and (D-trans) transparent node (A = ∅ or all targets ≡ D → D). The D^def
//! remains designated by the controller (`default_payload`), never by the
//! form. The § 2.3 system (R-default on δ(ℓ) alone) remains the default.

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::reduction::{Reduction, reduce_dabs};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// LCG seed for property tests (same constant as the other suites).
const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// LCG increment for property tests.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Mock cardinality: 4 values per level.
const CARDINALITY_MOCK: u64 = 4;
/// Test level.
const LEVEL_TEST: u32 = 1;
/// Test anchor.
const ANCHOR_TEST: Anchor = Anchor(0);

/// Node 0 plays the role of the default diagram D^def in property tests.
const NODE_DEFAUT: NodeId = NodeId(0);

struct MockKeys {
    with_defaut: bool,
}

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITY_MOCK
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        self.with_defaut.then_some(0)
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
    type Arcs = ArcRun<Self::Key>;
    const FINGERPRINT: ControllerId = ControllerId(13);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
    fn generalized_absorption(&self) -> bool {
        self.absorption
    }
}

/// Pool with the generalized default enabled (D-abs/D-trans) and default key 0.
fn arena_dabs() -> Arena<MockController> {
    Arena::new(
        MockController {
            keys: MockKeys { with_defaut: true },
            absorption: true,
        },
        Box::new(NoOpObserver),
    )
}

/// Pool with the § 2.3 system alone (absorption not enabled).
fn arena_23() -> Arena<MockController> {
    Arena::new(
        MockController {
            keys: MockKeys { with_defaut: true },
            absorption: false,
        },
        Box::new(NoOpObserver),
    )
}

/// M4-3 (D-abs): every arc toward D^def is absorbed, over any subdomain —
/// not just the δ(ℓ) = 0 arc. Here keys 0, 2 and 3 lead to ⊥ = D^def: they
/// disappear, leaving {1 ↦ ⊤}.
#[test]
fn test_dabs_absorption_tout_sous_domain() {
    let mut pool = arena_dabs();
    let faux = pool.intern(ANCHOR_TEST, Form::Terminal(false));
    let vrai = pool.intern(ANCHOR_TEST, Form::Terminal(true));
    let node = pool.intern(
        ANCHOR_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(0, faux), (1, vrai), (2, faux), (3, faux)]),
        },
    );
    assert_eq!(
        pool.resolve(node),
        Ok(Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(1, vrai)]),
        }),
        "D-abs: the arcs 0, 2, 3 toward D^def are absorbed"
    );
}

/// M4-3: without D-abs (§ 2.3 system, `generalized_absorption` false), only
/// the δ(ℓ) = 0 arc toward D^def is absorbed — the 2 ↦ ⊥ arc remains.
#[test]
fn test_without_dabs_rdefault_seul() {
    let mut pool = arena_23();
    let faux = pool.intern(ANCHOR_TEST, Form::Terminal(false));
    let vrai = pool.intern(ANCHOR_TEST, Form::Terminal(true));
    let node = pool.intern(
        ANCHOR_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(0, faux), (1, vrai), (2, faux)]),
        },
    );
    assert_eq!(
        pool.resolve(node),
        Ok(Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(1, vrai), (2, faux)]),
        }),
        "R-default: only the δ(ℓ) = 0 arc is absorbed"
    );
}

/// M4-3 (D-trans): A = ∅ after absorption → collapse into D^def. A node whose
/// only arc (key 3, off δ) leads to D^def is transparent: it is D^def itself.
#[test]
fn test_dtrans_form_empty_collapse() {
    let mut pool = arena_dabs();
    let faux = pool.intern(ANCHOR_TEST, Form::Terminal(false));
    let node = pool.intern(
        ANCHOR_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(3, faux)]),
        },
    );
    assert_eq!(node, faux, "D-trans: ν(ℓ, {{3 ↦ D^def}}) = D^def");
    assert_eq!(pool.resolve(node), Ok(Form::Terminal(false)));
}

/// M4-3 (D-trans) "all targets ≡ D": a single explicit arc on δ collapses
/// toward its target; two explicit arcs toward the same target outside
/// coverage do not collapse (presence, confluence § 2.3).
#[test]
fn test_dtrans_coverage() {
    let mut pool = arena_dabs();
    let vrai = pool.intern(ANCHOR_TEST, Form::Terminal(true));
    // Only the explicit δ = 0 arc, target ⊤: covers (absents = D^def ≠ ⊤,
    // but the coverage rule allows collapse on the δ arc alone).
    let seul_delta = pool.intern(
        ANCHOR_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(0, vrai)]),
        },
    );
    assert_eq!(seul_delta, vrai, "δ arc alone: collapse into its target");
    // Two explicit arcs toward ⊤, outside coverage: no collapse.
    let deux_arcs = pool.intern(
        ANCHOR_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(1, vrai), (2, vrai)]),
        },
    );
    assert_eq!(
        pool.resolve(deux_arcs),
        Ok(Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(1, vrai), (2, vrai)]),
        }),
        "outside coverage: both arcs remain (presence)"
    );
}

/// M4-3: without a default key (δ(ℓ) = None), D-abs does not apply — the
/// absent arcs would not be worth D^def.
#[test]
fn test_without_key_defaut_pas_dabs() {
    let mut pool = Arena::new(
        MockController {
            keys: MockKeys { with_defaut: false },
            absorption: true,
        },
        Box::new(NoOpObserver),
    );
    let faux = pool.intern(ANCHOR_TEST, Form::Terminal(false));
    let node = pool.intern(
        ANCHOR_TEST,
        Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(1, faux), (2, faux)]),
        },
    );
    assert_eq!(
        pool.resolve(node),
        Ok(Form::Node {
            level: Level(LEVEL_TEST),
            arcs: ArcRun::from_pairs([(1, faux), (2, faux)]),
        }),
        "no δ(ℓ): no absorption"
    );
}

/// M4-3 (D-trans): a literally empty form (A = ∅) with a designated default
/// collapses into D^def, even without any arc to designate it.
#[test]
fn test_dtrans_form_empty_litterale() {
    let est_defaut = |node: NodeId| node == NODE_DEFAUT;
    let result = reduce_dabs(
        ArcRun::from_pairs([]),
        Some(0),
        &est_defaut,
        CARDINALITY_MOCK,
        Some(NODE_DEFAUT),
    );
    assert_eq!(result, Reduction::Collapse(NODE_DEFAUT));
    // Without a designated D^def: the empty form remains an empty form.
    let result = reduce_dabs(
        ArcRun::from_pairs([]),
        Some(0),
        &est_defaut,
        CARDINALITY_MOCK,
        None,
    );
    assert_eq!(result, Reduction::Form(ArcRun::from_pairs([])));
}

/// M4-3 (property tier, LCG): confluence of the generalized default —
/// `reduce_dabs ∘ reduce_dabs = reduce_dabs` (idempotence), no resulting
/// form keeps an arc toward D^def (when δ exists), and a collapse can only
/// target D^def or a covered target.
#[test]
fn test_propriete_dabs_confluence() {
    let est_defaut = |node: NodeId| node == NODE_DEFAUT;
    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed >> 33
    };

    for _ in 0..1000 {
        let nb_arcs = next() % 6;
        let arcs: ArcRun<u64> = (0..nb_arcs)
            .map(|_| {
                let key = next() % CARDINALITY_MOCK;
                let target = NodeId((next() % 3) as u32);
                (key, target)
            })
            .collect();
        let key_defaut = if next() % 2 == 0 { Some(0) } else { None };

        let defaut = key_defaut.map(|_| NODE_DEFAUT);
        let first = reduce_dabs(
            arcs.clone(),
            key_defaut,
            &est_defaut,
            CARDINALITY_MOCK,
            defaut,
        );
        // Idempotence: a reduced form is a fixed point (a Collapse targets a
        // node, not an arc form — nothing to re-reduce).
        if let Reduction::Form(arcs_reduceds) = &first {
            let second = reduce_dabs(
                arcs_reduceds.clone(),
                key_defaut,
                &est_defaut,
                CARDINALITY_MOCK,
                defaut,
            );
            assert_eq!(first.clone(), second, "idempotence of reduce_dabs");
        }

        // D-abs: when δ(ℓ) exists, no arc toward D^def remains.
        if key_defaut.is_some()
            && let Reduction::Form(a) = &first
        {
            assert!(
                a.values().all(|c| !est_defaut(*c)),
                "no arc toward D^def after D-abs"
            );
        }
        // Inactivity without δ(ℓ): never a collapse on an empty form, even
        // with D^def designated (the absent arcs would not be worth D^def).
        if key_defaut.is_none()
            && arcs.is_empty()
            && let Reduction::Collapse(_) = &first
        {
            panic!("collapse on empty form without a default key");
        }
        // D-trans / R-collapse: a collapse only happens if the coverage is
        // satisfied (same rules as `reduce`) or if A = ∅ after absorption
        // (then the target is D^def).
        if let Reduction::Collapse(target) = &first {
            // Same arcs after D-abs as in `reduce_dabs`.
            let arcs_filtres: ArcRun<u64> = match key_defaut {
                Some(_) => ArcRun::from_pairs(
                    arcs.iter()
                        .filter(|(_, c)| !est_defaut(**c))
                        .map(|(k, v)| (*k, *v)),
                ),
                None => arcs.clone(),
            };
            let toutes_defaut = !arcs.is_empty() && arcs.values().all(|c| est_defaut(*c));
            let couvre = match key_defaut {
                None => arcs_filtres.len() as u64 >= CARDINALITY_MOCK,
                Some(key) => {
                    est_defaut(*target)
                        || toutes_defaut
                        || (arcs_filtres.len() == 1 && arcs_filtres.contains_key(&key))
                }
            };
            assert!(couvre, "lawful collapse: target={target:?}");
        }
    }
}
