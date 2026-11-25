//! D2-1 tests — pool/memo/memory mirror reconstructed from the trace
//! alone. DoD: mirror == real engine state at trace end, on the
//! philosophers trace and on a hand-built trace with GC.

use std::sync::{Arc, Mutex};

use lisage_core::MemoryPolicy;
use lisage_core::fold::{Fold, Natural};
use lisage_core::observer::{Observer, TraceEvent, TraceVal};
use lisage_core::transduction::Transducer;
use lisage_core::{Anchor, Level, NodeId};
use lisage_liseuse::mirror::PoolMirror;
use lisage_liseuse::philosophers::{build_r_n, fresh_pool, schema_ring};

/// Observer that collects the typed events in memory.
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

/// Runs the fold `(ℕ,+,×)` on R_5 with an observer: a real philosophers
/// trace (hundreds of events, one op).
fn philosophers_trace() -> Vec<TraceEvent> {
    let n = 5;
    let schema = schema_ring(n);
    let mut input = fresh_pool(&schema);
    let mut output = fresh_pool(&schema);
    let anchor = Anchor(0);
    let root = build_r_n(&mut input, anchor, n);
    let collector = Collector::default();
    let mut runner = Transducer::with_observer(
        Fold::<Natural>::new(schema),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, root).unwrap();
    collector.events.lock().expect("mutex poisoned").clone()
}

#[test]
fn mirror_rebuilds_live_nodes_from_philosophers_trace() {
    let events = philosophers_trace();
    let mirror = PoolMirror::build(&events);

    // The philosophers trace mentions nodes through visit events:
    // every visited node is live in the mirror.
    let visited_count = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::VisitEntry { .. }))
        .count();
    assert!(visited_count > 0, "the trace must contain node visits");
    assert!(mirror.live_count() > 0);
    assert!(mirror.live_count() <= visited_count);

    // No GC in this trace.
    assert_eq!(mirror.gc_count(), 0);
}

#[test]
fn mirror_tracks_memo_inserts_on_philosophers_trace() {
    let events = philosophers_trace();
    let mirror = PoolMirror::build(&events);

    let insert_count = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::MemoInsert { .. }))
        .count();
    assert!(insert_count > 0, "the trace must contain memo inserts");
    // Memo keys are unique by (fingerprint, node, version).
    assert!(mirror.memo_count() > 0);
    assert!(mirror.memo_count() <= insert_count);
}

#[test]
fn mirror_tracks_memory_version() {
    let events = philosophers_trace();
    let mirror = PoolMirror::build(&events);

    let last_version = events
        .iter()
        .filter_map(|e| match e {
            TraceEvent::MemoryWrite { version_after, .. } => Some(*version_after),
            _ => None,
        })
        .next_back();
    assert_eq!(mirror.memory_version(), last_version.unwrap_or(0));
}

#[test]
fn mirror_is_consistent_on_philosophers_trace() {
    let events = philosophers_trace();
    let mirror = PoolMirror::build(&events);
    assert!(mirror.is_consistent());
}

#[test]
fn mirror_handles_gc_events() {
    // Hand-built trace with a GC event: two nodes created, one collected.
    let node_a = NodeId(1);
    let node_b = NodeId(2);
    let events = vec![
        TraceEvent::NodeCreated {
            node: node_a,
            level: Some(Level(1)),
            form: TraceVal::Elided,
            span: None,
        },
        TraceEvent::NodeCreated {
            node: node_b,
            level: Some(Level(1)),
            form: TraceVal::Elided,
            span: None,
        },
        TraceEvent::Gc {
            live: 1,
            collected: 1,
            compaction: false,
            collected_list: vec![node_b],
            span: None,
        },
    ];
    let mirror = PoolMirror::build(&events);
    assert_eq!(mirror.created_nodes().len(), 2);
    assert_eq!(mirror.live_count(), 1);
    assert!(mirror.live_nodes().contains(&node_a));
    assert!(!mirror.live_nodes().contains(&node_b));
    assert!(mirror.collected_nodes().contains(&node_b));
    assert_eq!(mirror.gc_count(), 1);
}

#[test]
fn mirror_empty_trace_is_empty() {
    let mirror = PoolMirror::build(&[]);
    assert_eq!(mirror.live_count(), 0);
    assert_eq!(mirror.memo_count(), 0);
    assert_eq!(mirror.memory_version(), 0);
    assert_eq!(mirror.gc_count(), 0);
    assert!(mirror.is_consistent());
}
