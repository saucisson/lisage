//! Tests of the Reader v1 — step-by-step view (M2-6): JSONL v2
//! serialization of the new events (`Attribute`, `MemoHit`/`MemoMiss`,
//! `Round`), scrubber (state replay at a position), before/after
//! differential with ghosts, and SVG rendering with colored channels.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_core::PolicyMemory;
use lisage_core::arena::Arena;
use lisage_core::fold::Fold;
use lisage_core::observer::{ChannelAttribute, Observer, TraceEvent};
use lisage_core::saturate::{ParametersSaturation, PolicySaturation, saturer};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_core::transduction::Transducer;
use lisage_core::{Anchor, NodeId, Pool};
use lisage_lang::value::{Atom, Value};
use lisage_reader::step_by_step::{
    DiffTrace, Scrubber, diff, legend_channels, render_svg_not_a_not,
};
use lisage_reader::trace::{TRACE_VERSION, serialize_event};

type PoolRef = Arena<ReferenceController>;

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collecteur {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collecteur {
    fn new() -> Self {
        Self::default()
    }
    fn take(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("mutex poisoned").clone()
    }
}

impl Observer for Collecteur {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

fn schema_set() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]))
}

fn set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atoms: &[u64]) -> NodeId {
    compiled(
        pool,
        anchor,
        schema,
        &Value::Set(atoms.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M2-6: the trace schema is versioned v2 and serializes the new
/// step-by-step events.
#[test]
fn test_serialization_v2_events_not_a_not() {
    assert_eq!(TRACE_VERSION, 2);
    let attribute = serialize_event(&TraceEvent::Attribute {
        channel: ChannelAttribute::Inherited,
        node: NodeId(3),
        value: "Transfer(1)".to_string(),
    });
    assert_eq!(
        attribute,
        "{\"v\":2,\"type\":\"Attribute\",\"channel\":\"inherited\",\"node\":3,\"value\":\"Transfer(1)\"}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::MemoHit { node: NodeId(7) }),
        "{\"v\":2,\"type\":\"MemoHit\",\"node\":7}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::MemoMiss { node: NodeId(8) }),
        "{\"v\":2,\"type\":\"MemoMiss\",\"node\":8}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::Round { number: 2 }),
        "{\"v\":2,\"type\":\"Round\",\"number\":2}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::Gc {
            live: 12,
            collected: 5,
            compaction: true,
        }),
        "{\"v\":2,\"type\":\"Gc\",\"live\":12,\"collected\":5,\"compaction\":true}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Memory,
            node: NodeId(1),
            value: "true".to_string(),
        }),
        "{\"v\":2,\"type\":\"Attribute\",\"channel\":\"memory\",\"node\":1,\"value\":\"true\"}"
    );
}

/// M2-6: the runner emits `MemoMiss`/`MemoHit` and the `Attribute` events of
/// the inherited/synthesized channels during an observed transduction.
#[test]
fn test_runner_emits_memo_and_attribute() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collecteur = Collecteur::new();
    let mut runner = Transducer::with_observer(
        Fold::<lisage_core::fold::Naturel>::new(schema.clone()),
        PolicyMemory::No,
        Arc::new(collecteur.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();
    // Re-run to trigger MemoHits.
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();

    let events = collecteur.take();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoMiss { .. })),
        "at least one MemoMiss"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoHit { .. })),
        "at least one MemoHit on the second run"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attribute {
                channel: ChannelAttribute::Inherited,
                ..
            }
        )),
        "inherited channel emitted"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attribute {
                channel: ChannelAttribute::Synthesize,
                ..
            }
        )),
        "synthesized channel emitted"
    );
}

/// Identity transduction (cloneable) for the saturate test.
#[derive(Clone)]
struct Identity;

impl lisage_core::transduction::Transduction for Identity {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();
    fn fingerprint(&self) -> u64 {
        9001
    }
    fn h0(&self) {}
    fn phi_dispatch(&self, _level: lisage_core::Level, h: &(), _edge: u64) {
        let _ = h;
    }
    fn phi_aggregate(
        &self,
        _level: lisage_core::Level,
        _h: &(),
        _edges: &BTreeMap<u64, (Option<lisage_core::Level>, ())>,
    ) {
    }
    fn phi_out<P: lisage_core::Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: lisage_core::transduction::ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                lisage_core::Form::Node {
                    level: ctx.level,
                    edges: ctx.edges.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memory::EffectMemory::new(),
        )
    }
    fn psi<P: lisage_core::Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (
            pool.intern(anchor, lisage_core::Form::Terminal(payload)),
            (),
        )
    }
}

/// M2-6: `saturate` emits one `Round` per iteration when an observer is
/// provided.
#[test]
fn test_saturate_emits_rounds() {
    let schema = schema_set();
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let initial = set(&mut pool, anchor, &schema, &[0]);

    let collecteur = Collecteur::new();
    let levels: Vec<lisage_core::Level> = (0..schema.number_levels())
        .map(lisage_core::Level)
        .collect();
    let policy = PolicySaturation::Static(levels);
    let mut params = ParametersSaturation {
        transduction: &Identity,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(Arc::new(collecteur.clone())),
    };
    let result = saturer(&mut params).unwrap();
    let rounds: Vec<u32> = collecteur
        .take()
        .iter()
        .filter_map(|e| match e {
            TraceEvent::Round { number } => Some(*number),
            _ => None,
        })
        .collect();
    assert_eq!(rounds, vec![1], "identity: 1 round (immediate fixpoint)");
    assert_eq!(result.iterations, 1);
}

/// M2-6: the scrubber replays the state at a given position (nodes,
/// attributes, memo counters, rounds).
#[test]
fn test_scrubber_replay_and_diff() {
    let events = vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: Some(lisage_core::Level(0)),
        },
        TraceEvent::Attribute {
            channel: ChannelAttribute::Inherited,
            node: NodeId(1),
            value: "0".to_string(),
        },
        TraceEvent::MemoMiss { node: NodeId(1) },
        TraceEvent::Round { number: 1 },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: Some(lisage_core::Level(1)),
        },
        TraceEvent::MemoHit { node: NodeId(1) },
        TraceEvent::Round { number: 2 },
    ];
    let mut scrubber = Scrubber::new(events);
    assert_eq!(scrubber.length(), 7);

    scrubber.go_to(3);
    let state3 = scrubber.state();
    assert_eq!(state3.nodes, BTreeSet::from([NodeId(1)]));
    assert_eq!(state3.attributes.len(), 1);
    assert_eq!(state3.memo_misses, 1);
    assert_eq!(state3.memo_hits, 0);
    assert_eq!(state3.rounds, 0, "the Round at index 3 is not replayed yet");
    scrubber.go_to(4);
    assert_eq!(scrubber.state().rounds, 1);

    scrubber.go_to(100); // clamped to the length
    assert_eq!(scrubber.position(), 7);
    let state7 = scrubber.state();
    assert_eq!(state7.nodes, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert_eq!(state7.memo_hits, 1);
    assert_eq!(state7.rounds, 2);

    // Before/after differential from the end: node 2 is a ghost
    // at position 3, no addition.
    let diff = scrubber.diff_toward(3);
    assert!(diff.ajoutes.is_empty());
    assert_eq!(diff.ghosts, BTreeSet::from([NodeId(2)]));
    // Inverse diff: node 2 is a ghost.
    let mut result = Scrubber::new(vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: None,
        },
    ]);
    result.go_to(2);
    let diff_inv = result.diff_toward(1);
    assert!(diff_inv.ajoutes.is_empty());
    assert_eq!(diff_inv.ghosts, BTreeSet::from([NodeId(2)]));
}

/// M2-6: the step-by-step SVG rendering colors the channels, marks ghosts
/// as dashed, draws the timeline with the rounds, and is deterministic.
#[test]
fn test_rendered_svg_not_a_not_channels_and_ghosts() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collecteur = Collecteur::new();
    let mut runner = Transducer::with_observer(
        Fold::<lisage_core::fold::Naturel>::new(schema.clone()),
        PolicyMemory::No,
        Arc::new(collecteur.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();
    let mut events = collecteur.take();
    events.insert(0, TraceEvent::Round { number: 1 });
    // The runner does not observe the pool: we add the NodeCreated events
    // of the input diagram so that the state has nodes.
    let mut stack = vec![root];
    let mut already_seen = BTreeSet::new();
    while let Some(n) = stack.pop() {
        if !already_seen.insert(n) {
            continue;
        }
        events.push(TraceEvent::NodeCreated {
            node: n,
            level: None,
        });
        if let Ok(lisage_core::Form::Node { edges, .. }) = pool.resolve(n) {
            for target in edges.values() {
                stack.push(*target);
            }
        }
    }

    let mut scrubber = Scrubber::new(events);
    scrubber.go_to(scrubber.length());
    let diff_empty = DiffTrace::default();
    let svg = render_svg_not_a_not(&pool, root, &scrubber, &diff_empty);
    // Colored channels present (inherited blue, synthesized orange).
    let legend = legend_channels();
    assert!(
        svg.contains(legend["inherited"]),
        "inherited channel colored"
    );
    assert!(
        svg.contains(legend["synthesized"]),
        "synthesized channel colored"
    );
    // Timeline: round bar in purple + current position.
    assert!(svg.contains(legend["memory"]), "round bar / memory");
    assert!(svg.contains("step by step"), "dashboard in the title");

    // Ghost: diff with an empty state at the start → all present nodes are
    // not in the diff (additions), let's test the reverse: diff toward 0.
    let diff_ghosts = scrubber.diff_toward(0);
    assert!(
        !diff_ghosts.ghosts.is_empty(),
        "present nodes = ghosts when going back to 0"
    );
    let svg_ghosts = render_svg_not_a_not(&pool, root, &scrubber, &diff_ghosts);
    assert!(
        svg_ghosts.contains("stroke-dasharray=\"4 3\""),
        "ghosts dashed"
    );

    // Determinism.
    let svg2 = render_svg_not_a_not(&pool, root, &scrubber, &diff_empty);
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// Scrubber property (deterministic LCG generator, no proptest):
/// the replay is monotone by prefix (the state at k is included in the state
/// at k+1 for nodes and counters) and the diff is antisymmetric
/// (`diff(a→b).ajoutes == diff(b→a).ghosts` and vice versa).
#[test]
fn test_scrubber_properties_prefixed_monotone_and_diff_antisymmetric() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;

    let mut graine = GRAINE_LCG;
    let mut next = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _cas in 0..50 {
        let length = 1 + (next() % 40) as usize;
        let events: Vec<TraceEvent> = (0..length)
            .map(|_| {
                let node = NodeId(1 + (next() % 6) as u32);
                match next() % 5 {
                    0 => TraceEvent::NodeCreated { node, level: None },
                    1 => TraceEvent::Attribute {
                        channel: ChannelAttribute::Inherited,
                        node,
                        value: "x".to_string(),
                    },
                    2 => TraceEvent::MemoHit { node },
                    3 => TraceEvent::MemoMiss { node },
                    _ => TraceEvent::Round {
                        number: 1 + (next() % 3) as u32,
                    },
                }
            })
            .collect();
        let mut scrubber = Scrubber::new(events);
        let mut precedent = scrubber.state();
        for position in 1..=scrubber.length() {
            scrubber.go_to(position);
            let state = scrubber.state();
            assert!(
                precedent.nodes.iter().all(|n| state.nodes.contains(n)),
                "node monotonicity at position {position}"
            );
            assert!(precedent.memo_hits <= state.memo_hits);
            assert!(precedent.memo_misses <= state.memo_misses);
            assert!(precedent.rounds <= state.rounds);
            assert!(precedent.attributes.len() <= state.attributes.len());
            // Antisymmetry of the diff between positions k-1 and k.
            let diff_before = diff(&precedent, &state);
            let diff_back = diff(&state, &precedent);
            assert_eq!(diff_before.ajoutes, diff_back.ghosts);
            assert_eq!(diff_before.ghosts, diff_back.ajoutes);
            precedent = state;
        }
    }
}
