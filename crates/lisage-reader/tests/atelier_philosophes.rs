//! Didactic workshop on the golden thread (backlog § 9, request of November
//! 10, 2026): R_n measured with commutable reductions — shared nodes
//! (R-share) against paths (unfolded DAG). The sharing is visible to the
//! naked eye: R₃ fits in a few shared forms against 20 paths (one per
//! reachable state), and disabling R-collapse/R-default can only increase.

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::ReferenceController;
use lisage_core::workshop::{Reductions, WorkshopController, measure};

use lisage_reader::philosophers::{build_r_n, pool_nine, schema_ring, words_reachable};

const ANCHOR: Anchor = Anchor(0);

fn measure_r_n(n: u64, reductions: Reductions) -> (u64, u64, u64) {
    let schema = schema_ring(n);
    let mut pool: Arena<WorkshopController<ReferenceController>> = Arena::new(
        WorkshopController::new(ReferenceController::new(schema.clone()), reductions),
        Box::new(NoOpObserver),
    );
    let root = build_r_n(&mut pool, ANCHOR, n);
    let report = measure(&pool, root, None).expect("interned node");
    (report.nodes_shared, report.paths, report.size_unfolded)
}

/// Golden thread: R₃ = 20 reachable states = 20 paths (one per state); the
/// shared diagram fits in 33 nodes against an unfolded size of 126
/// (sum of path lengths, shortened by R-collapse) — the sharing
/// is visible to the naked eye.
#[test]
fn test_r3_shared_against_size_unfolded() {
    let (nodes, paths, size_unfolded) = measure_r_n(3, Reductions::ALL);
    assert_eq!(paths, 20, "one path per reachable state of R₃");
    assert_eq!(words_reachable(3).len() as u64, 20);
    assert_eq!(size_unfolded, 126, "sum of the lengths of the 20 paths");
    assert_eq!(nodes, 33, "shared nodes of R₃ in flat encoding");
    assert!(
        nodes < size_unfolded,
        "sharing (R-share) fits in {nodes} nodes against {size_unfolded} unfolded"
    );
}

/// Disabling R-collapse / R-default can only increase (or leave equal) the
/// two metrics on the same construction.
#[test]
fn test_commutation_not_reduced_never() {
    let (nodes_all, paths_all, size_all) = measure_r_n(3, Reductions::ALL);
    let (nodes_no, paths_no, size_no) = measure_r_n(3, Reductions::NO);
    assert!(nodes_no >= nodes_all);
    assert!(paths_no >= paths_all);
    assert!(size_no >= size_all);
    let (nodes_without_collapse, _, _) = measure_r_n(3, Reductions::without_collapse());
    assert!(nodes_without_collapse >= nodes_all);
    let (nodes_without_default, _, _) = measure_r_n(3, Reductions::without_default());
    assert!(nodes_without_default >= nodes_all);
}

/// The reference pool (without the workshop wrapper) gives the same diagram
/// as `Reductions::ALL`: the wrapper changes nothing when everything is active.
#[test]
fn test_wrapper_transparent_when_all_active() {
    let schema = schema_ring(3);
    let mut reference = pool_nine(&schema);
    let root_ref = build_r_n(&mut reference, ANCHOR, 3);
    let report_ref = measure(&reference, root_ref, None).expect("interned node");
    let (nodes, paths, size) = measure_r_n(3, Reductions::ALL);
    assert_eq!(report_ref.nodes_shared, nodes);
    assert_eq!(report_ref.paths, paths);
    assert_eq!(report_ref.size_unfolded, size);
}
