//! Relative form tests (M4-2, statement C8, spec § 2.4.5): shift
//! activation (relative anchor), composed anchors per fragment, key
//! transformation group (canonical GroupShift instance, min
//! section = 0), input edge (term, handle). Test model: the sliding
//! window — the sequence base lives on the input edge, the slide is
//! the anchor displacement, the pool stays at O(1) forms.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::relative::{
    AnchorComposed, AnchorRelative, EdgeInput, Fragment, GroupKeys, GroupShift,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// Seed of the LCG for property tests (same constant as the other suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplier of the LCG for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// Increment of the LCG for property tests.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Declared cardinality of the mock: large enough to never collapse.
const CARDINALITE_MOCK: u64 = 1_000_000;
/// Size of the sliding window of the test model.
const SIZE_FENETRE: u64 = 4;
/// Number of slides of the sliding window of the test model.
const NB_GLISSES: u64 = 50;
/// Number of offsets per fragment of the O(Σ kᵢ) degradation test.
const NB_DECALAGES_PAR_FRAGMENT: u64 = 8;
/// Level of the "machine" fragment.
const LEVEL_MACHINE: u32 = 1;
/// Level of the "conveyor" fragment.
const LEVEL_CONVEYOR: u32 = 2;
/// Base position of the relative anchor.
const POSITION_BASE: u32 = 100;

struct MockKeys;

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITE_MOCK
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

/// M4-2: relative anchor — shift activation. The absolute position is read
/// `position + offset`, the slide shifts, the return to an absolute anchor is
/// saturated.
#[test]
fn test_anchor_relative_shift() {
    let anchor = AnchorRelative::new(POSITION_BASE);
    assert_eq!(anchor.offset, 0);
    assert_eq!(anchor.absolute(), POSITION_BASE as i64);
    let slid = anchor.decalee(1).decalee(1).decalee(-1);
    assert_eq!(slid.absolute(), POSITION_BASE as i64 + 1);
    assert_eq!(slid.toward_anchor(), Anchor(POSITION_BASE + 1));
    // Since an absolute M1 anchor: zero offset.
    let since = AnchorRelative::since_anchor(Anchor(7));
    assert_eq!(since.offset, 0);
    assert_eq!(since.absolute(), 7);
    // Saturation of the conversion to an absolute anchor: a negative offset
    // below position 0 clamps to 0 (no wrap), a high overflow
    // clamps to u32::MAX.
    assert_eq!(
        AnchorRelative::new(0).decalee(-1).toward_anchor(),
        Anchor(0)
    );
    assert_eq!(
        AnchorRelative::new(u32::MAX).decalee(5).toward_anchor(),
        Anchor(u32::MAX)
    );
}

/// M4-2: the effective shift read is saturated in ℕ (total):
/// key + negative argument clamps to 0, high overflow clamps to u64::MAX.
#[test]
fn test_group_shift_saturation() {
    let group = GroupShift;
    assert_eq!(group.apply(&0, &-5), 0, "low clamp to 0");
    assert_eq!(group.apply(&10, &-5), 5);
    assert_eq!(group.apply(&u64::MAX, &1), u64::MAX, "high clamp");
    assert_eq!(group.apply(&u64::MAX, &-1), u64::MAX - 1);
}

/// M4-2: composed anchors — named record of offsets, one per
/// fragment; zero = start of fragment; lexicographic order of names;
/// composition point by point.
#[test]
fn test_anchor_composed_fragments() {
    let machine = Fragment("machine".to_string());
    let conveyor = Fragment("conveyor".to_string());
    let anchor = AnchorComposed::new()
        .with(machine.clone(), 3)
        .with(conveyor.clone(), -1);
    assert_eq!(anchor.offset(&machine), 3);
    assert_eq!(anchor.offset(&conveyor), -1);
    // Absent fragment: zero = start of fragment.
    assert_eq!(anchor.offset(&Fragment("other".to_string())), 0);
    assert!(!anchor.is_null());
    // The slide shifts a single fragment.
    let slid = anchor.decalee(&machine, 2);
    assert_eq!(slid.offset(&machine), 5);
    assert_eq!(slid.offset(&conveyor), -1);
    // Composition point by point (absolute read = sum of the arguments).
    let composed = anchor.composed(&slid);
    assert_eq!(composed.offset(&machine), 8);
    assert_eq!(composed.offset(&conveyor), -2);
    // Canonical order: lexicographic by name.
    let names: Vec<&str> = anchor.fragments().map(|(f, _)| f.0.as_str()).collect();
    assert_eq!(names, ["conveyor", "machine"]);
    assert!(AnchorComposed::new().is_null());
}

/// M4-2, test model: the **sliding window**. The sequence base is
/// the argument on the input edge, the slide is the anchor displacement;
/// the window stored in relative form (keys 0..W-1, min section = 0) is
/// shared: the pool stays at O(1) forms while the base ranges over ℕ, and the
/// absolute read gives back the window {base..base+W-1}.
#[test]
fn test_window_sliding_pool_constant() {
    let group = GroupShift;
    let mut pool = arena();
    let anchor = Anchor(0);
    let true_node = pool.intern(anchor, Form::Terminal(true));
    // Relative window: keys 0..W-1 (min section = 0), all true.
    let edges: BTreeMap<u64, NodeId> = (0..SIZE_FENETRE).map(|k| (k, true_node)).collect();
    let window = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            edges,
        },
    );
    let nodes_after_window = pool.node_count();
    // Slides: the base advances by 1 at each step, the stored form is
    // unchanged (R-share), only the argument of the input edge changes.
    for base in 0..NB_GLISSES as i64 {
        let edge = EdgeInput::new(base, pool.handle(window, anchor));
        // Absolute read: the window covers {base..base+W-1}.
        for k in 0..SIZE_FENETRE {
            assert_eq!(edge.read_key(&group, &k), (base + k as i64) as u64);
        }
        // Re-interning the same relative form: hit, no node created.
        let same = pool.intern(
            anchor,
            Form::Node {
                level: Level(1),
                edges: (0..SIZE_FENETRE).map(|k| (k, true_node)).collect(),
            },
        );
        assert_eq!(same, window, "shared relative form");
    }
    assert_eq!(
        pool.node_count(),
        nodes_after_window,
        "the pool stays at O(1) forms during the slide"
    );
}

/// M4-2: composed anchors — the degradation stays in O(Σ kᵢ) irregular
/// points, never in O(∏): the forms factorize per fragment.
#[test]
fn test_anchors_composed_degradation_sum() {
    let mut pool = arena();
    let anchor = Anchor(0);
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let machine = Fragment("machine".to_string());
    let conveyor = Fragment("conveyor".to_string());
    // k₁ forms for the machine fragment, k₂ for the conveyor fragment.
    for i in 0..NB_DECALAGES_PAR_FRAGMENT {
        pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_MACHINE),
                edges: BTreeMap::from([(i, true_node)]),
            },
        );
    }
    for j in 0..NB_DECALAGES_PAR_FRAGMENT {
        pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_CONVEYOR),
                edges: BTreeMap::from([(j, true_node)]),
            },
        );
    }
    // 1 terminal + k₁ + k₂ nodes: the sum, not the product k₁ × k₂.
    let expected = 1 + 2 * NB_DECALAGES_PAR_FRAGMENT as usize;
    assert_eq!(pool.node_count(), expected);
    assert!(expected < (1 + NB_DECALAGES_PAR_FRAGMENT as usize).pow(2));
    // The composed anchor carries the two named offsets.
    let anchor = AnchorComposed::new().with(machine, 2).with(conveyor, 5);
    assert_eq!(anchor.offset(&Fragment("machine".to_string())), 2);
    assert_eq!(anchor.offset(&Fragment("conveyor".to_string())), 5);
}

/// M4-2 (property tier, LCG): contract of the transformation group —
/// confluent (idempotent) section, homomorphism f(n₁)∘f(n₂) =
/// f(n₁ ⊕ n₂), inverse t⁻¹, trivial kernel (f injective), composition of the
/// input edges. Bounded values to avoid saturation.
#[test]
fn test_property_group_shift() {
    let group = GroupShift;
    let mut graine = GRAINE_LCG;
    let mut next = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };
    // Bound of the test arguments/keys: no saturation.
    const BOUND: u64 = 500;

    for _ in 0..1000 {
        // Random edges (keys in [0, BOUND), arbitrary targets).
        let count_edges = 1 + next() % 8;
        let edges: BTreeMap<u64, NodeId> = (0..count_edges)
            .map(|_| (next() % BOUND, NodeId(next() as u32)))
            .collect();
        // (iii) Confluence: the section is idempotent — applied to the
        // normalized form, it makes the argument neutral and the same form.
        let (n1, edges_norm) = group.section_edges(&edges);
        let (n2, edges_norm2) = group.section_edges(&edges_norm);
        assert_eq!(n2, group.neutre(), "section idempotent");
        assert_eq!(edges_norm2, edges_norm);
        // The min = 0 section does start at key 0.
        assert!(edges_norm.contains_key(&0));
        assert_eq!(n1, *edges.keys().next().unwrap() as i64);

        // (v) Homomorphism: f(n₁ ⊕ n₂) = f(n₁)∘f(n₂). Key large
        // enough that the intermediate reads do not saturate to 0.
        let key_big = BOUND + next() % BOUND;
        let a1 = (next() % (2 * BOUND)) as i64 - BOUND as i64;
        let a2 = (next() % (2 * BOUND)) as i64 - BOUND as i64;
        let composed = group.composed(&a1, &a2);
        assert_eq!(
            group.apply(&key_big, &composed),
            group.apply(&group.apply(&key_big, &a1), &a2)
        );

        // (i) Inverse: f(n) then f(n)⁻¹ = identity (key large enough).
        let n = (next() % BOUND) as i64;
        assert_eq!(
            group.apply(&group.apply(&key_big, &n), &group.inverser(&n)),
            key_big
        );

        // (vi) Trivial kernel: f injective on the arguments.
        let n1 = (next() % BOUND) as i64;
        let n2 = (next() % BOUND) as i64;
        if group.apply(&key_big, &n1) == group.apply(&key_big, &n2) {
            assert_eq!(
                n1, n2,
                "f(n₁) = f(n₂) ⇒ n₁ = n₂ (storage modulo the kernel)"
            );
        }

        // Input edges: the absolute read composes the arguments.
        let handle = Handle {
            controller: ControllerId(11),
            anchor: Anchor(0),
            node: NodeId(1),
        };
        let edge1 = EdgeInput::new(a1, handle);
        let edge2 = EdgeInput::new(a2, handle);
        let edge_composed = edge1.composed(&group, &edge2);
        assert_eq!(
            edge_composed.read_key(&group, &key_big),
            edge2.read_key(&group, &edge1.read_key(&group, &key_big))
        );
    }
}
