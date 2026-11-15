//! Tests for relative forms (M4-2, statement C8, spec § 2.4.5): shift
//! activation (relative anchor), composite anchors per fragment, key
//! transformation group (canonical instance ShiftGroup, min = 0 section),
//! entry arc (term, handle). Test model: the sliding window — the sequence
//! base lives on the entry arc, the slide is the anchor displacement, the pool
//! stays at O(1) forms.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::relatif::{
    AnchorComposee, AnchorRelative, CycleGroup, Fragment, InputArc, ShiftGroup,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// LCG seed for property tests (same constant as the other suites).
const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// LCG increment for property tests.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Declared mock cardinality: large enough to never collapse.
const CARDINALITY_MOCK: u64 = 1_000_000;
/// Size of the sliding window of the test model.
const SIZE_FENETRE: u64 = 4;
/// Number of slides of the sliding window of the test model.
const NB_GLISSES: u64 = 50;
/// Number of shifts per fragment in the O(Σ kᵢ) degradation test.
const NB_DECALAGES_PAR_FRAGMENT: u64 = 8;
/// Level of the "machine" fragment.
const LEVEL_MACHINE: u32 = 1;
/// Level of the "convoyeur" fragment.
const LEVEL_CONVOYEUR: u32 = 2;
/// Base position of the relative anchor.
const POSITION_BASE: u32 = 100;

struct MockKeys;

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITY_MOCK
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(11);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

fn arena() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

/// M4-2: relative anchor — shift activation. The absolute position reads as
/// `position + decalage`, the slide shifts, the return to an absolute anchor
/// is saturated.
#[test]
fn test_anchor_relative_shift() {
    let anchor = AnchorRelative::new(POSITION_BASE);
    assert_eq!(anchor.decalage, 0);
    assert_eq!(anchor.absolue(), POSITION_BASE as i64);
    let glissee = anchor.decalee(1).decalee(1).decalee(-1);
    assert_eq!(glissee.absolue(), POSITION_BASE as i64 + 1);
    assert_eq!(glissee.vers_anchor(), Anchor(POSITION_BASE + 1));
    // From an absolute M1 anchor: zero shift.
    let depuis = AnchorRelative::depuis_anchor(Anchor(7));
    assert_eq!(depuis.decalage, 0);
    assert_eq!(depuis.absolue(), 7);
    // Saturation of the conversion to an absolute anchor: a negative shift
    // below position 0 clamps to 0 (no wrap), an overflow above clamps to
    // u32::MAX.
    assert_eq!(AnchorRelative::new(0).decalee(-1).vers_anchor(), Anchor(0));
    assert_eq!(
        AnchorRelative::new(u32::MAX).decalee(5).vers_anchor(),
        Anchor(u32::MAX)
    );
}

/// M4-2: the effective shift reading is saturated in ℕ (total): key + negative
/// argument clamps to 0, overflow above clamps to u64::MAX.
#[test]
fn test_groupe_shift_saturation() {
    let groupe = ShiftGroup;
    assert_eq!(groupe.apply(&0, &-5), 0, "low clamp to 0");
    assert_eq!(groupe.apply(&10, &-5), 5);
    assert_eq!(groupe.apply(&u64::MAX, &1), u64::MAX, "high clamp");
    assert_eq!(groupe.apply(&u64::MAX, &-1), u64::MAX - 1);
}

/// M4-2: composite anchors — named recording of shifts, one per fragment;
/// zero = fragment start; lexicographic order of names; pointwise
/// composition.
#[test]
fn test_anchor_composee_fragments() {
    let machine = Fragment("machine".to_string());
    let convoyeur = Fragment("convoyeur".to_string());
    let anchor = AnchorComposee::new()
        .avec(machine.clone(), 3)
        .avec(convoyeur.clone(), -1);
    assert_eq!(anchor.decalage(&machine), 3);
    assert_eq!(anchor.decalage(&convoyeur), -1);
    // Absent fragment: zero = fragment start.
    assert_eq!(anchor.decalage(&Fragment("other".to_string())), 0);
    assert!(!anchor.est_nulle());
    // The slide shifts a single fragment.
    let glissee = anchor.decalee(&machine, 2);
    assert_eq!(glissee.decalage(&machine), 5);
    assert_eq!(glissee.decalage(&convoyeur), -1);
    // Pointwise composition (absolute reading = sum of the arguments).
    let composee = anchor.compose(&glissee);
    assert_eq!(composee.decalage(&machine), 8);
    assert_eq!(composee.decalage(&convoyeur), -2);
    // Canonical order: lexicographic by name.
    let noms: Vec<&str> = anchor.fragments().map(|(f, _)| f.0.as_str()).collect();
    assert_eq!(noms, ["convoyeur", "machine"]);
    assert!(AnchorComposee::new().est_nulle());
}

/// M4-2, test model: the **sliding window**. The sequence base is the
/// argument on the entry arc, the slide is the anchor displacement; the window
/// stored relatively (keys 0..W-1, min = 0 section) is shared: the pool stays
/// at O(1) forms while the base ranges over ℕ, and the absolute reading gives
/// back the window {base..base+W-1}.
#[test]
fn test_fenetre_glissante_pool_constant() {
    let groupe = ShiftGroup;
    let mut pool = arena();
    let anchor = Anchor(0);
    let vrai = pool.intern(anchor, Form::Terminal(true));
    // Relative window: keys 0..W-1 (min = 0 section), all true.
    let arcs: BTreeMap<u64, NodeId> = (0..SIZE_FENETRE).map(|k| (k, vrai)).collect();
    let fenetre = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs,
        },
    );
    let nodes_apres_fenetre = pool.node_count();
    // Slide: the base advances by 1 at each step, the stored form is
    // unchanged (R-share), only the entry arc's argument changes.
    for base in 0..NB_GLISSES as i64 {
        let arc = InputArc::new(base, pool.handle(fenetre, anchor));
        // Absolute reading: the window covers {base..base+W-1}.
        for k in 0..SIZE_FENETRE {
            assert_eq!(arc.read_key(&groupe, &k), (base + k as i64) as u64);
        }
        // Re-intern the same relative form: hit, no node created.
        let same = pool.intern(
            anchor,
            Form::Node {
                level: Level(1),
                arcs: (0..SIZE_FENETRE).map(|k| (k, vrai)).collect(),
            },
        );
        assert_eq!(same, fenetre, "shared relative form");
    }
    assert_eq!(
        pool.node_count(),
        nodes_apres_fenetre,
        "the pool stays at O(1) forms during the slide"
    );
}

/// M4-2: composite anchors — the degradation stays in O(Σ kᵢ) irregular
/// points, never in O(∏): the forms factorize per fragment.
#[test]
fn test_anchors_composees_degradation_somme() {
    let mut pool = arena();
    let anchor = Anchor(0);
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let machine = Fragment("machine".to_string());
    let convoyeur = Fragment("convoyeur".to_string());
    // k₁ forms for the machine fragment, k₂ for the convoyeur fragment.
    for i in 0..NB_DECALAGES_PAR_FRAGMENT {
        pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_MACHINE),
                arcs: BTreeMap::from([(i, vrai)]),
            },
        );
    }
    for j in 0..NB_DECALAGES_PAR_FRAGMENT {
        pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_CONVOYEUR),
                arcs: BTreeMap::from([(j, vrai)]),
            },
        );
    }
    // 1 terminal + k₁ + k₂ nodes: the sum, not the product k₁ × k₂.
    let expected = 1 + 2 * NB_DECALAGES_PAR_FRAGMENT as usize;
    assert_eq!(pool.node_count(), expected);
    assert!(expected < (1 + NB_DECALAGES_PAR_FRAGMENT as usize).pow(2));
    // The composite anchor carries the two named shifts.
    let anchor = AnchorComposee::new().avec(machine, 2).avec(convoyeur, 5);
    assert_eq!(anchor.decalage(&Fragment("machine".to_string())), 2);
    assert_eq!(anchor.decalage(&Fragment("convoyeur".to_string())), 5);
}

/// M4-2 (property tier, LCG): contract of the transformation group —
/// confluent section (idempotent), homomorphism f(n₁)∘f(n₂) = f(n₁ ⊕ n₂),
/// inverse t⁻¹, trivial kernel (f injective), composition of entry arcs.
/// Bounded values to avoid saturation.
#[test]
fn test_propriete_groupe_shift() {
    let groupe = ShiftGroup;
    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed >> 33
    };
    // Bound on the test's arguments/keys: no saturation.
    const BORNE: u64 = 500;

    for _ in 0..1000 {
        // Random arcs (keys in [0, BORNE), arbitrary targets).
        let nb_arcs = 1 + next() % 8;
        let arcs: BTreeMap<u64, NodeId> = (0..nb_arcs)
            .map(|_| (next() % BORNE, NodeId(next() as u32)))
            .collect();
        // (iii) Confluence: the section is idempotent — applied to the
        // normalized form, it yields the neutral argument and the same form.
        let (n1, arcs_norm) = groupe.section_arcs(&arcs);
        let (n2, arcs_norm2) = groupe.section_arcs(&arcs_norm);
        assert_eq!(n2, groupe.neutral(), "idempotent section");
        assert_eq!(arcs_norm2, arcs_norm);
        // The min = 0 section indeed starts at key 0.
        assert!(arcs_norm.contains_key(&0));
        assert_eq!(n1, *arcs.keys().next().unwrap() as i64);

        // (v) Homomorphism: f(n₁ ⊕ n₂) = f(n₁)∘f(n₂). Key large enough that
        // the intermediate readings do not saturate to 0.
        let key_grande = BORNE + next() % BORNE;
        let a1 = (next() % (2 * BORNE)) as i64 - BORNE as i64;
        let a2 = (next() % (2 * BORNE)) as i64 - BORNE as i64;
        let compose = groupe.compose(&a1, &a2);
        assert_eq!(
            groupe.apply(&key_grande, &compose),
            groupe.apply(&groupe.apply(&key_grande, &a1), &a2)
        );

        // (i) Inverse: f(n) then f(n)⁻¹ = identity (key large enough).
        let n = (next() % BORNE) as i64;
        assert_eq!(
            groupe.apply(&groupe.apply(&key_grande, &n), &groupe.invert(&n)),
            key_grande
        );

        // (vi) Trivial kernel: f injective over the arguments.
        let n1 = (next() % BORNE) as i64;
        let n2 = (next() % BORNE) as i64;
        if groupe.apply(&key_grande, &n1) == groupe.apply(&key_grande, &n2) {
            assert_eq!(
                n1, n2,
                "f(n₁) = f(n₂) ⇒ n₁ = n₂ (storage modulo the kernel)"
            );
        }

        // Entry arcs: the absolute reading composes the arguments.
        let handle = Handle {
            controller: ControllerId(11),
            anchor: Anchor(0),
            node: NodeId(1),
        };
        let arc1 = InputArc::new(a1, handle);
        let arc2 = InputArc::new(a2, handle);
        let arc_compose = arc1.compose(&groupe, &arc2);
        assert_eq!(
            arc_compose.read_key(&groupe, &key_grande),
            arc2.read_key(&groupe, &arc1.read_key(&groupe, &key_grande))
        );
    }
}
