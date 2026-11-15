//! Didactic workshop on the golden thread (backlog § 9, request of November 10,
//! 2026): R_n measured with the commutable reductions — shared nodes
//! (R-share) against paths (unfolded DAG). The sharing is visible to the naked eye:
//! R₃ fits in a few shared forms against 20 paths (one per
//! reachable state), and cutting R-collapse/R-default can only increase it.

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::atelier::{AtelierController, Reductions, measure};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::ReferenceController;

use lisage_liseuse::philosophes::{build_r_n, mots_atteignables, pool_neuf, schema_anneau};

const ANCHOR: Anchor = Anchor(0);

fn measure_r_n(n: u64, reductions: Reductions) -> (u64, u64, u64) {
    let schema = schema_anneau(n);
    let mut pool: Arena<AtelierController<ReferenceController>> = Arena::new(
        AtelierController::new(ReferenceController::new(schema.clone()), reductions),
        Box::new(NoOpObserver),
    );
    let root = build_r_n(&mut pool, ANCHOR, n);
    let rapport = measure(&pool, root, None).expect("interned node");
    (
        rapport.nodes_partages,
        rapport.chemins,
        rapport.size_depliee,
    )
}

/// Golden thread: R₃ = 20 reachable states = 20 paths (one per state); the
/// shared diagram fits in 33 nodes against an unfolded size of 126
/// (sum of path lengths, shortened by R-collapse) — the sharing
/// is visible to the naked eye.
#[test]
fn test_r3_partage_contre_size_depliee() {
    let (nodes, chemins, size_depliee) = measure_r_n(3, Reductions::TOUTES);
    assert_eq!(chemins, 20, "one path per reachable state of R₃");
    assert_eq!(mots_atteignables(3).len() as u64, 20);
    assert_eq!(size_depliee, 126, "sum of the lengths of the 20 paths");
    assert_eq!(nodes, 33, "shared nodes of R₃ in flat encoding");
    assert!(
        nodes < size_depliee,
        "sharing (R-share) fits in {nodes} nodes against {size_depliee} unfolded"
    );
}

/// Cutting R-collapse / R-default can only increase (or leave equal) the
/// two metrics on the same construction.
#[test]
fn test_commutation_ne_reduit_jamais() {
    let (nodes_toutes, chemins_toutes, size_toutes) = measure_r_n(3, Reductions::TOUTES);
    let (nodes_aucune, chemins_aucune, size_aucune) = measure_r_n(3, Reductions::AUCUNE);
    assert!(nodes_aucune >= nodes_toutes);
    assert!(chemins_aucune >= chemins_toutes);
    assert!(size_aucune >= size_toutes);
    let (nodes_sans_collapse, _, _) = measure_r_n(3, Reductions::sans_collapse());
    assert!(nodes_sans_collapse >= nodes_toutes);
    let (nodes_sans_defaut, _, _) = measure_r_n(3, Reductions::sans_defaut());
    assert!(nodes_sans_defaut >= nodes_toutes);
}

/// The reference pool (without the workshop wrapper) gives the same diagram as
/// `Reductions::TOUTES`: the wrapper changes nothing when everything is active.
#[test]
fn test_enveloppe_transparente_quand_tout_actif() {
    let schema = schema_anneau(3);
    let mut reference = pool_neuf(&schema);
    let root_ref = build_r_n(&mut reference, ANCHOR, 3);
    let rapport_ref = measure(&reference, root_ref, None).expect("interned node");
    let (nodes, chemins, size) = measure_r_n(3, Reductions::TOUTES);
    assert_eq!(rapport_ref.nodes_partages, nodes);
    assert_eq!(rapport_ref.chemins, chemins);
    assert_eq!(rapport_ref.size_depliee, size);
}
