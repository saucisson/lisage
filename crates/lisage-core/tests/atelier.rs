//! Tests for the reduction dashboard (didactic workshop, backlog § 9):
//! commutation of R-collapse and R-default via `AtelierController`, and
//! didactic metrics — shared nodes (R-share) vs paths (size of the unfolded
//! DAG). Pool errors are propagated, never swallowed.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::atelier::{
    AtelierController, Reductions, compter_chemins, compter_noeuds, compter_taille_depliee, mesurer,
};
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd};
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

/// Commuted R-collapse: a node whose arcs all lead to the same target
/// collapses into that target; without R-collapse, the node is kept.
#[test]
fn test_collapse_commute() {
    // 1-variable BDD: {0 → ⊥, 1 → ⊥}.
    let mut avec = pool_bdd(Reductions::TOUTES);
    let faux = avec.intern(ANCRE, Form::Terminal(false));
    let racine_avec = avec.intern(
        ANCRE,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, faux), (1, faux)]),
        },
    );
    assert_eq!(racine_avec, faux, "R-collapse: the node reduces to ⊥");
    let rapport = mesurer(&avec, racine_avec, None).expect("interned node");
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
    assert_ne!(racine_sans, faux, "without R-collapse: the node is kept");
    let rapport = mesurer(&sans, racine_sans, None).expect("interned node");
    assert_eq!(rapport.noeuds_partages, 2, "node + terminal ⊥");
    assert_eq!(rapport.chemins, 2, "two paths to ⊥");
}

/// Commuted R-default: the explicit δ(ℓ) arc toward D_δ is absorbed or not.
#[test]
fn test_defaut_commute() {
    // Region over domain [0, 3): {point 0 → ⊤, VIDE → ⊥}.
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
    let Form::Node { arcs, .. } = avec.resolve(racine_avec).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 1, "R-default: the VIDE arc to ⊥ is absorbed");
    let rapport = mesurer(&avec, racine_avec, None).expect("interned node");
    assert_eq!(rapport.noeuds_partages, 2, "node + ⊤");
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
    let Form::Node { arcs, .. } = sans.resolve(racine_sans).expect("interned node") else {
        panic!("the root is a node");
    };
    assert_eq!(arcs.len(), 2, "without R-default: the VIDE arc is kept");
    let rapport = mesurer(&sans, racine_sans, None).expect("interned node");
    assert_eq!(rapport.noeuds_partages, 3, "node + ⊤ + ⊥");
    assert_eq!(rapport.chemins, 2);
}

/// Metrics on a diamond: 4 shared nodes (root, n1, ⊤, ⊥) and 4 paths — each
/// root × n1 arc combination is a path of the unfolded DAG, even when the
/// sub-paths are shared.
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
    assert_eq!(compter_noeuds(&pool, racine).expect("interned node"), 4);
    assert_eq!(compter_chemins(&pool, racine).expect("interned node"), 4);
    // Unfolded size: root (1) + 2 × n1 (1 + ⊤ + ⊥ = 3) = 7.
    assert_eq!(
        compter_taille_depliee(&pool, racine).expect("interned node"),
        7
    );
    // A non-interned NodeId is a propagated error.
    assert!(compter_noeuds(&pool, NodeId(u32::MAX)).is_err());
    assert!(compter_chemins(&pool, NodeId(u32::MAX)).is_err());
    assert!(compter_taille_depliee(&pool, NodeId(u32::MAX)).is_err());
    assert!(mesurer(&pool, NodeId(u32::MAX), None).is_err());
}

/// Property (LCG): on random BDDs, disabling reductions can only increase
/// (or leave equal) shared nodes and paths — the same form is rebuilt in both
/// pools (same seed).
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
    // Builds a random binary tree of depth ≤ 3 in the pool, with its own LCG
    // re-seeded at `amorce` (same form guaranteed).
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
        let rapport_avec = mesurer(&avec, racine_avec, None).expect("interned node");
        let rapport_sans = mesurer(&sans, racine_sans, None).expect("interned node");
        assert!(rapport_avec.chemins >= 1, "at least one path");
        assert!(
            rapport_sans.noeuds_partages >= rapport_avec.noeuds_partages,
            "without reduction: nodes ≥ (same form)"
        );
        assert!(
            rapport_sans.chemins >= rapport_avec.chemins,
            "without reduction: paths ≥ (same form)"
        );
    }
}

/// Narration (ADR-0100 decision 6): `mesurer` emits an `Attr`/`Synthétise`
/// event carrying the full report — pinned by a collecting observer.
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
    let rapport = mesurer(&pool, racine, Some(&collecteur)).expect("interned node");
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
        "the narration event carries the full report"
    );
}

/// Sliding window, absolute vs relative encoding (controller parameter,
/// backlog § 9): same paths (same logical object), but the absolute encoding
/// creates one node per base while the relative one shares a single form —
/// the window's "4 vs infinity", measured.
#[test]
fn test_fenetre_absolu_croit_relatif_constant() {
    use lisage_core::atelier::comparer_encodages_fenetre;

    const TAILLE: u64 = 4;
    // 10 bases: absolute = root + 10 windows + ⊤ = 12 nodes; relative = 3.
    // (R-collapse commuted: an all-true window would otherwise collapse.)
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport =
        comparer_encodages_fenetre(&mut pool, ANCRE, TAILLE, 10, None).expect("interned nodes");
    assert_eq!(rapport.absolu.noeuds_partages, 12);
    assert_eq!(rapport.relatif.noeuds_partages, 3);
    // Same logical object: same paths (10 bases × 4 keys) and same unfolded
    // size (root + 10 × (window + ⊤)).
    assert_eq!(rapport.absolu.chemins, 40);
    assert_eq!(rapport.relatif.chemins, 40);
    assert_eq!(rapport.absolu.taille_depliee, 51);
    assert_eq!(rapport.relatif.taille_depliee, 51);

    // 100 bases: the relative encoding stays at 3 nodes (constant), the absolute one grows to 102.
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport =
        comparer_encodages_fenetre(&mut pool, ANCRE, TAILLE, 100, None).expect("interned nodes");
    assert_eq!(
        rapport.relatif.noeuds_partages, 3,
        "the relative encoding is O(1)"
    );
    assert_eq!(
        rapport.absolu.noeuds_partages, 102,
        "the absolute encoding grows with the bases"
    );
    assert_eq!(rapport.relatif.chemins, 400);
}

/// Narration (ADR-0100): the encoding comparison emits an `Attr`/`Synthétise`
/// event carrying the two node counts.
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
        .expect("interned nodes");
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
        "the narration event carries the encoding comparison"
    );
}

/// Truth table of f = (x₀ ∧ x₁) ∨ (x₂ ∧ x₃) over 4 variables:
/// `table[a]` = bit₀(a) ∧ bit₁(a) ∨ bit₂(a) ∧ bit₃(a).
fn table_ou_deux_paires() -> Vec<bool> {
    (0..16)
        .map(|a| {
            let bit = |v: u32| (a >> v) & 1 == 1;
            (bit(0) && bit(1)) || (bit(2) && bit(3))
        })
        .collect()
}

/// Order parameter (backlog § 9): the same function compiled under two
/// orders has the same paths (2^n, one per assignment — same logical object)
/// and the same unfolded size (the full tree 2^{n+1}-1), but different
/// shared nodes — the order changes the diagram, not the function.
/// (R-collapse commuted: otherwise the paths would merge.)
/// Natural order [0,1,2,3]: 10 nodes; interleaved [0,2,1,3]: 12 nodes.
#[test]
fn test_ordre_change_noeuds_pas_chemins() {
    use lisage_core::atelier::{Ordre, comparer_ordres};

    let table = table_ou_deux_paires();
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport = comparer_ordres(
        &mut pool,
        ANCRE,
        &table,
        &Ordre::naturel(4),
        &Ordre::personnalise(vec![0, 2, 1, 3]),
        None,
    )
    .expect("interned nodes");
    assert_eq!(rapport.premier.noeuds_partages, 10, "natural order");
    assert_eq!(rapport.second.noeuds_partages, 12, "interleaved order");
    // Same logical object: 2^4 = 16 paths and unfolded tree 2^5-1 = 31.
    assert_eq!(rapport.premier.chemins, 16);
    assert_eq!(rapport.second.chemins, 16);
    assert_eq!(rapport.premier.taille_depliee, 31);
    assert_eq!(rapport.second.taille_depliee, 31);
}

/// The workshop's natural order is exactly the reference `compiler_bdd`
/// (same interned forms, same `NodeId` in the same pool).
#[test]
fn test_ordre_naturel_equivaut_a_compiler_bdd() {
    use lisage_core::atelier::{Ordre, compiler_bdd_ordre};

    let table = table_ou_deux_paires();
    let mut pool = pool_bdd(Reductions::TOUTES);
    let via_atelier = compiler_bdd_ordre(&mut pool, ANCRE, &Ordre::naturel(4), &table);
    let via_bdd = compiler_bdd(&mut pool, ANCRE, &BddKeys::new(4), &table);
    assert_eq!(via_atelier, via_bdd);
}

/// Didactic growth: over 6 variables, f = (x₀∧x₁)∨(x₂∧x₃)∨(x₄∧x₅), the
/// natural order groups the pairs (15 nodes), the interleaved order
/// [0,2,4,1,3,5] separates them (25 nodes) — same 64 paths on both sides.
#[test]
fn test_ordre_intercale_croissance_six_variables() {
    use lisage_core::atelier::{Ordre, comparer_ordres};

    let table: Vec<bool> = (0..64)
        .map(|a| {
            let bit = |v: u32| (a >> v) & 1 == 1;
            (bit(0) && bit(1)) || (bit(2) && bit(3)) || (bit(4) && bit(5))
        })
        .collect();
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport = comparer_ordres(
        &mut pool,
        ANCRE,
        &table,
        &Ordre::naturel(6),
        &Ordre::personnalise(vec![0, 2, 4, 1, 3, 5]),
        None,
    )
    .expect("interned nodes");
    assert_eq!(rapport.premier.noeuds_partages, 15, "natural order");
    assert_eq!(rapport.second.noeuds_partages, 25, "interleaved order");
    assert_eq!(rapport.premier.chemins, 64);
    assert_eq!(rapport.second.chemins, 64);
}

/// Narration (ADR-0100): the order comparison emits an `Attr`/`Synthétise`
/// event carrying the two order labels and the two node counts.
#[test]
fn test_narration_comparaison_ordres() {
    use std::sync::{Arc, Mutex};

    use lisage_core::atelier::{Ordre, comparer_ordres};
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

    let table = table_ou_deux_paires();
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let collecteur = Collecteur {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let rapport = comparer_ordres(
        &mut pool,
        ANCRE,
        &table,
        &Ordre::naturel(4),
        &Ordre::personnalise(vec![0, 2, 1, 3]),
        Some(&collecteur),
    )
    .expect("interned nodes");
    assert_eq!(rapport.premier.noeuds_partages, 10);
    let events = collecteur.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                valeur,
                ..
            } if valeur == "ordre=[0,1,2,3]_vs_[0,2,1,3] noeuds=10 contre 12"
        )),
        "the narration event carries the order comparison"
    );
}

/// Composite anchor parameter (backlog § 9, SWP thread): the slide is the
/// movement of the base within the anchor `{base, horizon}`; the horizon does
/// not move and the absolute reading of the window advances.
#[test]
fn test_ancre_fenetre_glisse() {
    use lisage_core::atelier::AncreFenetre;

    let ancre = AncreFenetre::nouvelle(0, 4);
    assert_eq!(ancre.base(), 0);
    assert_eq!(ancre.horizon(), 4);
    assert_eq!(ancre.fenetre_absolue(), (0, 4));
    assert_eq!(
        ancre.cle_entree(),
        0,
        "the base is the argument of the entry arc"
    );
    assert_eq!(ancre.etiquette(), "{base: 0, horizon: 4}");
    let glissee = ancre.glessee(2);
    assert_eq!(glissee.base(), 2);
    assert_eq!(glissee.horizon(), 4, "the slide only affects the base");
    assert_eq!(glissee.fenetre_absolue(), (2, 6));
    assert_eq!(glissee.cle_entree(), 2);
    assert_eq!(glissee.etiquette(), "{base: 2, horizon: 4}");
    // The original anchor is unchanged (the slide produces a new anchor).
    assert_eq!(ancre.base(), 0);
}

/// The window read under two composite anchors: each anchor has its root
/// (the entry arc carries its base), but both reach the same window form
/// (`meme_forme`) — same nodes, same paths, different absolute readings:
/// "the anchor moves, the pool never does". Over 50 slides, only the root is
/// created at each step, never the form.
#[test]
fn test_glisse_pool_constant_lectures_differentes() {
    use lisage_core::atelier::{AncreFenetre, comparer_ancres_fenetre, construire_fenetre_ancree};

    const TAILLE: u64 = 4;
    // (R-collapse commuted: an all-true window would otherwise collapse.)
    let mut pool = pool_bdd(Reductions::sans_collapse());
    let rapport = comparer_ancres_fenetre(
        &mut pool,
        ANCRE,
        TAILLE,
        &AncreFenetre::nouvelle(0, 4),
        &AncreFenetre::nouvelle(7, 4),
        None,
    )
    .expect("interned nodes");
    assert!(
        rapport.meme_forme,
        "both anchors reach the same window form"
    );
    assert_eq!(rapport.avant, rapport.apres, "same metrics on both sides");
    assert_eq!(rapport.avant.noeuds_partages, 3, "root + window + ⊤");
    assert_eq!(rapport.avant.chemins, 4, "one path per window key");
    assert_eq!(rapport.avant.taille_depliee, 6);
    assert_eq!(
        rapport.noeuds_pool, 4,
        "⊤ + shared window + one root per anchor"
    );
    assert_eq!(rapport.lecture_avant, (0, 4));
    assert_eq!(rapport.lecture_apres, (7, 11));

    // 50 slides (bases 100..150, all new): at each step the base enters via
    // the entry arc, the window form reached is always the same and the pool
    // creates only one root per slide — never a copy of the window.
    let noeuds_avant = pool.node_count();
    let (_, fenetre) =
        construire_fenetre_ancree(&mut pool, ANCRE, TAILLE, &AncreFenetre::nouvelle(0, 4));
    let ancre = AncreFenetre::nouvelle(100, TAILLE as i64);
    for delta in 0..50 {
        let glissee = ancre.glessee(delta);
        assert_eq!(
            glissee.fenetre_absolue(),
            (100 + delta, 100 + delta + TAILLE as i64)
        );
        let (racine, fenetre_glissee) =
            construire_fenetre_ancree(&mut pool, ANCRE, TAILLE, &glissee);
        assert_eq!(fenetre_glissee, fenetre, "window form shared at each slide");
        assert_ne!(racine, fenetre, "each base has its own root");
    }
    assert_eq!(
        pool.node_count(),
        noeuds_avant + 50,
        "a single root per slide: the window form is never duplicated"
    );
}

/// Property (LCG): the slide composes — sliding by `a` then by `b` equals
/// sliding by `a + b` — and the horizon is invariant (30 cases).
#[test]
fn test_propriete_glisse_composition_lcg() {
    use lisage_core::atelier::AncreFenetre;

    const GRAINE_LCG: u64 = 0x5EED_2026_1112_4E11;
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
    for _ in 0..NOMBRE_CAS {
        let a = suivant() as i64 % 1000;
        let b = suivant() as i64 % 1000;
        let horizon = suivant() as i64 % 16 + 1;
        let ancre = AncreFenetre::nouvelle(0, horizon);
        assert_eq!(
            ancre.glessee(a).glessee(b),
            ancre.glessee(a + b),
            "the slide composes: f(a)∘f(b) = f(a+b)"
        );
        assert_eq!(
            ancre.glessee(a).horizon(),
            horizon,
            "the horizon is invariant"
        );
    }
}

/// Narration (ADR-0100): the composite anchor comparison emits an
/// `Attr`/`Synthétise` event carrying the two labels, the two node counts and
/// the two absolute readings.
#[test]
fn test_narration_comparaison_ancres() {
    use std::sync::{Arc, Mutex};

    use lisage_core::atelier::{AncreFenetre, comparer_ancres_fenetre};
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
    let rapport = comparer_ancres_fenetre(
        &mut pool,
        ANCRE,
        4,
        &AncreFenetre::nouvelle(0, 4),
        &AncreFenetre::nouvelle(2, 4),
        Some(&collecteur),
    )
    .expect("interned nodes");
    assert_eq!(rapport.avant.noeuds_partages, 3);
    let events = collecteur.evenements.lock().expect("mutex").clone();
    assert!(
        events.iter().any(|e| matches!(
            e,
            TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                valeur,
                ..
            } if valeur == "ancre={base: 0, horizon: 4}_vs_{base: 2, horizon: 4} noeuds=3 contre 3 lecture=[0..4) contre [2..6) pool=4"
        )),
        "the narration event carries the anchor comparison"
    );
}
