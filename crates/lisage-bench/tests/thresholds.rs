//! MEM-18: non-regression thresholds (ADR-0012 anti-complacency) —
//! end-to-end bytes per node (arena + tables) and allocations per
//! interning as CI gates, on a deterministic workload built with
//! reproducible randomized IDs (so that a layout on/off comparison never
//! captures an artifact of the hash functions on sequential IDs).
//!
//! The thresholds are frozen constants: they may only be lowered (a
//! regression turns the gate red), and they are frozen for good after
//! milestone MEM-C. A gate must fail when the behavior regresses —
//! verified by a deliberately regressive witness commit, then reverted.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::bdd::BddController;
use lisage_core::observer::NoOpObserver;

/// Frozen threshold: end-to-end bytes per node on the BDD workload
/// (unique table + class arenas + shadow index over live nodes).
/// Measured 60.75 at MEM-18; frozen at 66.0. Freeze after MEM-C.
const SEUIL_OCTETS_PAR_NOEUD: f64 = 66.0;
/// Frozen threshold: allocations per interning on the seeded BDD
/// workload. Measured ~9.2 at MEM-18; frozen at 10.0. Freeze after MEM-C.
const SEUIL_ALLOCATIONS_PAR_INTERNEMENT: f64 = 10.0;
/// Number of interned nodes in the threshold workload.
const NB_INTERNEMENTS: u32 = 2000;
/// Fixed seed of the randomized IDs: reproducible across runs and
/// machines (xorshift64*), so the measurement is deterministic.
const SEED_IDS: u64 = 0x9E37_79B9_7F4A_7C15;

/// Counting allocator (MEM-18): wraps the system allocator and counts
/// allocations and allocated bytes, so the allocation threshold is a
/// real measurement, not an estimate.
struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static OCTETS_ALLOUES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        OCTETS_ALLOUES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        OCTETS_ALLOUES.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Serializes the tests of this binary: the counting allocator is global
/// to the test binary, so measurements must not run concurrently (under
/// nextest each test already runs in its own process).
static MESURE_LOCK: Mutex<()> = Mutex::new(());

/// Reset the allocation counters (call right before the measured loop).
fn reset_compteurs() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    OCTETS_ALLOUES.store(0, Ordering::Relaxed);
}

/// Allocations since the last reset.
fn allocations() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// Reproducible randomized IDs (MEM-18): xorshift64* with a fixed seed —
/// no external dependency (escalation list), deterministic across runs,
/// so a layout on/off comparison measures the layout, not the hash
/// functions fed with sequential IDs.
struct IdsAleatoires {
    state: u64,
}

impl IdsAleatoires {
    fn new(seed: u64) -> Self {
        IdsAleatoires { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Random key of the BDD domain {0, 1}.
    fn next_key(&mut self) -> u64 {
        self.next_u64() % 2
    }
    /// Random arc key of the BDD domain {0, 1}, paired with its
    /// complement so the node never R-collapses.
    fn next_arcs(&mut self, t0: NodeId, t1: NodeId) -> ArcRun<u64> {
        let k0 = self.next_key();
        ArcRun::from_pairs([(k0, t0), (1 - k0, t1)])
    }
}

fn pool_bdd() -> Arena<BddController> {
    Arena::new(BddController::new(64), Box::new(NoOpObserver))
}

/// Deterministic seeded workload: `count` distinct degree-2 nodes (one
/// per level) whose arc keys come from the reproducible randomized IDs —
/// the hash functions see randomized keys, not a sequential artifact.
fn workload_aleatoire(pool: &mut Arena<BddController>, count: u32) -> Vec<NodeId> {
    let t0 = pool.terminal(Anchor(0), false);
    let t1 = pool.terminal(Anchor(0), true);
    let mut ids = IdsAleatoires::new(SEED_IDS);
    (0..count)
        .map(|i| {
            pool.intern(
                Anchor(0),
                Form::Node {
                    level: Level(i),
                    arcs: ids.next_arcs(t0, t1),
                },
            )
        })
        .collect()
}

/// Threshold gate: end-to-end bytes per node on the seeded BDD workload.
/// Turns red on any regression of the arena + tables footprint.
#[test]
fn test_seuil_octets_par_noeud() {
    let _guard = MESURE_LOCK.lock().unwrap();
    let mut pool = pool_bdd();
    let nodes = workload_aleatoire(&mut pool, NB_INTERNEMENTS);
    assert_eq!(pool.live(), nodes.len() + 2, "nodes + 2 terminals");
    let bytes_par_noeud = pool.bytes_per_node();
    assert!(
        bytes_par_noeud <= SEUIL_OCTETS_PAR_NOEUD,
        "REGRESSION: bytes/node {bytes_par_noeud:.2} > threshold {SEUIL_OCTETS_PAR_NOEUD}"
    );
}

/// Threshold gate: allocations per interning, measured with the counting
/// allocator on the same seeded workload. Turns red on any regression of
/// the interning path (e.g. a form clone sneaking back into the unique
/// table).
#[test]
fn test_seuil_allocations_par_internement() {
    let _guard = MESURE_LOCK.lock().unwrap();
    let mut pool = pool_bdd();
    let t0 = pool.terminal(Anchor(0), false);
    let t1 = pool.terminal(Anchor(0), true);
    let mut ids = IdsAleatoires::new(SEED_IDS);
    reset_compteurs();
    for i in 0..NB_INTERNEMENTS {
        pool.intern(
            Anchor(0),
            Form::Node {
                level: Level(i),
                arcs: ids.next_arcs(t0, t1),
            },
        );
    }
    let par_internement = allocations() as f64 / NB_INTERNEMENTS as f64;
    assert!(
        par_internement <= SEUIL_ALLOCATIONS_PAR_INTERNEMENT,
        "REGRESSION: {par_internement:.3} allocations/intern > threshold {SEUIL_ALLOCATIONS_PAR_INTERNEMENT}"
    );
}

/// The randomized IDs are reproducible: same seed, same sequence — the
/// threshold workload is deterministic across runs and machines.
#[test]
fn test_ids_aleatoires_reproductibles() {
    let _guard = MESURE_LOCK.lock().unwrap();
    let mut a = IdsAleatoires::new(SEED_IDS);
    let mut b = IdsAleatoires::new(SEED_IDS);
    let seq_a: Vec<u64> = (0..100).map(|_| a.next_u64()).collect();
    let seq_b: Vec<u64> = (0..100).map(|_| b.next_u64()).collect();
    assert_eq!(seq_a, seq_b, "same seed: same sequence");
    let mut c = IdsAleatoires::new(SEED_IDS.wrapping_add(1));
    let seq_c: Vec<u64> = (0..100).map(|_| c.next_u64()).collect();
    assert_ne!(seq_a, seq_c, "different seed: different sequence");
}
