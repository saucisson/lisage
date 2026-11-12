//! Atelier didactique sur le fil rouge (backlog § 9, demande du 10 novembre
//! 2026) : R_n mesuré avec les réductions commutables — nœuds partagés
//! (R-share) contre chemins (DAG déplié). Le partage se voit à l'œil nu :
//! R₃ tient en quelques formes partagées contre 20 chemins (un par état
//! atteignable), et couper R-collapse/R-default ne peut qu'augmenter.

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
    let rapport = mesurer(&pool, racine, None).expect("nœud interné");
    (
        rapport.noeuds_partages,
        rapport.chemins,
        rapport.taille_depliee,
    )
}

/// Fil rouge : R₃ = 20 états atteignables = 20 chemins (un par état) ; le
/// diagramme partagé tient en 33 nœuds contre une taille dépliée de 126
/// (somme des longueurs de chemins, raccourcies par R-collapse) — le partage
/// se voit à l'œil nu.
#[test]
fn test_r3_partage_contre_taille_depliee() {
    let (noeuds, chemins, taille_depliee) = mesurer_r_n(3, Reductions::TOUTES);
    assert_eq!(chemins, 20, "un chemin par état atteignable de R₃");
    assert_eq!(mots_atteignables(3).len() as u64, 20);
    assert_eq!(taille_depliee, 126, "somme des longueurs des 20 chemins");
    assert_eq!(noeuds, 33, "nœuds partagés de R₃ en encodage plat");
    assert!(
        noeuds < taille_depliee,
        "le partage (R-share) tient en {noeuds} nœuds contre {taille_depliee} dépliés"
    );
}

/// Couper R-collapse / R-default ne peut qu'augmenter (ou laisser égal) les
/// deux métriques sur la même construction.
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

/// Le pool de référence (sans enveloppe atelier) donne le même diagramme que
/// `Reductions::TOUTES` : l'enveloppe ne change rien quand tout est actif.
#[test]
fn test_enveloppe_transparente_quand_tout_actif() {
    let schema = schema_anneau(3);
    let mut reference = pool_neuf(&schema);
    let racine_ref = construire_r_n(&mut reference, ANCRE, 3);
    let rapport_ref = mesurer(&reference, racine_ref, None).expect("nœud interné");
    let (noeuds, chemins, taille) = mesurer_r_n(3, Reductions::TOUTES);
    assert_eq!(rapport_ref.noeuds_partages, noeuds);
    assert_eq!(rapport_ref.chemins, chemins);
    assert_eq!(rapport_ref.taille_depliee, taille);
}
