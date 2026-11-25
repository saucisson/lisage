//! PERF-1 runner (issue #280): phase-level profile of the `alea-lcg`
//! intern path. Runs the deterministic LCG load once and prints one
//! JSON line with, per phase (table generation / BDD construction / node
//! counting): wall time in ms and allocations (counting global allocator,
//! MEM-18 pattern). Also reports the interning count of the bottom-up
//! Shannon expansion (2^n - 1 intern calls), the node count, the
//! allocations per interning, and the peak RSS.
//!
//! Usage: `profile_intern [n]` (default n = 20, the BENCH-1 reference
//! load alea-lcg-20: 106 760 nodes).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use lisage_algebra::interface::Anchor;
use lisage_bench::confrontation::table_lcg;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes};
use lisage_core::observer::NoOpObserver;

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

fn reset_compteurs() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    BYTES_ALLOCATED.store(0, Ordering::Relaxed);
}

fn allocations() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

fn bytes() -> u64 {
    BYTES_ALLOCATED.load(Ordering::Relaxed)
}

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
fn rss_max_ko() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("reading /proc/self/status");
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|remaining| remaining.trim().strip_suffix("kB"))
        .and_then(|count| count.trim().parse::<u64>().ok())
        .expect("VmHWM present")
}

/// One measured phase: wall time and allocations between two resets.
struct Phase {
    temps_ms: f64,
    allocations: u64,
    bytes: u64,
}

fn measure(phase: impl FnOnce()) -> Phase {
    reset_compteurs();
    let start = Instant::now();
    phase();
    Phase {
        temps_ms: start.elapsed().as_secs_f64() * 1000.0,
        allocations: allocations(),
        bytes: bytes(),
    }
}

fn main() {
    let n: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "20".to_string())
        .parse()
        .expect("n is an integer");
    assert!((1..=24).contains(&n), "n in 1..=24");

    // Phase 1: truth-table generation.
    let mut table = Vec::new();
    let phase_table = measure(|| table = table_lcg(n));
    let verifies = table.iter().filter(|value| **value).count() as u64;

    // Phase 2: BDD construction by Shannon expansion (the intern path).
    let keys = BddKeys::new(n);
    let mut pool = Arena::new(BddController::new(n), Box::new(NoOpObserver));
    let mut root = lisage_algebra::interface::NodeId(0);
    let phase_build = measure(|| root = compiler_bdd(&mut pool, Anchor(0), &keys, &table));

    // Phase 3: node counting (verification).
    let mut nodes = 0u64;
    let phase_count = measure(|| nodes = count_nodes(&pool, root));

    // The bottom-up expansion interns once per pair combined:
    // sum_{level=0}^{n-1} 2^level = 2^n - 1 intern calls.
    let interning = (1u64 << n) - 1;
    let allocs_build = phase_build.allocations;
    let rss = rss_max_ko();
    println!(
        "{{\"outil\":\"lisage-profile\",\"n\":{n},\"noeuds\":{nodes},\"verifie\":{verifies},\"internements\":{interning},\"temps_table_ms\":{:.3},\"temps_build_ms\":{:.3},\"temps_count_ms\":{:.3},\"allocs_table\":{},\"allocs_build\":{},\"allocs_count\":{},\"octets_build\":{},\"allocs_par_internement\":{:.3},\"rss_max_ko\":{rss}}}",
        phase_table.temps_ms,
        phase_build.temps_ms,
        phase_count.temps_ms,
        phase_table.allocations,
        allocs_build,
        phase_count.allocations,
        phase_build.bytes,
        allocs_build as f64 / interning as f64,
    );
}
