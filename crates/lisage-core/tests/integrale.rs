//! Tests de l'intégrale au pli (backlog § 9, question ouverte 9 de la
//! spec) : sur les clés-régions 1D, le pli discret (cardinal) devient
//! intégrale de Lebesgue (mesure). Différentiel : mesure continue ==
//! longueur analytique, et cardinal discret == comptage des points.

use std::collections::BTreeSet;

use lisage_algebra::interface::{Anchor, Form, Pool};
use lisage_core::arena::Arena;
use lisage_core::integrale::{
    compiler_union, integrale_constante, intersection, mesure_intervalle,
};
use lisage_core::observer::NoOpObserver;
use lisage_core::region::{Intervalle, RegionController, RegionKeys, VIDE};

type PoolRegion = Arena<RegionController>;

const ANCRE: Anchor = Anchor(0);
const TAILLE_DOMAINE: u64 = 12;

fn pool() -> PoolRegion {
    Arena::new(
        RegionController::nouveau(RegionKeys::nouvelle(vec![TAILLE_DOMAINE])),
        Box::new(NoOpObserver),
    )
}

/// Région pleine de l'univers : [0, TAILLE_DOMAINE).
fn region_pleine() -> Intervalle {
    Intervalle::nouveau(0, TAILLE_DOMAINE - 1)
}

#[test]
fn test_mesure_intervalle_continue() {
    // Longueur continue : [a, b] a pour mesure b − a (le point a mesure 0).
    assert_eq!(mesure_intervalle(&Intervalle::nouveau(2, 5)), 3);
    assert_eq!(mesure_intervalle(&Intervalle::point(4)), 0);
    assert_eq!(mesure_intervalle(&VIDE), 0);
    assert_eq!(mesure_intervalle(&Intervalle::nouveau(0, 11)), 11);
    // Le cardinal discret compte les points : [2, 5] en a 4.
    assert_eq!(Intervalle::nouveau(2, 5).taille(), 4);
}

#[test]
fn test_intersection_intervalles() {
    let a = Intervalle::nouveau(2, 7);
    let b = Intervalle::nouveau(5, 10);
    assert_eq!(intersection(&a, &b), Intervalle::nouveau(5, 7));
    // Disjoints → vide.
    let c = Intervalle::nouveau(8, 10);
    assert_eq!(intersection(&a, &c), VIDE);
    // Vide avec n'importe quoi → vide.
    assert_eq!(intersection(&VIDE, &a), VIDE);
    // Commutativité.
    assert_eq!(intersection(&a, &b), intersection(&b, &a));
}

/// Différentiel fondamental : l'intégrale de densité 1 sur l'union de
/// deux intervalles disjoints est la somme de leurs longueurs, tandis que
/// le pli cardinal (discret) est la somme de leurs tailles. C'est le
/// passage count → mesure.
#[test]
fn test_integrale_union_deux_intervalles() {
    let mut pool = pool();
    let intervalles = vec![Intervalle::nouveau(2, 5), Intervalle::nouveau(7, 9)];
    let racine = compiler_union(&mut pool, ANCRE, &intervalles);
    let mesure = integrale_constante(&pool, racine, &region_pleine()).expect("nœud interné");
    // Mesure continue : (5−2) + (9−7) = 3 + 2 = 5.
    assert_eq!(mesure, 5, "intégrale = somme des longueurs");
    // Cardinal discret : (5−2+1) + (9−7+1) = 4 + 3 = 7.
    let cardinal: u64 = intervalles.iter().map(Intervalle::taille).sum();
    assert_eq!(cardinal, 7, "cardinal = somme des tailles (points)");
    assert_ne!(mesure, cardinal, "count ≠ mesure sur le continu");
}

/// L'intégrale restreinte à une sous-région ne compte que la partie
/// de l'ensemble dénoté qui tombe dans cette sous-région.
#[test]
fn test_integrale_restreinte_a_une_region() {
    let mut pool = pool();
    let intervalles = vec![Intervalle::nouveau(2, 8)];
    let racine = compiler_union(&mut pool, ANCRE, &intervalles);
    // Restriction à [5, 11] : mesure de [2,8] ∩ [5,11] = [5,8], longueur 3.
    let sous_region = Intervalle::nouveau(5, 11);
    let mesure = integrale_constante(&pool, racine, &sous_region).expect("nœud interné");
    assert_eq!(mesure, 3);
    // Restriction à une région disjointe : mesure nulle.
    let disjointe = Intervalle::nouveau(9, 11);
    let mesure_nulle = integrale_constante(&pool, racine, &disjointe).expect("nœud interné");
    assert_eq!(mesure_nulle, 0);
}

/// Le terminal ⊤ seul rend la mesure de toute la région courante ;
/// le terminal ⊥ rend 0.
#[test]
fn test_integrale_terminaux() {
    let mut pool = pool();
    let vrai = pool.intern(ANCRE, Form::Terminal(true));
    let faux = pool.intern(ANCRE, Form::Terminal(false));
    let pleine = region_pleine();
    assert_eq!(
        integrale_constante(&pool, vrai, &pleine).expect("nœud interné"),
        TAILLE_DOMAINE - 1,
        "⊤ rend la mesure de la région pleine"
    );
    assert_eq!(
        integrale_constante(&pool, faux, &pleine).expect("nœud interné"),
        0,
        "⊥ rend 0"
    );
}

/// Propriété (LCG) : pour un sous-ensemble S de [0, T) tiré au sort,
/// l'intégrale du diagramme d'union des intervalles de S égale la somme
/// des longueurs des intervalles, et le cardinal discret égale |S|.
#[test]
fn test_propriete_integrale_contre_cardinal_lcg() {
    const GRAINE_LCG: u64 = 0xC0FF_EE00_1234_5678;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 30;
    let mut graine = GRAINE_LCG;
    for _ in 0..NOMBRE_CAS {
        let mut suivant = || {
            graine = graine
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            graine
        };
        // Sous-ensemble S de [0, TAILLE_DOMAINE) tiré au LCG.
        let ensemble: BTreeSet<u64> = (0..TAILLE_DOMAINE).filter(|_| suivant() % 2 == 1).collect();
        // Compression en intervalles disjoints.
        let mut intervalles = Vec::new();
        let mut point = 0;
        while point < TAILLE_DOMAINE {
            if ensemble.contains(&point) {
                let mut fin = point;
                while fin + 1 < TAILLE_DOMAINE && ensemble.contains(&(fin + 1)) {
                    fin += 1;
                }
                intervalles.push(Intervalle::nouveau(point, fin));
                point = fin + 1;
            } else {
                point += 1;
            }
        }
        let mut pool = pool();
        let racine = compiler_union(&mut pool, ANCRE, &intervalles);
        let mesure = integrale_constante(&pool, racine, &region_pleine()).expect("nœud interné");
        // Mesure analytique : somme des (fin − début) des intervalles.
        let mesure_attendue: u64 = intervalles
            .iter()
            .map(|i| i.borne_sup().saturating_sub(i.borne_inf()))
            .sum();
        assert_eq!(mesure, mesure_attendue, "intégrale == somme des longueurs");
        // Cardinal analytique : |S|.
        let cardinal_attendu = ensemble.len() as u64;
        let cardinal: u64 = intervalles.iter().map(Intervalle::taille).sum();
        assert_eq!(cardinal, cardinal_attendu, "cardinal == |S|");
        // Sur le continu, mesure ≤ cardinal (chaque point a mesure 0).
        assert!(mesure <= cardinal);
    }
}
