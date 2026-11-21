//! Liseuse v1 tests — step-by-step view (M2-6): JSONL v2 serialization
//! of the new events (`Attr`, `MemoHit`/`MemoMiss`, `Round`),
//! scrubber (state replay at a position), before/after differential
//! with ghosts, and SVG rendering with colored channels.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_core::MemoryPolicy;
use lisage_core::arena::Arena;
use lisage_core::observer::{ChannelAttr, Observer, TraceEvent, TraceVal};
use lisage_core::pli::Pli;
use lisage_core::saturate::{ParametresSaturation, PolicySaturation, saturer};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::Transducteur;
use lisage_core::{Anchor, NodeId, Pool};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::step_by_step::{
    DiffTrace, Scrubber, diff, legende_channels, rendre_svg_step_by_step,
};
use lisage_liseuse::trace::{TRACE_VERSION, serialiser_evenement};

type PoolRef = Arena<ReferenceController>;

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collector {
    evenements: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Collector {
    fn new() -> Self {
        Self::default()
    }
    fn prendre(&self) -> Vec<TraceEvent> {
        self.evenements.lock().expect("mutex poisoned").clone()
    }
}

impl Observer for Collector {
    fn on_event(&self, event: &TraceEvent) {
        self.evenements
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

fn schema_set() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1), Atom(2)]))
}

fn set(pool: &mut PoolRef, anchor: Anchor, schema: &Schema, atomes: &[u64]) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Set(atomes.iter().map(|a| Value::Atom(Atom(*a))).collect()),
    )
    .unwrap()
}

/// M2-6 / D1-1 / D1-2: the trace schema is versioned v5 and serializes the
/// step-by-step events, each carrying its `"span"` field.
#[test]
fn test_serialisation_v5_evenements_step_by_step() {
    assert_eq!(TRACE_VERSION, 5);
    let attr = serialiser_evenement(&TraceEvent::Attr {
        channel: ChannelAttr::Inherited,
        node: NodeId(3),
        value: TraceVal::texte("Transfert(1)"),
        span: None,
    });
    assert_eq!(
        attr,
        "{\"v\":5,\"type\":\"Attr\",\"span\":null,\"canal\":\"herite\",\"noeud\":3,\"valeur\":\"Transfert(1)\"}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::MemoHit {
            node: NodeId(7),
            span: None
        }),
        "{\"v\":5,\"type\":\"MemoHit\",\"span\":null,\"noeud\":7}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::MemoMiss {
            node: NodeId(8),
            span: None
        }),
        "{\"v\":5,\"type\":\"MemoMiss\",\"span\":null,\"noeud\":8}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::Round {
            numero: 2,
            niveaux_actifs: vec![0, 1],
            niveaux_modifies: vec![1],
            avant: NodeId(4),
            image: NodeId(5),
            apres: NodeId(6),
            croissance: 2,
            span: None
        }),
        "{\"v\":5,\"type\":\"Round\",\"span\":null,\"numero\":2,\"niveaux_actifs\":[0,1],\"niveaux_modifies\":[1],\"avant\":4,\"image\":5,\"apres\":6,\"croissance\":2}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::Gc {
            live: 12,
            collected: 5,
            compaction: true,
            liste_collectes: vec![NodeId(3), NodeId(4)],
            span: None,
        }),
        "{\"v\":5,\"type\":\"Gc\",\"span\":null,\"vivants\":12,\"collectes\":5,\"compaction\":true,\"liste_collectes\":[3,4]}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::Attr {
            channel: ChannelAttr::Memory,
            node: NodeId(1),
            value: TraceVal::texte("true"),
            span: None,
        }),
        "{\"v\":5,\"type\":\"Attr\",\"span\":null,\"canal\":\"memoire\",\"noeud\":1,\"valeur\":\"true\"}"
    );
}

/// M2-6: the runner emits `MemoMiss`/`MemoHit` and the `Attr` of the
/// inherited/synthesized channels during an observed transduction.
#[test]
fn test_runner_emet_memo_and_attr() {
    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collector = Collector::new();
    let mut runner = Transducteur::with_observer(
        Pli::<lisage_core::pli::Naturel>::new(schema.clone()),
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

    let evenements = collector.prendre();
    assert!(
        evenements
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoMiss { .. })),
        "at least one MemoMiss"
    );
    assert!(
        evenements
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoHit { .. })),
        "at least one MemoHit on the second run"
    );
    assert!(
        evenements.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                channel: ChannelAttr::Inherited,
                ..
            }
        )),
        "inherited channel emitted"
    );
    assert!(
        evenements.iter().any(|e| matches!(
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
struct Identite;

impl lisage_core::transduction::Transduction for Identite {
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
        ctx: lisage_core::transduction::ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                lisage_core::Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memory::EffetMemory::new(),
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
fn test_saturate_emet_rounds() {
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
    let policy = PolicySaturation::Statique(levels);
    let mut params = ParametresSaturation {
        transduction: &Identite,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(Arc::new(collector.clone())),
    };
    let result = saturer(&mut params).unwrap();
    let rounds: Vec<u32> = collector
        .prendre()
        .iter()
        .filter_map(|e| match e {
            TraceEvent::Round { numero, .. } => Some(*numero),
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
    let evenements = vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: Some(lisage_core::Level(0)),
            forme: TraceVal::texte("Noeud"),
            span: None,
        },
        TraceEvent::Attr {
            channel: ChannelAttr::Inherited,
            node: NodeId(1),
            value: TraceVal::texte("0"),
            span: None,
        },
        TraceEvent::MemoMiss {
            node: NodeId(1),
            span: None,
        },
        TraceEvent::Round {
            numero: 1,
            niveaux_actifs: Vec::new(),
            niveaux_modifies: Vec::new(),
            avant: NodeId(0),
            image: NodeId(0),
            apres: NodeId(0),
            croissance: 0,
            span: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: Some(lisage_core::Level(1)),
            forme: TraceVal::texte("Noeud"),
            span: None,
        },
        TraceEvent::MemoHit {
            node: NodeId(1),
            span: None,
        },
        TraceEvent::Round {
            numero: 2,
            niveaux_actifs: Vec::new(),
            niveaux_modifies: Vec::new(),
            avant: NodeId(0),
            image: NodeId(0),
            apres: NodeId(0),
            croissance: 0,
            span: None,
        },
    ];
    let mut scrubber = Scrubber::new(evenements);
    assert_eq!(scrubber.longueur(), 7);

    scrubber.aller_a(3);
    let etat3 = scrubber.etat();
    assert_eq!(etat3.nodes, BTreeSet::from([NodeId(1)]));
    assert_eq!(etat3.attributes.len(), 1);
    assert_eq!(etat3.memo_misses, 1);
    assert_eq!(etat3.memo_hits, 0);
    assert_eq!(etat3.rounds, 0, "the Round at index 3 is not replayed yet");
    scrubber.aller_a(4);
    assert_eq!(scrubber.etat().rounds, 1);

    scrubber.aller_a(100); // clamped to the length
    assert_eq!(scrubber.position(), 7);
    let etat7 = scrubber.etat();
    assert_eq!(etat7.nodes, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert_eq!(etat7.memo_hits, 1);
    assert_eq!(etat7.rounds, 2);

    // Before/after differential from the end: node 2 is a ghost
    // at position 3, no addition.
    let diff = scrubber.diff_vers(3);
    assert!(diff.ajoutes.is_empty());
    assert_eq!(diff.ghosts, BTreeSet::from([NodeId(2)]));
    // Inverse diff: node 2 is a ghost.
    let mut retour = Scrubber::new(vec![
        TraceEvent::NodeCreated {
            node: NodeId(1),
            level: None,
            forme: TraceVal::texte("Noeud"),
            span: None,
        },
        TraceEvent::NodeCreated {
            node: NodeId(2),
            level: None,
            forme: TraceVal::texte("Noeud"),
            span: None,
        },
    ]);
    retour.aller_a(2);
    let diff_inv = retour.diff_vers(1);
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
    let mut runner = Transducteur::with_observer(
        Pli::<lisage_core::pli::Naturel>::new(schema.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();
    let mut evenements = collector.prendre();
    evenements.insert(
        0,
        TraceEvent::Round {
            numero: 1,
            niveaux_actifs: Vec::new(),
            niveaux_modifies: Vec::new(),
            avant: NodeId(0),
            image: NodeId(0),
            apres: NodeId(0),
            croissance: 0,
            span: None,
        },
    );
    // The runner does not observe the pool: we add the NodeCreated of the
    // input diagram so that the state has nodes.
    let mut stack = vec![root];
    let mut deja_vu = BTreeSet::new();
    while let Some(n) = stack.pop() {
        if !deja_vu.insert(n) {
            continue;
        }
        evenements.push(TraceEvent::NodeCreated {
            node: n,
            level: None,
            forme: TraceVal::texte("Noeud"),
            span: None,
        });
        if let Ok(lisage_core::Form::Node { arcs, .. }) = pool.resolve(n) {
            for target in arcs.values() {
                stack.push(*target);
            }
        }
    }

    let mut scrubber = Scrubber::new(evenements);
    scrubber.aller_a(scrubber.longueur());
    let diff_empty = DiffTrace::default();
    let svg = rendre_svg_step_by_step(&pool, root, &scrubber, &diff_empty);
    // Colored channels present (inherited blue, synthesized orange).
    let legende = legende_channels();
    assert!(
        svg.contains(legende["inherited"]),
        "inherited channel colored"
    );
    assert!(
        svg.contains(legende["synthesized"]),
        "synthesized channel colored"
    );
    // Timeline: round bar in purple + current position.
    assert!(svg.contains(legende["memory"]), "round bar / memory");
    assert!(svg.contains("step by step"), "dashboard as title");

    // Ghost: diff with an empty state at the start → all the nodes are ghosts
    // not in the diff (additions), let's test the reverse: diff towards 0.
    let diff_ghosts = scrubber.diff_vers(0);
    assert!(
        !diff_ghosts.ghosts.is_empty(),
        "present nodes = ghosts when going back to 0"
    );
    let svg_ghosts = rendre_svg_step_by_step(&pool, root, &scrubber, &diff_ghosts);
    assert!(
        svg_ghosts.contains("stroke-dasharray=\"4 3\""),
        "ghosts dashed"
    );

    // Determinism.
    let svg2 = rendre_svg_step_by_step(&pool, root, &scrubber, &diff_empty);
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// Scrubber property (deterministic LCG generator, no proptest):
/// the replay is monotone by prefix (the state at k is included in the state at
/// k+1 for nodes and counters) and the diff is antisymmetric
/// (`diff(a→b).ajoutes == diff(b→a).ghosts` and vice versa).
#[test]
fn test_scrubber_proprietes_prefixe_monotone_and_diff_antisymetrique() {
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
        let longueur = 1 + (next() % 40) as usize;
        let evenements: Vec<TraceEvent> = (0..longueur)
            .map(|_| {
                let node = NodeId(1 + (next() % 6) as u32);
                match next() % 5 {
                    0 => TraceEvent::NodeCreated {
                        node,
                        level: None,
                        forme: TraceVal::texte("Noeud"),
                        span: None,
                    },
                    1 => TraceEvent::Attr {
                        channel: ChannelAttr::Inherited,
                        node,
                        value: TraceVal::texte("x"),
                        span: None,
                    },
                    2 => TraceEvent::MemoHit { node, span: None },
                    3 => TraceEvent::MemoMiss { node, span: None },
                    _ => TraceEvent::Round {
                        numero: 1 + (next() % 3) as u32,
                        niveaux_actifs: Vec::new(),
                        niveaux_modifies: Vec::new(),
                        avant: NodeId(0),
                        image: NodeId(0),
                        apres: NodeId(0),
                        croissance: 0,
                        span: None,
                    },
                }
            })
            .collect();
        let mut scrubber = Scrubber::new(evenements);
        let mut precedent = scrubber.etat();
        for position in 1..=scrubber.longueur() {
            scrubber.aller_a(position);
            let etat = scrubber.etat();
            assert!(
                precedent.nodes.iter().all(|n| etat.nodes.contains(n)),
                "node monotonicity at position {position}"
            );
            assert!(precedent.memo_hits <= etat.memo_hits);
            assert!(precedent.memo_misses <= etat.memo_misses);
            assert!(precedent.rounds <= etat.rounds);
            assert!(precedent.attributes.len() <= etat.attributes.len());
            // Antisymmetry of the diff between positions k-1 and k.
            let diff_avant = diff(&precedent, &etat);
            let diff_arriere = diff(&etat, &precedent);
            assert_eq!(diff_avant.ajoutes, diff_arriere.ghosts);
            assert_eq!(diff_avant.ghosts, diff_arriere.ajoutes);
            precedent = etat;
        }
    }
}

/// D1-1: an observed transduction run is wrapped in an Op span, each node
/// visit opens a Visite span, and every event carries its span with a
/// stack-disciplined nesting.
#[test]
fn test_spans_run_transduction_op_visite_et_contexte() {
    use lisage_core::observer::SpanKind;

    let schema = schema_set();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = set(&mut pool, anchor, &schema, &[0, 1]);

    let collector = Collector::new();
    let mut runner = Transducteur::with_observer(
        Pli::<lisage_core::pli::Naturel>::new(schema.clone()),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let _ = runner.run(&pool, &mut output, anchor, root).unwrap();

    let evenements = collector.prendre();
    // The run opens with an Op span and closes with its SpanClose.
    let (premier_span, kind_premier) = match &evenements[0] {
        TraceEvent::SpanOpen { span, kind, lane } => {
            assert_eq!(*lane, 0);
            (*span, *kind)
        }
        autre => panic!("first event must be SpanOpen, got {autre:?}"),
    };
    assert_eq!(kind_premier, SpanKind::Op);
    match evenements.last() {
        Some(TraceEvent::SpanClose { span }) => assert_eq!(*span, premier_span),
        autre => panic!("last event must be SpanClose, got {autre:?}"),
    }
    // At least one Visite span was opened.
    assert!(
        evenements.iter().any(|e| matches!(
            e,
            TraceEvent::SpanOpen {
                kind: SpanKind::Visite,
                ..
            }
        )),
        "at least one Visite span"
    );
    // Stack discipline: SpanOpen pushes, SpanClose pops the matching id,
    // and every other event carries the innermost open span.
    let mut pile: Vec<lisage_core::observer::SpanId> = Vec::new();
    for evenement in &evenements {
        match evenement {
            TraceEvent::SpanOpen { span, .. } => pile.push(*span),
            TraceEvent::SpanClose { span } => {
                assert_eq!(
                    pile.pop(),
                    Some(*span),
                    "SpanClose must close the innermost span"
                )
            }
            autre => assert_eq!(
                autre.span(),
                pile.last().copied(),
                "every event carries the innermost open span: {autre:?}"
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
    let policy = PolicySaturation::Statique(levels);
    let mut params = ParametresSaturation {
        transduction: &Identite,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(Arc::new(collector.clone())),
    };
    let _ = saturer(&mut params).unwrap();

    let evenements = collector.prendre();
    let span_round = evenements
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
        evenements.iter().any(|e| matches!(
            e,
            TraceEvent::Round { numero: 1, span, .. } if *span == Some(span_round)
        )),
        "the Round event carries the Round span"
    );
    // The Round span is closed by a matching SpanClose.
    assert!(
        evenements
            .iter()
            .any(|e| matches!(e, TraceEvent::SpanClose { span } if *span == span_round)),
        "the Round span is closed"
    );
}

/// D1-1: golden serialization of the span boundary events (schema v5).
#[test]
fn test_serialisation_span_open_close_v5() {
    use lisage_core::observer::{SpanId, SpanKind};

    assert_eq!(
        serialiser_evenement(&TraceEvent::SpanOpen {
            span: SpanId(9),
            kind: SpanKind::Op,
            lane: 0,
        }),
        "{\"v\":5,\"type\":\"SpanOpen\",\"span\":9,\"kind\":\"op\",\"lane\":0}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::SpanOpen {
            span: SpanId(10),
            kind: SpanKind::Visite,
            lane: 2,
        }),
        "{\"v\":5,\"type\":\"SpanOpen\",\"span\":10,\"kind\":\"visite\",\"lane\":2}"
    );
    assert_eq!(
        serialiser_evenement(&TraceEvent::SpanClose { span: SpanId(9) }),
        "{\"v\":5,\"type\":\"SpanClose\",\"span\":9}"
    );
    // A spanned event renders its span id.
    assert_eq!(
        serialiser_evenement(&TraceEvent::MemoHit {
            node: NodeId(7),
            span: Some(SpanId(10)),
        }),
        "{\"v\":5,\"type\":\"MemoHit\",\"span\":10,\"noeud\":7}"
    );
}
