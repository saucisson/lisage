//! Region key algebra tests, intervals fragment (backlog § 9, open question
//! 9 of the spec): canonical normal form (C1 relative to the region
//! algebra), cardinality, default, pool sharing, and differential against
//! naive enumeration on one level.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{Intervalle, RegionController, RegionKeys, VIDE};

type PoolRegion = Arena<RegionController>;

const ANCRE: Anchor = Anchor(0);

fn pool(domaines: Vec<u64>) -> PoolRegion {
    Arena::new(
        RegionController::nouveau(RegionKeys::nouvelle(domaines)),
        Box::new(NoOpObserver),
    )
}

#[test]
fn test_intervalle_vide_normalise() {
    let vide = Intervalle::nouveau(5, 3);
    assert_eq!(vide, VIDE);
    assert!(vide.est_vide());
    assert_eq!(vide.taille(), 0);
    assert!(!vide.contient(3));
    let point = Intervalle::point(4);
    assert!(!point.est_vide());
    assert_eq!(point.taille(), 1);
    assert!(point.contient(4));
    assert!(!point.contient(5));
    assert_eq!(Intervalle::nouveau(2, 5).taille(), 4);
}

#[test]
fn test_normal_form_canonique_et_idempotente() {
    let keys = RegionKeys::nouvelle(vec![8]);
    // Any writing of an empty interval has the same normal form.
    assert_eq!(keys.normal_form(&Intervalle::nouveau(7, 2)), VIDE);
    assert_eq!(keys.normal_form(&VIDE), VIDE);
    // Forme normale idempotente, et stable sur les non-vides.
    let intervalle = Intervalle::nouveau(2, 5);
    assert_eq!(keys.normal_form(&keys.normal_form(&intervalle)), intervalle);
}

#[test]
fn test_cardinalite_et_defaut_par_niveau() {
    let keys = RegionKeys::nouvelle(vec![8, 3]);
    assert_eq!(keys.cardinality(Level(0)), 8);
    assert_eq!(keys.cardinality(Level(1)), 3);
    // δ(ℓ) = the empty interval, the continuous "absent".
    assert_eq!(keys.default(Level(0)), Some(VIDE));
    assert_eq!(keys.default(Level(1)), Some(VIDE));
}

/// Relative C1: two writings of the same region share the same node in the
/// pool (R-share after normal form), two distinct regions do not.
#[test]
fn test_partage_pool_c1_relatif() {
    let mut pool = pool(vec![8]);
    let terminal = pool.intern(ANCRE, Form::Terminal(true));
    let niveau = Level(0);
    let forme_vide_a = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(VIDE, terminal)]),
    };
    let forme_vide_b = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(Intervalle::nouveau(9, 2), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCRE, forme_vide_a.clone()),
        pool.intern(ANCRE, forme_vide_b),
        "empty interval under two writings = same node"
    );
    let forme_point_a = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(Intervalle::point(3), terminal)]),
    };
    let forme_point_b = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(Intervalle::nouveau(3, 3), terminal)]),
    };
    assert_eq!(
        pool.intern(ANCRE, forme_point_a.clone()),
        pool.intern(ANCRE, forme_point_b),
        "point written two ways = same node"
    );
    let forme_autre = Form::Node {
        level: niveau,
        arcs: BTreeMap::from([(Intervalle::point(4), terminal)]),
    };
    assert_ne!(
        pool.intern(ANCRE, forme_point_a),
        pool.intern(ANCRE, forme_autre),
        "distinct regions = distinct nodes"
    );
}

/// Property (LCG): the normal form is idempotent, and taille/est_vide/
/// contient are consistent with the normalized representation.
#[test]
fn test_propriete_normalisation_lcg() {
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let keys = RegionKeys::nouvelle(vec![16]);
    let mut graine = GRAINE_LCG;
    for _ in 0..NOMBRE_CAS {
        let mut suivant = || {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            graine
        };
        let inf = suivant() % 16;
        let sup = suivant() % 16;
        let intervalle = Intervalle::nouveau(inf, sup);
        let normalisee = keys.normal_form(&intervalle);
        assert_eq!(keys.normal_form(&normalisee), normalisee, "idempotence");
        assert_eq!(normalisee.est_vide(), inf > sup);
        assert_eq!(
            normalisee.taille(),
            if inf > sup { 0 } else { sup - inf + 1 }
        );
        let point = suivant() % 16;
        assert_eq!(
            normalisee.contient(point),
            !normalisee.est_vide() && inf <= point && point <= sup
        );
    }
}

/// Differential against naive enumeration: a level-0 node whose arcs are
/// disjoint intervals to `terminal(true)` (the set S) or `terminal(false)`
/// (its complement) represents S; the point count of the DD's true arcs
/// equals |S|. The `VIDE` arc to the default is removed by R-default.
#[test]
fn test_differentiel_union_intervalles_contre_enumeration() {
    const GRAINE_LCG: u64 = 0x2545_F491_4F6C_DD1D;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    const TAILLE_DOMAINE: u64 = 12;
    let mut pool = pool(vec![TAILLE_DOMAINE]);
    let vrai_terminal = pool.intern(ANCRE, Form::Terminal(true));
    let faux_terminal = pool.intern(ANCRE, Form::Terminal(false));
    let mut graine = GRAINE_LCG;
    for _ in 0..NOMBRE_CAS {
        let mut suivant = || {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            graine
        };
        // Subset S of [0, TAILLE_DOMAINE) drawn with the LCG.
        let ensemble: BTreeSet<u64> = (0..TAILLE_DOMAINE).filter(|_| suivant() % 2 == 1).collect();
        // Compression of S and its complement into disjoint intervals.
        let mut arcs = BTreeMap::new();
        let mut point = 0;
        while point < TAILLE_DOMAINE {
            let dans_s = ensemble.contains(&point);
            let mut fin = point;
            while fin + 1 < TAILLE_DOMAINE && ensemble.contains(&(fin + 1)) == dans_s {
                fin += 1;
            }
            arcs.insert(
                Intervalle::nouveau(point, fin),
                if dans_s { vrai_terminal } else { faux_terminal },
            );
            point = fin + 1;
        }
        // The absent arc leads to the default D_δ = terminal(false): R-default.
        arcs.insert(VIDE, faux_terminal);
        let racine = pool.intern(
            ANCRE,
            Form::Node {
                level: Level(0),
                arcs,
            },
        );
        let compte_dd: u64 = match pool.resolve(racine).expect("interned node") {
            Form::Terminal(valeur) => {
                // R-collapse: S empty or full, the node reduces to the terminal.
                if *valeur { TAILLE_DOMAINE } else { 0 }
            }
            Form::Node { arcs, .. } => {
                assert!(
                    !arcs.contains_key(&VIDE),
                    "R-default removes the arc to the default"
                );
                arcs.iter()
                    .filter(|(_, cible)| **cible == vrai_terminal)
                    .map(|(intervalle, _)| intervalle.taille())
                    .sum()
            }
        };
        assert_eq!(
            compte_dd,
            ensemble.len() as u64,
            "DD count == naive cardinal of S"
        );
    }
}
