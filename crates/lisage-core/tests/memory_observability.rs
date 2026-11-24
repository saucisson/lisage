//! Memory observability tests (ADR-0109 deferred tracks, trace v7):
//! every memory read of `φ_out` is traced (`MemoryRead`, via the probe),
//! every op closes with the level-memory state (`MemoryState`, the
//! `(0, 0)` witness of a nil memory `M = ()`), and the synthesized
//! channel is emitted at every completed visit (miss, terminal, hit).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, MemoryPolicy, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::memory::EffectMemory;
use lisage_core::observer::{ChannelAttr, NoOpObserver, Observer, TraceEvent, TraceVal};
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{PhiOutContext, Transducer, Transduction};
use lisage_lang::value::Atom;

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Trace collector (in emission order).
#[derive(Clone, Default)]
struct Collector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Observer for Collector {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

impl Collector {
    /// Snapshot of the collected events.
    fn take(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("mutex poisoned").clone()
    }
}

/// Transduction with a level memory (`M = u64`): every `φ_out` first
/// reads the memory at `(node, level)` then writes `h` back, so a
/// second run on the same runner reads a stored value.
struct ReadsMemory;

impl Transduction for ReadsMemory {
    type H = u64;
    type S = u64;
    type Out = bool;
    type M = u64;

    fn fingerprint(&self) -> u64 {
        3101
    }
    fn h0(&self) -> u64 {
        0
    }
    fn phi_disp(&self, _level: Level, h: &u64, _arc: u64) -> u64 {
        h + 1
    }
    fn phi_aggr(&self, _level: Level, _h: &u64, arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).max().unwrap_or(0)
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<u64>) {
        // The observed read: goes through the runner's probe, which
        // emits the `MemoryRead` event.
        let read = ctx.memory.read(ctx.node_input, ctx.level);
        let effect = EffectMemory::one(ctx.node_input, ctx.level, read.copied().unwrap_or(*ctx.h));
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            effect,
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), 0)
    }
}

/// Identity transduction with a nil memory (`M = ()`).
#[derive(Clone)]
struct IdentityNilMemory;

impl Transduction for IdentityNilMemory {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        3201
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            EffectMemory::new(),
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

/// Two-level input: `b` (level 1) over `a` (level 0) and `true`, `a`
/// over `true` and `false`.
fn input_two_levels(input: &mut PoolRef) -> (NodeId, NodeId) {
    let anchor = Anchor(0);
    let true_node = input.intern(anchor, Form::Terminal(true));
    let false_node = input.intern(anchor, Form::Terminal(false));
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: ArcRun::from_pairs([(0, true_node), (1, false_node)]),
        },
    );
    let b = input.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: ArcRun::from_pairs([(0, a), (1, true_node)]),
        },
    );
    (a, b)
}

/// ADR-0109 track (v7): every memory read of `φ_out` is traced by the
/// probe — miss first (value elided), hit on the second run of the
/// same runner (stored value rendered), always inside a visit span.
#[test]
fn test_memory_read_traced_on_miss_and_hit() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let (a, b) = input_two_levels(&mut input);

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        ReadsMemory,
        MemoryPolicy::PerLevel,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, b).unwrap();

    // First pass: every internal visit reads before any write of its
    // own, but after the parent — `b` is visited before `a`, so `b`
    // misses; `a` is fresh too. Keys traced with the visited node and
    // level, miss flag, elided value, inside a visit span.
    let first = collector.take();
    let misses: Vec<&TraceEvent> = first
        .iter()
        .filter(|e| matches!(e, TraceEvent::MemoryRead { .. }))
        .collect();
    assert!(
        misses.len() >= 2,
        "one read per internal visit, got {}",
        misses.len()
    );
    for event in &misses {
        assert!(matches!(
            event,
            TraceEvent::MemoryRead {
                hit: false,
                value: TraceVal::Elided,
                span: Some(_),
                ..
            }
        ));
    }
    let keys: Vec<(u32, u32)> = misses
        .iter()
        .map(|e| match e {
            TraceEvent::MemoryRead { node, level, .. } => (node.0, *level),
            _ => unreachable!(),
        })
        .collect();
    assert!(
        keys.contains(&(b.0, 1)) && keys.contains(&(a.0, 0)),
        "the reads carry the visited (node, level), got {keys:?}"
    );

    // Second pass on the same runner: the memory retains the first
    // pass, so every read hits and renders its stored value.
    runner.run(&input, &mut output, anchor, b).unwrap();
    let second = collector.take();
    let hits: Vec<&TraceEvent> = second
        .iter()
        .filter(|e| matches!(e, TraceEvent::MemoryRead { .. }))
        .collect();
    assert_eq!(
        hits.len(),
        misses.len(),
        "same internal visits on the second pass"
    );
    for event in &hits {
        assert!(matches!(
            event,
            TraceEvent::MemoryRead {
                hit: true,
                value: TraceVal::Integer(_),
                span: Some(_),
                ..
            }
        ));
    }
}

/// ADR-0109 track (v7): a nil memory (`M = ()`) yields exactly one
/// `MemoryState (0, 0)` per op, emitted immediately before `OpEnd` —
/// the traceable witness that the computed table never stores.
#[test]
fn test_memory_state_nil_memory_witness() {
    let mut pool = pool();
    let (_a, b) = input_two_levels(&mut pool);
    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        IdentityNilMemory,
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    runner.run_same_pool(&mut pool, Anchor(0), b).unwrap();
    runner.run_same_pool(&mut pool, Anchor(0), b).unwrap();

    let events = collector.take();
    let op_starts = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::OpStart { .. }))
        .count();
    assert_eq!(op_starts, 2, "one op per run");
    let states: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, TraceEvent::MemoryState { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(states.len(), 2, "one MemoryState per op");
    for index in states {
        assert!(
            matches!(
                events[index],
                TraceEvent::MemoryState {
                    version: 0,
                    entries: 0,
                    ..
                }
            ),
            "the nil memory keeps (0, 0) at every op"
        );
        assert!(
            matches!(events[index + 1], TraceEvent::OpEnd { .. }),
            "MemoryState is emitted immediately before OpEnd"
        );
    }
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoryRead { .. })),
        "a nil memory never reads"
    );
}

/// ADR-0109 track (v7): a memory `M ≠ ()` shows its effective writes
/// in the per-op `MemoryState` — the version grows across the ops and
/// the state matches the runner's memory at the end of each op.
#[test]
fn test_memory_state_tracks_writes() {
    let mut input = pool();
    let mut output = pool();
    let anchor = Anchor(0);
    let (a, b) = input_two_levels(&mut input);

    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        ReadsMemory,
        MemoryPolicy::PerLevel,
        Arc::new(collector.clone()),
    );
    // First op: `b` then `a` write — two distinct keys.
    runner.run(&input, &mut output, anchor, b).unwrap();
    let version_first = runner.version_memory();
    assert_eq!(version_first, 2, "one bump per effective write");
    let events = collector.take();
    let state_matches = events
        .iter()
        .rev()
        .find_map(|e| match e {
            TraceEvent::MemoryState { version, entries, .. } => Some((*version, *entries)),
            _ => None,
        })
        .expect("a MemoryState was emitted");
    assert_eq!(state_matches, (2, 2), "two entries stored at two keys");

    // Second op on the same runner: same keys rewritten, version
    // bumped again, entries unchanged.
    runner.run(&input, &mut output, anchor, b).unwrap();
    assert_eq!(runner.version_memory(), 4, "the version grows per op");
    let events = collector.take();
    let state_matches = events
        .iter()
        .rev()
        .find_map(|e| match e {
            TraceEvent::MemoryState { version, entries, .. } => Some((*version, *entries)),
            _ => None,
        })
        .expect("a MemoryState was emitted");
    assert_eq!(state_matches, (4, 2), "same keys, bumped version");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoryWrite { .. })),
        "the writes are traced alongside"
    );
}

/// ADR-0109 track (S channel): every completed visit emits its
/// synthesized `Attr` — internal misses (after `φ_aggr`), terminal
/// visits (after `ψ`) and memo hits (re-emitted so the parent
/// aggregates an input the trace has shown). Two runs on the same
/// runner cover the three visit kinds, so the invariant is
/// `count(VisitExit) == count(synthesized Attr)`.
#[test]
fn test_s_channel_at_every_completed_visit() {
    let mut pool = pool();
    let (_a, b) = input_two_levels(&mut pool);
    let collector = Collector::new();
    let mut runner = Transducer::with_observer(
        IdentityNilMemory,
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    // Run 1: internal misses and terminal visits. Run 2: memo hits.
    runner.run_same_pool(&mut pool, Anchor(0), b).unwrap();
    runner.run_same_pool(&mut pool, Anchor(0), b).unwrap();

    let events = collector.take();
    let visit_exits = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::VisitExit { .. }))
        .count();
    let synthesized = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    ..
                }
            )
        })
        .count();
    assert!(visit_exits > 0, "the scenario visits nodes");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoMiss { .. })),
        "the first run misses"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, TraceEvent::MemoHit { .. })),
        "the second run hits"
    );
    assert_eq!(
        synthesized, visit_exits,
        "one synthesized Attr per completed visit (miss, terminal, hit)"
    );
    // Every synthesized Attr sits inside its visit span.
    let with_span = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    ..
                }
            )
        })
        .filter(|e| matches!(e, TraceEvent::Attr { span: Some(_), .. }))
        .count();
    assert_eq!(with_span, synthesized, "the synthesized Attr carries its visit span");
}
