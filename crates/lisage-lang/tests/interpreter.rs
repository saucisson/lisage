//! Tests de l'interprète de référence (M1-7) sur l'exemple de la spec § 1 :
//! K = {a, b}, V = {0, 1}, S = { {a↦1, b↦0}, {a↦0, b↦0} }.

use std::collections::{BTreeMap, BTreeSet};

use lisage_lang::interpreter::{count, exists, filter, lookup, map, member, saturate};
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
