//! Tests du tableau de bord des réductions (atelier didactique, backlog § 9)
//! : commutation de R-collapse et R-default via `AtelierController`, et
//! métriques didactiques — nœuds partagés (R-share) contre chemins (taille
//! du DAG déplié). Les erreurs du pool sont propagées, jamais avalées.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::atelier::{
    AtelierController, Reductions, compter_chemins, compter_noeuds, compter_taille_depliee, mesurer,
};
use lisage_core::bdd::BddController;
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{Intervalle, RegionController, RegionKeys, VIDE};

type PoolBdd = Arena<AtelierController<BddController>>;
type PoolRegion = Arena<AtelierController<RegionController>>;

const ANCRE: Anchor = Anchor(0);

fn pool_bdd(reductions: Reductions) -> PoolBdd {
    Arena::new(
        AtelierController::nouveau(BddController::new(2), reductions),
        Box::new(NoOpObserver),
    )
}

fn pool_region(reductions: Reductions) -> PoolRegion {
    Arena::new(
        AtelierController::nouveau(
            RegionController::nouveau(RegionKeys::nouvelle(vec![3])),
            reductions,
        ),
        Box::new(NoOpObserver),
    )
}

/// R-collapse commuté : un nœud dont tous les arcs mènent à la même cible
/// collapse en cette cible ; sans R-collapse, le nœud est conservé.
#[test]
fn test_collapse_commute() {
    // BDD 1 variable : {0 → ⊥, 1 → ⊥}.
    let mut avec = pool_bdd(Reductions::TOUTES);
    let faux = avec.intern(ANCRE, Form::Terminal(false));
    let racine_avec = avec.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, faux), (1, faux)]),
        },
    );
    assert_eq!(racine_avec, faux, "R-collapse : le nœud se réduit à ⊥");
    let rapport = mesurer(&avec, racine_avec, None).expect("nœud interné");
    assert_eq!(rapport.noeuds_partages, 1);
    assert_eq!(rapport.chemins, 1);

    let mut sans = pool_bdd(Reductions::sans_collapse());
    let faux = sans.intern(ANCRE, Form::Terminal(false));
    let racine_sans = sans.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, faux), (1, faux)]),
        },
    );
    assert_ne!(racine_sans, faux, "sans R-collapse : le nœud est conservé");
    let rapport = mesurer(&sans, racine_sans, None).expect("nœud interné");
    assert_eq!(rapport.noeuds_partages, 2, "nœud + terminal ⊥");
    assert_eq!(rapport.chemins, 2, "deux chemins vers ⊥");
}

/// R-default commuté : l'arc δ(ℓ) explicite vers D_δ est absorbé ou non.
#[test]
fn test_defaut_commute() {
    // Région sur domaine [0, 3) : {point 0 → ⊤, VIDE → ⊥}.
    let mut avec = pool_region(Reductions::TOUTES);
    let vrai = avec.intern(ANCRE, Form::Terminal(true));
    let faux = avec.intern(ANCRE, Form::Terminal(false));
    let racine_avec = avec.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(Intervalle::point(0), vrai), (VIDE, faux)]),
        },
    );
    let Form::Node { arcs, .. } = avec.resolve(racine_avec).expect("nœud interné") else {
        panic!("la racine est un nœud");
    };
    assert_eq!(arcs.len(), 1, "R-default : l'arc VIDE vers ⊥ est absorbé");
    let rapport = mesurer(&avec, racine_avec, None).expect("nœud interné");
    assert_eq!(rapport.noeuds_partages, 2, "nœud + ⊤");
    assert_eq!(rapport.chemins, 1);

    let mut sans = pool_region(Reductions::sans_defaut());
    let vrai = sans.intern(ANCRE, Form::Terminal(true));
    let faux = sans.intern(ANCRE, Form::Terminal(false));
    let racine_sans = sans.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(Intervalle::point(0), vrai), (VIDE, faux)]),
        },
    );
    let Form::Node { arcs, .. } = sans.resolve(racine_sans).expect("nœud interné") else {
        panic!("la racine est un nœud");
    };
    assert_eq!(arcs.len(), 2, "sans R-default : l'arc VIDE est conservé");
    let rapport = mesurer(&sans, racine_sans, None).expect("nœud interné");
    assert_eq!(rapport.noeuds_partages, 3, "nœud + ⊤ + ⊥");
    assert_eq!(rapport.chemins, 2);
}

/// Métriques sur un losange : 4 nœuds partagés (racine, n1, ⊤, ⊥) et 4
/// chemins — chaque combinaison d'arcs racine × n1 est un chemin du DAG
/// déplié, même quand les sous-chemins sont partagés.
#[test]
fn test_chemins_contre_noeuds() {
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let vrai = pool.intern(ANCRE, Form::Terminal(true));
    let faux = pool.intern(ANCRE, Form::Terminal(false));
    let n1 = pool.intern(
        ANCRE,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let racine = pool.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, n1), (1, n1)]),
        },
    );
    assert_eq!(compter_noeuds(&pool, racine).expect("nœud interné"), 4);
    assert_eq!(compter_chemins(&pool, racine).expect("nœud interné"), 4);
    // Taille dépliée : racine (1) + 2 × n1 (1 + ⊤ + ⊥ = 3) = 7.
    assert_eq!(
        compter_taille_depliee(&pool, racine).expect("nœud interné"),
        7
    );
    // Un NodeId non interné est une erreur propagée.
    assert!(compter_noeuds(&pool, NodeId(u32::MAX)).is_err());
    assert!(compter_chemins(&pool, NodeId(u32::MAX)).is_err());
    assert!(compter_taille_depliee(&pool, NodeId(u32::MAX)).is_err());
    assert!(mesurer(&pool, NodeId(u32::MAX), None).is_err());
}

/// Propriété (LCG) : sur des BDD aléatoires, désactiver les réductions ne
/// peut qu'augmenter (ou laisser égal) nœuds partagés et chemins — la même
/// forme est reconstruite dans les deux pools (même graine).
#[test]
fn test_propriete_commutation_lcg() {
    const GRAINE_LCG: u64 = 0x0A7E_11E5_2026_1110;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    let mut suivant = || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine
    };
    // Construit un arbre binaire aléatoire de profondeur ≤ 3 dans le pool,
    // avec son propre LCG ré-amorcé à `amorce` (même forme garantie).
    fn arbre(pool: &mut PoolBdd, profondeur: u32, amorce: u64) -> NodeId {
        let mut graine = amorce;
        fn suivant(graine: &mut u64) -> u64 {
            *graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            *graine
        }
        fn construit(pool: &mut PoolBdd, profondeur: u32, graine: &mut u64) -> NodeId {
            if profondeur == 0 || !suivant(graine).is_multiple_of(3) {
                return pool.intern(ANCRE, Form::Terminal(suivant(graine) % 2 == 1));
            }
            let gauche = construit(pool, profondeur - 1, graine);
            let droite = construit(pool, profondeur - 1, graine);
            pool.intern(
                ANCRE,
                Form::Node {
                    level: Level(profondeur),
                    arcs: BTreeMap::from([(0, gauche), (1, droite)]),
                },
            )
        }
        construit(pool, profondeur, &mut graine)
    }
    for _ in 0..NOMBRE_CAS {
        let amorce = suivant();
        let mut avec = pool_bdd(Reductions::TOUTES);
        let racine_avec = arbre(&mut avec, 3, amorce);
        let mut sans = pool_bdd(Reductions::AUCUNE);
        let racine_sans = arbre(&mut sans, 3, amorce);
        let rapport_avec = mesurer(&avec, racine_avec, None).expect("nœud interné");
        let rapport_sans = mesurer(&sans, racine_sans, None).expect("nœud interné");
        assert!(rapport_avec.chemins >= 1, "au moins un chemin");
        assert!(
            rapport_sans.noeuds_partages >= rapport_avec.noeuds_partages,
            "sans réduction : nœuds ≥ (même forme)"
        );
        assert!(
            rapport_sans.chemins >= rapport_avec.chemins,
            "sans réduction : chemins ≥ (même forme)"
        );
    }
}

/// Narration (ADR-0100 décision 6) : `mesurer` émet un événement
/// `Attr`/`Synthétise` portant le rapport complet — épinglé par un
/// observateur collecteur.
#[test]
fn test_narration_mesurer() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{CanalAttr, TraceEvent};

    #[derive(Clone)]
    struct Collecteur {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::sans_collapse());
    let vrai = pool.intern(ANCRE, Form::Terminal(true));
    let faux = pool.intern(ANCRE, Form::Terminal(false));
    let n1 = pool.intern(
        ANCRE,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let racine = pool.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, n1), (1, n1)]),
        },
    );
    let collecteur = Collecteur {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let rapport = mesurer(&pool, racine, Some(&collecteur)).expect("nœud interné");
    assert_eq!(rapport.noeuds_partages, 4);
    let events = collecteur.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                valeur,
                ..
            } if valeur == "noeuds_partages=4 chemins=4 taille_depliee=7"
        )),
        "l'événement de narration porte le rapport complet"
    );
}

/// Fenêtre glissante, encodage absolu contre relatif (paramètre du
/// contrôleur, backlog § 9) : mêmes chemins (même objet logique), mais
/// l'absolu crée un nœud par base tandis que le relatif partage une seule
/// forme — le « 4 contre l'infini » de la fenêtre, mesuré.
#[test]
fn test_fenetre_absolu_croit_relatif_constant() {
    use lisage_core::atelier::comparer_encodages_fenetre;

    const TAILLE: u64 = 4;
    // 10 bases : absolu = racine + 10 fenêtres + ⊤ = 12 nœuds ; relatif = 3.
    // (R-collapse commuté : une fenêtre toute-vraie collapserait sinon.)
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport =
        comparer_encodages_fenetre(&mut pool, ANCRE, TAILLE, 10, None).expect("nœuds internés");
    assert_eq!(rapport.absolu.noeuds_partages, 12);
    assert_eq!(rapport.relatif.noeuds_partages, 3);
    // Même objet logique : mêmes chemins (10 bases × 4 clés) et même
    // taille dépliée (racine + 10 × (fenêtre + ⊤)).
    assert_eq!(rapport.absolu.chemins, 40);
    assert_eq!(rapport.relatif.chemins, 40);
    assert_eq!(rapport.absolu.taille_depliee, 51);
    assert_eq!(rapport.relatif.taille_depliee, 51);

    // 100 bases : le relatif reste à 3 nœuds (constant), l'absolu passe à 102.
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport =
        comparer_encodages_fenetre(&mut pool, ANCRE, TAILLE, 100, None).expect("nœuds internés");
    assert_eq!(rapport.relatif.noeuds_partages, 3, "le relatif est O(1)");
    assert_eq!(
        rapport.absolu.noeuds_partages, 102,
        "l'absolu croît avec les bases"
    );
    assert_eq!(rapport.relatif.chemins, 400);
}

/// Narration (ADR-0100) : la comparaison d'encodages émet un événement
/// `Attr`/`Synthétise` portant les deux comptes de nœuds.
#[test]
fn test_narration_comparaison_encodages() {
    use std::sync::{Arc, Mutex};

    use lisage_core::atelier::comparer_encodages_fenetre;
    use lisage_core::observer::{CanalAttr, TraceEvent};

    #[derive(Clone)]
    struct Collecteur {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl lisage_core::observer::Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let mut pool = pool_bdd(Reductions::sans_collapse());
    let collecteur = Collecteur {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let rapport = comparer_encodages_fenetre(&mut pool, ANCRE, 4, 10, Some(&collecteur))
        .expect("nœuds internés");
    assert_eq!(rapport.relatif.noeuds_partages, 3);
    let events = collecteur.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                valeur,
                ..
            } if valeur == "encodage=relatif_vs_absolu noeuds=3 contre 12"
        )),
        "l'événement de narration porte la comparaison d'encodages"
    );
}
