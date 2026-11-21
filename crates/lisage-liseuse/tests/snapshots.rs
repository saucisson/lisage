//! D2-2 tests — snapshots and O(log n) seek over a trace.
//! DoD: seek answers fast (binary search + tail replay), existing
//! Scrubber behavior unchanged.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use lisage_core::MemoryPolicy;
use lisage_core::observer::{Observer, TraceEvent};
use lisage_core::pli::{Naturel, Pli};
use lisage_core::transduction::Transducteur;
use lisage_core::{Anchor, NodeId};
use lisage_liseuse::philosophers::{build_r_n, pool_neuf, schema_anneau};
use lisage_liseuse::snapshots::{DEFAULT_SNAPSHOT_BUDGET, SnapshotScrubber};
use lisage_liseuse::step_by_step::Scrubber;

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

/// Runs the fold `(ℕ,+,×)` on R_5 with an observer.
fn philosophers_trace() -> Vec<TraceEvent> {
    let n = 5;
    let schema = schema_anneau(n);
    let mut input = pool_neuf(&schema);
    let mut output = pool_neuf(&schema);
    let anchor = Anchor(0);
    let root = build_r_n(&mut input, anchor, n);
    let collector = Collector::default();
    let mut runner = Transducteur::with_observer(
        Pli::<Naturel>::new(schema),
        MemoryPolicy::NoMemory,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, root).unwrap();
    collector.events.lock().expect("mutex poisoned").clone()
}

#[test]
fn snapshots_are_built_at_budget_boundaries() {
    let events = philosophers_trace();
    let scrubber = SnapshotScrubber::with_budget(events.clone(), 100);
    // With 879 events and budget 100: snapshots at 0, 100, 200, ..., 800.
    assert!(scrubber.snapshot_count() >= 8);
    assert!(scrubber.snapshot_count() <= 10);
}

#[test]
fn seek_matches_legacy_scrubber_state() {
    let events = philosophers_trace();
    let mut snapshot_scrubber = SnapshotScrubber::with_budget(events.clone(), 50);
    let mut legacy = Scrubber::new(events.clone());

    for position in [0, 10, 100, 250, 500, events.len() - 1, events.len()] {
        snapshot_scrubber.seek(position);
        legacy.aller_a(position);
        assert_eq!(
            snapshot_scrubber.etat(),
            legacy.etat(),
            "snapshot seek must match legacy replay at position {position}"
        );
    }
}

#[test]
fn seek_is_fast_on_large_trace() {
    // Build a synthetic large trace (100k events) to verify seek < 50 ms.
    let mut events: Vec<TraceEvent> = Vec::with_capacity(100_000);
    for i in 0..100_000u32 {
        events.push(TraceEvent::NodeShared {
            node: NodeId(i),
            span: None,
        });
    }
    let mut scrubber = SnapshotScrubber::with_budget(events, DEFAULT_SNAPSHOT_BUDGET);
    let start = Instant::now();
    scrubber.seek(99_999);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 50,
        "seek on 100k events took {} ms (budget 50 ms)",
        elapsed.as_millis()
    );
}

#[test]
fn seek_clamps_to_trace_length() {
    let events = philosophers_trace();
    let mut scrubber = SnapshotScrubber::new(events.clone());
    scrubber.seek(events.len() + 1000);
    assert_eq!(scrubber.position(), events.len());
}

#[test]
fn empty_trace_seek_is_safe() {
    let mut scrubber = SnapshotScrubber::new(vec![]);
    scrubber.seek(0);
    assert_eq!(scrubber.position(), 0);
    assert!(scrubber.etat().nodes.is_empty());
}

#[test]
fn legacy_scrubber_unchanged() {
    // The existing Scrubber behavior is not modified by D2-2.
    let events = philosophers_trace();
    let mut legacy = Scrubber::new(events.clone());
    legacy.aller_a(100);
    assert_eq!(legacy.position(), 100);
    assert_eq!(legacy.longueur(), events.len());
}
