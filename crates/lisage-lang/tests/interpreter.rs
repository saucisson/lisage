//! Tests de l'interprète de référence (M1-7) sur l'exemple de la spec § 1 :
//! K = {a, b}, V = {0, 1}, S = { {a↦1, b↦0}, {a↦0, b↦0} }.

use std::collections::{BTreeMap, BTreeSet};

use lisage_lang::interpreter::{
    appliquer, composer, count, difference, exists, filter, intersection, iterer, lookup, map,
    member, saturate, union,
};
use lisage_lang::value::{Atom, Value};

fn atome(valeur: u64) -> Value {
    Value::Atom(Atom(valeur))
}

fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(cle, valeur)| (atome(*cle), atome(*valeur)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// L'exemple de la spec : S = { {a↦1, b↦0}, {a↦0, b↦0} } avec a = 0, b = 1.
fn exemple_spec() -> BTreeSet<Value> {
    [map_de(&[(0, 1), (1, 0)]), map_de(&[(0, 0), (1, 0)])]
        .into_iter()
        .collect()
}

#[test]
fn test_count_exemple_spec() {
    assert_eq!(count(&exemple_spec()), 2);
}

#[test]
fn test_lookup_exemple_spec() {
    let resultat = lookup(&exemple_spec(), &atome(0));
    assert_eq!(resultat.len(), 2);
    assert_eq!(resultat[&map_de(&[(0, 1), (1, 0)])], Some(atome(1)));
    assert_eq!(resultat[&map_de(&[(0, 0), (1, 0)])], Some(atome(0)));
}

#[test]
fn test_lookup_cle_absente() {
    let resultat = lookup(&exemple_spec(), &atome(9));
    assert!(resultat.values().all(|valeur| valeur.is_none()));
}

#[test]
fn test_filter_exemple_spec() {
    // p(m) = (m(a) = 1 ∨ a absent), spec § 4.
    let garde = filter(&exemple_spec(), |membre| match membre {
        Value::Map(map) => map.get(&atome(0)) != Some(&atome(0)),
        _ => false,
    });
    assert_eq!(count(&garde), 1);
    assert!(member(&garde, &map_de(&[(0, 1), (1, 0)])));
}

#[test]
fn test_exists_et_map() {
    let s = exemple_spec();
    assert!(exists(&s, |m| m == &map_de(&[(0, 0), (1, 0)])));
    assert!(!exists(&s, |m| m == &map_de(&[(0, 1), (1, 1)])));
    let image = map(&s, |m| m.clone());
    assert_eq!(count(&image), 2);
}

#[test]
fn test_saturate_termine_et_idempotent() {
    // pas : ajoute la map {a↦1, b↦1} si absente.
    let pas = |courant: &BTreeSet<Value>| {
        let mut suivant = courant.clone();
        suivant.insert(map_de(&[(0, 1), (1, 1)]));
        suivant
    };
    let init: BTreeSet<Value> = BTreeSet::new();
    let point_fixe = saturate(pas, init.clone());
    assert_eq!(count(&point_fixe), 1);
    assert_eq!(saturate(pas, point_fixe.clone()), point_fixe);
}

#[test]
fn test_union_intersection_difference() {
    let s = exemple_spec();
    let complementaire: BTreeSet<Value> = [map_de(&[(0, 1), (1, 1)])].into_iter().collect();
    let reunion = union(&s, &complementaire);
    assert_eq!(count(&reunion), 3);
    assert!(member(&reunion, &map_de(&[(0, 1), (1, 1)])));
    let intersection_s = intersection(&s, &complementaire);
    assert_eq!(count(&intersection_s), 0);
    let difference_s = difference(&s, &complementaire);
    assert_eq!(difference_s, s);
    let difference_complementaire = difference(&complementaire, &s);
    assert_eq!(difference_complementaire, complementaire);
}

#[test]
fn test_hof_appliquer_composer() {
    let double = |v: &u64| v * 2;
    let carre = |v: &u64| v * v;
    assert_eq!(appliquer(double, &21), 42);
    let double_carre = composer(&double, &carre);
    assert_eq!(double_carre(&3), 18);
    let carre_double = composer(&carre, &double);
    assert_eq!(carre_double(&3), 36);
}

#[test]
fn test_hof_iterer() {
    let double = |v: &u64| v * 2;
    assert_eq!(iterer(&double, 0, 1), 1);
    assert_eq!(iterer(&double, 1, 1), 2);
    assert_eq!(iterer(&double, 10, 1), 1024);
}

#[test]
fn test_hof_iterer_propriete_lcg() {
    // Propriété : iterer(f, n, x) == n applications manuelles de f,
    // pour n tiré d'un LCG déterministe (mêmes constantes que lisage-plan).
    const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NOMBRE_CAS: u64 = 50;
    const BORNE_N: u64 = 12;
    const BORNE_X: u64 = 100;
    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };
    let increment = |v: &u64| (v + 1) % BORNE_X;
    for _cas in 0..NOMBRE_CAS {
        let n = suivant() % BORNE_N;
        let x = suivant() % BORNE_X;
        let mut attendu = x;
        for _ in 0..n {
            attendu = increment(&attendu);
        }
        assert_eq!(iterer(&increment, n, x), attendu);
    }
}

#[test]
fn test_hof_composition_associative() {
    // Propriété : (f ∘ g) ∘ h == f ∘ (g ∘ h).
    let f = |v: &u64| v.wrapping_add(1);
    let g = |v: &u64| v.wrapping_mul(3);
    let h = |v: &u64| v.wrapping_sub(2);
    let f_apres_g = composer(&f, &g);
    let g_apres_h = composer(&g, &h);
    let gauche = composer(&f_apres_g, &h);
    let droite = composer(&f, &g_apres_h);
    for x in 0..20 {
        assert_eq!(gauche(&x), droite(&x));
    }
}
