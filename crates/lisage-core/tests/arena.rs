//! Tests de l'arena (M1-1) : R-share (table unique), handles, événements
//! de trace épinglés par un observateur enregistreur.

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

/// Observateur enregistreur : épingle la narration (contrat ADR-0005).
#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<TraceEvent>>,
}

impl Observer for RecordingObserver {
    fn on_event(&self, event: &TraceEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

fn arena_avec_trace() -> (Arena<MockController>, std::sync::Arc<RecordingObserver>) {
    let observateur = std::sync::Arc::new(RecordingObserver::default());
    let copie = observateur.clone();
    struct Pont(std::sync::Arc<RecordingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    (
        Arena::new(MockController { keys: MockKeys }, Box::new(Pont(copie))),
        observateur,
    )
}

#[test]
fn test_r_share_table_unique() {
    let (mut arena, _trace) = arena_avec_trace();
    let anchor = Anchor(0);
    let premier = arena.intern(anchor, Form::Terminal(true));
    let second = arena.intern(anchor, Form::Terminal(true));
    assert_eq!(premier, second, "R-share : même forme ⇒ même nœud");
    assert_eq!(arena.node_count(), 1);
}

#[test]
fn test_formes_distinctes_nodes_distincts() {
    let (mut arena, _trace) = arena_avec_trace();
    let anchor = Anchor(0);
    let vrai_ = arena.intern(anchor, Form::Terminal(true));
    let faux = arena.intern(anchor, Form::Terminal(false));
    assert_ne!(vrai_, faux);
    assert_eq!(arena.node_count(), 2);
}

#[test]
fn test_handles_unicite() {
    let (mut arena, _trace) = arena_avec_trace();
    let anchor = Anchor(3);
    let terminal = arena.terminal(anchor, true);
    let node = arena.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(1, terminal)]),
        },
    );
    let handle_a = arena.handle(node, anchor);
    let handle_b = arena.handle(node, anchor);
    assert_eq!(handle_a, handle_b);
    assert_eq!(handle_a.controller, ControllerId(7));
    assert_eq!(handle_a.anchor, Anchor(3));
}

#[test]
fn test_resolve_erreur_typee() {
    let (arena, _trace) = arena_avec_trace();
    let resultat = arena.resolve(NodeId(99));
    assert_eq!(
        resultat,
        Err(lisage_core::PoolError::UnknownNode(NodeId(99)))
    );
}

/// Algèbre de clés non identité : deux orthographes (0 et 2) ont la même
/// forme normale — sert à prouver que la normalisation est appliquée avant
/// le hachage (sinon C1 serait violé).
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
fn test_normalisation_cles_avant_hachage() {
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
            arcs: BTreeMap::from([(0, terminal)]),
        },
    );
    let via_2 = arena.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(2, terminal)]),
        },
    );
    assert_eq!(
        via_0, via_2,
        "clés 0 et 2, même forme normale ⇒ même nœud (C1)"
    );
}

#[test]
fn test_events_trace_narration() {
    let (mut arena, trace) = arena_avec_trace();
    let anchor = Anchor(0);
    let node = arena.intern(anchor, Form::Terminal(true));
    arena.intern(anchor, Form::Terminal(true));
    let noeud = arena.intern(
        anchor,
        Form::Node {
            level: Level(5),
            arcs: BTreeMap::from([(1, node)]),
        },
    );
    let events = trace.events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        &[
            TraceEvent::NodeCreated { node, level: None },
            TraceEvent::NodeShared { node },
            TraceEvent::NodeCreated {
                node: noeud,
                level: Some(Level(5))
            },
        ]
    );
}

#[test]
fn test_events_reduction_applied() {
    let (mut arena, trace) = arena_avec_trace();
    let anchor = Anchor(0);
    let defaut = arena.terminal(anchor, false); // D_δ = NodeId(0)
    let autre = arena.terminal(anchor, true);
    let niveau = Level(3);
    // R-collapse : tous les arcs mènent à D_δ et δ est explicite ⇒ Collapse(D_δ).
    let cible = arena.intern(
        anchor,
        Form::Node {
            level: niveau,
            arcs: BTreeMap::from([(0, defaut), (1, defaut)]),
        },
    );
    assert_eq!(cible, defaut);
    // R-default : l'arc δ(ℓ) mène à D_δ, il est retiré, la forme est conservée.
    let noeud = arena.intern(
        anchor,
        Form::Node {
            level: niveau,
            arcs: BTreeMap::from([(0, defaut), (1, autre)]),
        },
    );
    assert_ne!(noeud, defaut);
    let events = trace.events.lock().unwrap();
    assert!(
        events.contains(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Collapse,
            level: niveau,
        }),
        "l'événement ReductionApplied{{Collapse}} doit être émis (trace épinglée)"
    );
    assert!(
        events.contains(&TraceEvent::ReductionApplied {
            rule: ReductionRule::Default,
            level: niveau,
        }),
        "l'événement ReductionApplied{{Default}} doit être émis (trace épinglée)"
    );
}

#[test]
fn test_noop_observer_par_defaut() {
    let mut arena: Arena<MockController> =
        Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver));
    let node = arena.terminal(Anchor(0), true);
    assert_eq!(arena.node_count(), 1);
    assert!(matches!(arena.resolve(node), Ok(Form::Terminal(true))));
}
