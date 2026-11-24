//! ALIGN-3 runner: share of the observer dispatch on the intern path.
//! Runs the deterministic LCG load once with the chosen observer
//! (`noop` = `NoOpObserver`, the default; `counting` = a wired observer
//! tallying every event with atomic counters, the lightest real consumer
//! of the trace) and prints one JSON line with the build-phase wall time,
//! allocations (counting global allocator, MEM-18 pattern), the event
//! tallies and the peak RSS. One process per run; the reference numbers
//! are the median of 5 direct `cargo run --release` runs per mode.
//!
//! Usage: `observer_share <noop|counting> [n]` (default n = 20, the
//! BENCH-1 reference load alea-lcg-20: 106 760 nodes, 2^20 - 1 intern
//! calls).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use lisage_algebra::interface::Anchor;
use lisage_bench::confrontation::table_lcg;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes};
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};

/// Counting allocator (MEM-18 pattern): wraps the system allocator and
/// counts allocations and allocated bytes.
struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Wired observer for the `counting` mode: tallies the events emitted on
/// the intern path with atomic counters (no allocation, no I/O) — the
/// lightest real consumer of the trace, standing in for Liseuse's
/// serializer without its I/O cost.
#[derive(Default)]
struct CountingObserver {
    creations: AtomicU64,
    shares: AtomicU64,
    reductions: AtomicU64,
}

impl Observer for CountingObserver {
    fn on_event(&self, event: &TraceEvent) {
        match event {
            TraceEvent::NodeCreated { .. } => {
                self.creations.fetch_add(1, Ordering::Relaxed);
            }
            TraceEvent::NodeShared { .. } => {
                self.shares.fetch_add(1, Ordering::Relaxed);
            }
            TraceEvent::ReductionApplied { .. } => {
                self.reductions.fetch_add(1, Ordering::Relaxed);
            }
            TraceEvent::SpanOpen { .. }
            | TraceEvent::SpanClose { .. }
            | TraceEvent::DiagramBuilt { .. }
            | TraceEvent::Attr { .. }
            | TraceEvent::MemoHit { .. }
            | TraceEvent::MemoMiss { .. }
            | TraceEvent::Gc { .. }
            | TraceEvent::Round { .. }
            | TraceEvent::Histogram { .. }
            | TraceEvent::OpStart { .. }
            | TraceEvent::OpEnd { .. }
            | TraceEvent::VisitEntry { .. }
            | TraceEvent::VisitExit { .. }
            | TraceEvent::MemoConsult { .. }
            | TraceEvent::MemoInsert { .. }
            | TraceEvent::MemoryWrite { .. }
            | TraceEvent::StructureChange { .. }
            | TraceEvent::LocalFire { .. } => {}
        }
    }
}

/// Bridge so the arena can own the observer while the runner keeps a
/// handle on the counters (same pattern as `bench_contention`).
struct Bridge(Arc<CountingObserver>);

impl Observer for Bridge {
    fn on_event(&self, event: &TraceEvent) {
        self.0.on_event(event);
    }
}

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
fn peak_rss_kib() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("reading /proc/self/status");
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.trim().strip_suffix("kB"))
        .and_then(|number| number.trim().parse::<u64>().ok())
        .expect("VmHWM present")
}

fn main() {
    let mode = std::env::args()
        .nth(1)
        .expect("usage: observer_share <noop|counting> [n]");
    assert!(
        mode == "noop" || mode == "counting",
        "mode is noop|counting"
    );
    let n: u32 = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "20".to_string())
        .parse()
        .expect("n is an integer");
    assert!((1..=24).contains(&n), "n in 1..=24");

    // Truth-table generation, outside the measured block (same as
    // profile_intern phase 1).
    let table = table_lcg(n);

    // BDD construction by Shannon expansion (the intern path), with the
    // chosen observer wired into the arena.
    let keys = BddKeys::new(n);
    let counting = Arc::new(CountingObserver::default());
    let observer: Box<dyn Observer> = match mode.as_str() {
        "counting" => Box::new(Bridge(counting.clone())),
        _ => Box::new(NoOpObserver),
    };
    let mut pool = Arena::new(BddController::new(n), observer);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    BYTES_ALLOCATED.store(0, Ordering::Relaxed);
    let start = Instant::now();
    let root = compiler_bdd(&mut pool, Anchor(0), &keys, &table);
    let build_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let allocations_build = ALLOCATIONS.load(Ordering::Relaxed);
    let bytes_build = BYTES_ALLOCATED.load(Ordering::Relaxed);

    let nodes = count_nodes(&pool, root);
    let interns = (1u64 << n) - 1;
    let (creations, shares, reductions) = match mode.as_str() {
        "counting" => (
            counting.creations.load(Ordering::Relaxed),
            counting.shares.load(Ordering::Relaxed),
            counting.reductions.load(Ordering::Relaxed),
        ),
        _ => (0, 0, 0),
    };
    let events = creations + shares + reductions;
    let peak_rss = peak_rss_kib();
    println!(
        "{{\"tool\":\"lisage-observer-share\",\"mode\":\"{mode}\",\"n\":{n},\"nodes\":{nodes},\"interns\":{interns},\"build_time_ms\":{build_time_ms:.3},\"allocations_build\":{allocations_build},\"bytes_build\":{bytes_build},\"events\":{events},\"creations\":{creations},\"shares\":{shares},\"reductions\":{reductions},\"peak_rss_kib\":{peak_rss}}}",
    );
}
