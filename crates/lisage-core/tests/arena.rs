//! Arena tests (M1-1): R-share (unique table), handles, trace events
//! pinned by a recording observer.

use std::collections::BTreeMap;
use std::sync::Mutex;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::{NoOpObserver, Observer, ReductionRule, TraceEvent};

struct MockKeys;

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        2
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(0)
    }
}

struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(7);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

/// Recording observer: pins the narration (ADR-0005 contract).
#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<TraceEvent>>,
}

impl Observer for RecordingObserver {
    fn on_event(&self, event: &TraceEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

fn arena_with_trace() -> (Arena<MockController>, std::sync::Arc<RecordingObserver>) {
    let observer = std::sync::Arc::new(RecordingObserver::default());
    let copy = observer.clone();
    struct Pont(std::sync::Arc<RecordingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    (
        Arena::new(MockController { keys: MockKeys }, Box::new(Pont(copy))),
        observer,
    )
}

#[test]
fn test_r_share_table_unique() {
    let (mut arena, _trace) = arena_with_trace();
    let anchor = Anchor(0);
    let first = arena.intern(anchor, Form::Terminal(true));
    let second = arena.intern(anchor, Form::Terminal(true));
    assert_eq!(first, second, "R-share: same form ⇒ same node");
    assert_eq!(arena.node_count(), 1);
}

#[test]
fn test_forms_distinct_nodes_distinct() {
    let (mut arena, _trace) = arena_with_trace();
    let anchor = Anchor(0);
    let true_node = arena.intern(anchor, Form::Terminal(true));
    let false_node = arena.intern(anchor, Form::Terminal(false));
    assert_ne!(true_node, false_node);
    assert_eq!(arena.node_count(), 2);
}

#[test]
fn test_handles_uniqueness() {
    let (mut arena, _trace) = arena_with_trace();
    let anchor = Anchor(3);
    let terminal = arena.terminal(anchor, true);
    let node = arena.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(1, terminal)]),
        },
    );
    let handle_a = arena.handle(node, anchor);
    let handle_b = arena.handle(node, anchor);
    assert_eq!(handle_a, handle_b);
    assert_eq!(handle_a.controller, ControllerId(7));
    assert_eq!(handle_a.anchor, Anchor(3));
}

#[test]
fn test_resolve_error_typed() {
    let (arena, _trace) = arena_with_trace();
    let result = arena.resolve(NodeId(99));
    assert_eq!(result, Err(lisage_core::PoolError::UnknownNode(NodeId(99))));
}

/// Non-identity key algebra: two spellings (0 and 2) have the same
/// normal form — used to prove that normalization is applied before
/// hashing (otherwise C1 would be violated).
struct NormalizingKeys;

impl KeyAlgebra for NormalizingKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        key % 2
    }
    fn cardinality(&self, _level: Level) -> u64 {
        2
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        Some(0)
    }
}

struct NormalizingController {
    keys: NormalizingKeys,
}

impl Controller for NormalizingController {
    type Key = u64;
    type Payload = bool;
    type Keys = NormalizingKeys;
    const FINGERPRINT: ControllerId = ControllerId(8);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
}

#[test]
fn test_normalisation_keys_before_hashing() {
    let mut arena: Arena<NormalizingController> = Arena::new(
        NormalizingController {
            keys: NormalizingKeys,
        },
        Box::new(NoOpObserver),
    );
    let anchor = Anchor(0);
    let terminal = arena.terminal(anchor, true);
    let via_0 = arena.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(0, terminal)]),
        },
    );
    let via_2 = arena.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(2, terminal)]),
        },
    );
    assert_eq!(
        via_0, via_2,
        "keys 0 and 2, same normal form ⇒ same node (C1)"
    );
}

#[test]
fn test_events_trace_narration() {
    let (mut arena, trace) = arena_with_trace();
    let anchor = Anchor(0);
    let node = arena.intern(anchor, Form::Terminal(true));
    arena.intern(anchor, Form::Terminal(true));
    let node_form = arena.intern(
        anchor,
        Form::Node {
            level: Level(5),
            edges: BTreeMap::from([(1, node)]),
        },
    );
    let events = trace.events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        &[
            TraceEvent::NodeCreated { node, level: None },
            TraceEvent::NodeShared { node },
            TraceEvent::NodeCreated {
                node: node_form,
                level: Some(Level(5))
            },
        ]
    );
}

#[test]
fn test_events_reduction_applied() {
    let (mut arena, trace) = arena_with_trace();
    let anchor = Anchor(0);
    let default = arena.terminal(anchor, false); // D_δ = NodeId(0)
    let other = arena.terminal(anchor, true);
    let level = Level(3);
    // R-collapse: all edges lead to D_δ and δ is explicit ⇒ Collapse(D_δ).
    let target = arena.intern(
        anchor,
        Form::Node {
            level,
            edges: BTreeMap::from([(0, default), (1, default)]),
        },
    );
    assert_eq!(target, default);
    // R-default: the δ(ℓ) edge leads to D_δ, it is removed, the form is kept.
    let node = arena.intern(
        anchor,
        Form::Node {
            level,
            edges: BTreeMap::from([(0, default), (1, other)]),
        },
    );
    assert_ne!(node, default);
    let events = trace.events.lock().unwrap();
    assert!(
        events.contains(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level,
        }),
        "the ReductionApplied{{Collapse}} event must be emitted (pinned trace)"
    );
    assert!(
        events.contains(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Default,
            level,
        }),
        "the ReductionApplied{{Default}} event must be emitted (pinned trace)"
    );
}

#[test]
fn test_noop_observer_by_default() {
    let mut arena: Arena<MockController> =
        Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver));
    let node = arena.terminal(Anchor(0), true);
    assert_eq!(arena.node_count(), 1);
    assert!(matches!(arena.resolve(node), Ok(Form::Terminal(true))));
}
