//! Didactic workshop on the golden thread (backlog § 9, request of November 10,
//! 2026): R_n measured with the commutable reductions — shared nodes
//! (R-share) against paths (unfolded DAG). The sharing is visible to the naked eye:
//! R₃ fits in a few shared forms against 20 paths (one per
//! reachable state), and cutting R-collapse/R-default can only increase it.

use lisage_core::Anchor;
use lisage_core::arena::Arena;
use lisage_core::atelier::{AtelierController, Reductions, mesurer};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::ReferenceController;

use lisage_liseuse::philosophes::{construire_r_n, mots_atteignables, pool_neuf, schema_anneau};

const ANCRE: Anchor = Anchor(0);

fn mesurer_r_n(n: u64, reductions: Reductions) -> (u64, u64, u64) {
    let schema = schema_anneau(n);
    let mut pool: Arena<AtelierController<ReferenceController>> = Arena::new(
        AtelierController::nouveau(ReferenceController::new(schema.clone()), reductions),
        Box::new(NoOpObserver),
    );
    let racine = construire_r_n(&mut pool, ANCRE, n);
    let rapport = mesurer(&pool, racine, None).expect("interned node");
    (
        rapport.noeuds_partages,
        rapport.chemins,
        rapport.taille_depliee,
    )
}

/// Golden thread: R₃ = 20 reachable states = 20 paths (one per state); the
/// shared diagram fits in 33 nodes against an unfolded size of 126
/// (sum of path lengths, shortened by R-collapse) — the sharing
/// is visible to the naked eye.
#[test]
fn test_r3_partage_contre_taille_depliee() {
    let (noeuds, chemins, taille_depliee) = mesurer_r_n(3, Reductions::TOUTES);
    assert_eq!(chemins, 20, "one path per reachable state of R₃");
    assert_eq!(mots_atteignables(3).len() as u64, 20);
    assert_eq!(taille_depliee, 126, "sum of the lengths of the 20 paths");
    assert_eq!(noeuds, 33, "shared nodes of R₃ in flat encoding");
    assert!(
        noeuds < taille_depliee,
        "sharing (R-share) fits in {noeuds} nodes against {taille_depliee} unfolded"
    );
}

/// Cutting R-collapse / R-default can only increase (or leave equal) the
/// two metrics on the same construction.
#[test]
fn test_commutation_ne_reduit_jamais() {
    let (noeuds_toutes, chemins_toutes, taille_toutes) = mesurer_r_n(3, Reductions::TOUTES);
    let (noeuds_aucune, chemins_aucune, taille_aucune) = mesurer_r_n(3, Reductions::AUCUNE);
    assert!(noeuds_aucune >= noeuds_toutes);
    assert!(chemins_aucune >= chemins_toutes);
    assert!(taille_aucune >= taille_toutes);
    let (noeuds_sans_collapse, _, _) = mesurer_r_n(3, Reductions::sans_collapse());
    assert!(noeuds_sans_collapse >= noeuds_toutes);
    let (noeuds_sans_defaut, _, _) = mesurer_r_n(3, Reductions::sans_defaut());
    assert!(noeuds_sans_defaut >= noeuds_toutes);
}

/// The reference pool (without the workshop wrapper) gives the same diagram as
/// `Reductions::TOUTES`: the wrapper changes nothing when everything is active.
#[test]
fn test_enveloppe_transparente_quand_tout_actif() {
    let schema = schema_anneau(3);
    let mut reference = pool_neuf(&schema);
    let racine_ref = construire_r_n(&mut reference, ANCRE, 3);
    let rapport_ref = mesurer(&reference, racine_ref, None).expect("interned node");
    let (noeuds, chemins, taille) = mesurer_r_n(3, Reductions::TOUTES);
    assert_eq!(rapport_ref.noeuds_partages, noeuds);
    assert_eq!(rapport_ref.chemins, chemins);
    assert_eq!(rapport_ref.taille_depliee, taille);
}
