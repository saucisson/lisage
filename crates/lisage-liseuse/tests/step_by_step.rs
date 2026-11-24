//! Liseuse v1 tests — step-by-step view (M2-6): JSONL v2 serialization
//! of the new events (`Attr`, `MemoHit`/`MemoMiss`, `Round`),
//! scrubber (state replay at a position), before/after differential
//! with ghosts, and SVG rendering with colored channels.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_core::MemoryPolicy;
use lisage_core::arena::Arena;
use lisage_core::fold::Fold;
use lisage_core::observer::{ChannelAttr, Observer, TraceEvent, TraceVal};
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::Transducer;
use lisage_core::{Anchor, NodeId, Pool};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::step_by_step::{
    DiffTrace, Scrubber, diff, legend_channels, render_svg_step_by_step,
};
use lisage_liseuse::trace::{TRACE_VERSION, serialize_event};

type PoolRef = Arena<ReferenceController>;

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collector {
    fn new() -> Self {
        Self::default()
    }
    fn take(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("mutex poisoned").clone()
    }
}

impl Observer for Collector {
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
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atoms.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M2-6 / D1-1 / D1-2: the trace schema is versioned v6 and serializes the
/// step-by-step events, each carrying its `"span"` field.
#[test]
fn test_serialization_v6_events_step_by_step() {
    assert_eq!(TRACE_VERSION, 6);
    let attr = serialize_event(&TraceEvent::Attr {
        channel: ChannelAttr::Inherited,
        node: NodeId(3),
        value: TraceVal::text("Transfert(1)"),
        span: None,
    });
    assert_eq!(
        attr,
        "{\"v\":6,\"type\":\"Attr\",\"span\":null,\"canal\":\"herite\",\"noeud\":3,\"valeur\":\"Transfert(1)\"}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::MemoHit {
            node: NodeId(7),
            span: None
        }),
        "{\"v\":6,\"type\":\"MemoHit\",\"span\":null,\"noeud\":7}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::MemoMiss {
            node: NodeId(8),
            span: None
        }),
        "{\"v\":6,\"type\":\"MemoMiss\",\"span\":null,\"noeud\":8}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::Round {
            number: 2,
            active_levels: vec![0, 1],
            modified_levels: vec![1],
            before: NodeId(4),
            image: NodeId(5),
            after: NodeId(6),
            growth: 2,
            span: None
        }),
        "{\"v\":6,\"type\":\"Round\",\"span\":null,\"numero\":2,\"niveaux_actifs\":[0,1],\"niveaux_modifies\":[1],\"avant\":4,\"image\":5,\"apres\":6,\"croissance\":2}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::Gc {
            live: 12,
            collected: 5,
            compaction: true,
            collected_list: vec![NodeId(3), NodeId(4)],
            span: None,
        }),
        "{\"v\":6,\"type\":\"Gc\",\"span\":null,\"vivants\":12,\"collectes\":5,\"compaction\":true,\"liste_collectes\":[3,4]}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::Attr {
            channel: ChannelAttr::Memory,
            node: NodeId(1),
            value: TraceVal::text("true"),
            span: None,
        }),
        "{\"v\":6,\"type\":\"Attr\",\"span\":null,\"canal\":\"memoire\",\"noeud\":1,\"valeur\":\"true\"}"
    );
}

/// M2-6: the runner emits `MemoMiss`/`MemoHit` and the `Attr` of the
/// inherited/synthesized channels during an observed transduction.
#[test]
fn test_runner_emits_memo_and_attr() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        Fold::<lisage_core::fold::Natural>::new(schema.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();
    // Re-run to trigger MemoHit.
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();

    let events = collector.take();
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
            TraceEvent::Attr {
                channel: ChannelAttr::Inherited,
                ..
            }
        )),
        "inherited channel emitted"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                channel: ChannelAttr::Synthesized,
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
    fn phi_disp(&self, _level: lisage_core::Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(
        &self,
        _level: lisage_core::Level,
        _h: &(),
        _arcs: &BTreeMap<u64, (Option<lisage_core::Level>, ())>,
    ) {
    }
    fn phi_out<P: lisage_core::Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: lisage_core::transduction::PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffectMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                lisage_core::Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
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

    let collector = Collector::new();
    let levels: Vec<lisage_core::Level> = (0..schema.number_levels())
        .map(lisage_core::Level)
        .collect();
    let policy = PolicySaturation::Static(levels);
    let mut params = SaturationParameters {
        transduction: &Identity,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(Arc::new(collector.clone())),
    };
    let result = saturate(&mut params).unwrap();
    let rounds: Vec<u32> = collector
        .take()
        .iter()
        .filter_map(|e| match e {
            TraceEvent::Round { number, .. } => Some(*number),
            _ => None,
        })
        .collect();
    assert_eq!(rounds, vec![1], "identity: 1 round (immediate fixpoint)");
    assert_eq!(result.iterations, 1);
}

/// M2-6: the scrubber replays the state at a given position (nodes, attributes,
/// memo counters, rounds).
#[test]
fn test_scrubber_replay_and_diff() {
    let events = vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: Some(lisage_core::Level(0)),
            form: TraceVal::text("Noeud"),
            span: None,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Inherited,
            node: NodeId(1),
            value: TraceVal::text("0"),
            span: None,
        },
        TraceEvent::MemoMiss {
            node: NodeId(1),
            span: None,
        },
        TraceEvent::Round {
            number: 1,
            active_levels: Vec::new(),
            modified_levels: Vec::new(),
            before: NodeId(0),
            image: NodeId(0),
            after: NodeId(0),
            growth: 0,
            span: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: Some(lisage_core::Level(1)),
            form: TraceVal::text("Noeud"),
            span: None,
        },
        TraceEvent::MemoHit {
            node: NodeId(1),
            span: None,
        },
        TraceEvent::Round {
            number: 2,
            active_levels: Vec::new(),
            modified_levels: Vec::new(),
            before: NodeId(0),
            image: NodeId(0),
            after: NodeId(0),
            growth: 0,
            span: None,
        },
    ];
    let mut scrubber = Scrubber::new(events);
    assert_eq!(scrubber.length(), 7);

    scrubber.round_a(3);
    let state3 = scrubber.state();
    assert_eq!(state3.nodes, BTreeSet::from([NodeId(1)]));
    assert_eq!(state3.attributes.len(), 1);
    assert_eq!(state3.memo_misses, 1);
    assert_eq!(state3.memo_hits, 0);
    assert_eq!(state3.rounds, 0, "the Round at index 3 is not replayed yet");
    scrubber.round_a(4);
    assert_eq!(scrubber.state().rounds, 1);

    scrubber.round_a(100); // clamped to the length
    assert_eq!(scrubber.position(), 7);
    let state7 = scrubber.state();
    assert_eq!(state7.nodes, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert_eq!(state7.memo_hits, 1);
    assert_eq!(state7.rounds, 2);

    // Before/after differential from the end: node 2 is a ghost
    // at position 3, no addition.
    let diff = scrubber.diff_to(3);
    assert!(diff.ajoutes.is_empty());
    assert_eq!(diff.ghosts, BTreeSet::from([NodeId(2)]));
    // Inverse diff: node 2 is a ghost.
    let mut trip = Scrubber::new(vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: None,
            form: TraceVal::text("Noeud"),
            span: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: None,
            form: TraceVal::text("Noeud"),
            span: None,
        },
    ]);
    trip.round_a(2);
    let diff_inv = trip.diff_to(1);
    assert!(diff_inv.ajoutes.is_empty());
    assert_eq!(diff_inv.ghosts, BTreeSet::from([NodeId(2)]));
}

/// M2-6: the step-by-step SVG rendering colors the channels, marks the ghosts
/// as dashed, draws the timeline with the rounds, and is deterministic.
#[test]
fn test_rendu_svg_step_by_step_channels_and_ghosts() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        Fold::<lisage_core::fold::Natural>::new(schema.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();
    let mut events = collector.take();
    events.insert(
        0,
        TraceEvent::Round {
            number: 1,
            active_levels: Vec::new(),
            modified_levels: Vec::new(),
            before: NodeId(0),
            image: NodeId(0),
            after: NodeId(0),
            growth: 0,
            span: None,
        },
    );
    // The runner does not observe the pool: we add the NodeCreated of the
    // input diagram so that the state has nodes.
    let mut stack = vec![root];
    let mut already_seen = BTreeSet::new();
    while let Some(n) = stack.pop() {
        if !already_seen.insert(n) {
            continue;
        }
        events.push(TraceEvent::NodeCreated {
            node: n,
            level: None,
            form: TraceVal::text("Noeud"),
            span: None,
        });
        if let Ok(lisage_core::Form::Node { arcs, .. }) = pool.resolve(n) {
            for target in arcs.values() {
                stack.push(*target);
            }
        }
    }

    let mut scrubber = Scrubber::new(events);
    scrubber.round_a(scrubber.length());
    let diff_empty = DiffTrace::default();
    let svg = render_svg_step_by_step(&pool, root, &scrubber, &diff_empty);
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
    assert!(svg.contains("step by step"), "dashboard as title");

    // Ghost: diff with an empty state at the start → all the nodes are ghosts
    // not in the diff (additions), let's test the reverse: diff towards 0.
    let diff_ghosts = scrubber.diff_to(0);
    assert!(
        !diff_ghosts.ghosts.is_empty(),
        "present nodes = ghosts when going back to 0"
    );
    let svg_ghosts = render_svg_step_by_step(&pool, root, &scrubber, &diff_ghosts);
    assert!(
        svg_ghosts.contains("stroke-dasharray=\"4 3\""),
        "ghosts dashed"
    );

    // Determinism.
    let svg2 = render_svg_step_by_step(&pool, root, &scrubber, &diff_empty);
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// Scrubber property (deterministic LCG generator, no proptest):
/// the replay is monotone by prefix (the state at k is included in the state at
/// k+1 for nodes and counters) and the diff is antisymmetric
/// (`diff(a→b).ajoutes == diff(b→a).ghosts` and vice versa).
#[test]
fn test_scrubber_properties_prefixe_monotone_and_diff_antisymmetric() {
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
        let length = 1 + (next() % 40) as usize;
        let events: Vec<TraceEvent> = (0..length)
            .map(|_| {
                let node = NodeId(1 + (next() % 6) as u32);
                match next() % 5 {
                    0 => TraceEvent::NodeCreated {
                        node,
                        level: None,
                        form: TraceVal::text("Noeud"),
                        span: None,
                    },
                    1 => TraceEvent::Attr {
                        channel: ChannelAttr::Inherited,
                        node,
                        value: TraceVal::text("x"),
                        span: None,
                    },
                    2 => TraceEvent::MemoHit { node, span: None },
                    3 => TraceEvent::MemoMiss { node, span: None },
                    _ => TraceEvent::Round {
                        number: 1 + (next() % 3) as u32,
                        active_levels: Vec::new(),
                        modified_levels: Vec::new(),
                        before: NodeId(0),
                        image: NodeId(0),
                        after: NodeId(0),
                        growth: 0,
                        span: None,
                    },
                }
            })
            .collect();
        let mut scrubber = Scrubber::new(events);
        let mut previous = scrubber.state();
        for position in 1..=scrubber.length() {
            scrubber.round_a(position);
            let state = scrubber.state();
            assert!(
                previous.nodes.iter().all(|n| state.nodes.contains(n)),
                "node monotonicity at position {position}"
            );
            assert!(previous.memo_hits <= state.memo_hits);
            assert!(previous.memo_misses <= state.memo_misses);
            assert!(previous.rounds <= state.rounds);
            assert!(previous.attributes.len() <= state.attributes.len());
            // Antisymmetry of the diff between positions k-1 and k.
            let diff_before = diff(&previous, &state);
            let diff_arriere = diff(&state, &previous);
            assert_eq!(diff_before.ajoutes, diff_arriere.ghosts);
            assert_eq!(diff_before.ghosts, diff_arriere.ajoutes);
            previous = state;
        }
    }
}

/// D1-1: an observed transduction run is wrapped in an Op span, each node
/// visit opens a Visit span, and every event carries its span with a
/// stack-disciplined nesting.
#[test]
fn test_spans_run_transduction_op_visit_and_context() {
    use lisage_core::observer::SpanKind;

    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        Fold::<lisage_core::fold::Natural>::new(schema.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();

    let events = collector.take();
    // The run opens with an Op span and closes with its SpanClose.
    let (first_span, kind_first) = match &events[0] {
        TraceEvent::SpanOpen { span, kind, lane } => {
            assert_eq!(*lane, 0);
            (*span, *kind)
        }
        other => panic!("first event must be SpanOpen, got {other:?}"),
    };
    assert_eq!(kind_first, SpanKind::Op);
    match events.last() {
        Some(TraceEvent::SpanClose { span }) => assert_eq!(*span, first_span),
        other => panic!("last event must be SpanClose, got {other:?}"),
    }
    // At least one Visit span was opened.
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::SpanOpen {
                kind: SpanKind::Visit,
                ..
            }
        )),
        "at least one Visite span"
    );
    // Stack discipline: SpanOpen pushes, SpanClose pops the matching id,
    // and every other event carries the innermost open span.
    let mut pile: Vec<lisage_core::observer::SpanId> = Vec::new();
    for event in &events {
        match event {
            TraceEvent::SpanOpen { span, .. } => pile.push(*span),
            TraceEvent::SpanClose { span } => {
                assert_eq!(
                    pile.pop(),
                    Some(*span),
                    "SpanClose must close the innermost span"
                )
            }
            other => assert_eq!(
                other.span(),
                pile.last().copied(),
                "every event carries the innermost open span: {other:?}"
            ),
        }
    }
    assert!(pile.is_empty(), "all spans closed at the end of the run");
}

/// D1-1: each saturation round opens a Round span and the `Round` event
/// carries it.
#[test]
fn test_spans_saturation_round() {
    use lisage_core::observer::SpanKind;

    let schema = schema_set();
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let initial = set(&mut pool, anchor, &schema, &[0]);

    let collector = Collector::new();
    let levels: Vec<lisage_core::Level> = (0..schema.number_levels())
        .map(lisage_core::Level)
        .collect();
    let policy = PolicySaturation::Static(levels);
    let mut params = SaturationParameters {
        transduction: &Identity,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(Arc::new(collector.clone())),
    };
    let _ = saturate(&mut params).unwrap();

    let events = collector.take();
    let span_round = events
        .iter()
        .find_map(|e| match e {
            TraceEvent::SpanOpen {
                kind: SpanKind::Round,
                span,
                ..
            } => Some(*span),
            _ => None,
        })
        .expect("one Round span per iteration");
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Round { number: 1, span, .. } if *span == Some(span_round)
        )),
        "the Round event carries the Round span"
    );
    // The Round span is closed by a matching SpanClose.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TraceEvent::SpanClose { span } if *span == span_round)),
        "the Round span is closed"
    );
}

/// D1-1: golden serialization of the span boundary events (schema v6).
#[test]
fn test_serialization_span_open_close_v6() {
    use lisage_core::observer::{SpanId, SpanKind};

    assert_eq!(
        serialize_event(&TraceEvent::SpanOpen {
            span: SpanId(9),
            kind: SpanKind::Op,
            lane: 0,
        }),
        "{\"v\":6,\"type\":\"SpanOpen\",\"span\":9,\"kind\":\"op\",\"lane\":0}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::SpanOpen {
            span: SpanId(10),
            kind: SpanKind::Visit,
            lane: 2,
        }),
        "{\"v\":6,\"type\":\"SpanOpen\",\"span\":10,\"kind\":\"visite\",\"lane\":2}"
    );
    assert_eq!(
        serialize_event(&TraceEvent::SpanClose { span: SpanId(9) }),
        "{\"v\":6,\"type\":\"SpanClose\",\"span\":9}"
    );
    // A spanned event renders its span id.
    assert_eq!(
        serialize_event(&TraceEvent::MemoHit {
            node: NodeId(7),
            span: Some(SpanId(10)),
        }),
        "{\"v\":6,\"type\":\"MemoHit\",\"span\":10,\"noeud\":7}"
    );
}
