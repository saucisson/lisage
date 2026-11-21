//! Tests of the reference interpreter (M1-7) on the spec § 1 example:
//! K = {a, b}, V = {0, 1}, S = { {a↦1, b↦0}, {a↦0, b↦0} }.

use std::collections::{BTreeMap, BTreeSet};

use lisage_lang::interpreter::{
    apply, composed, count, difference, exists, filter, intersection, iterate, lookup, map, member,
    saturate, union,
};
use lisage_lang::value::{Atom, Value};

fn atom(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_of(pairs: &[(u64, u64)]) -> Value {
    Value::Map(
        pairs
            .iter()
            .map(|(key, value)| (atom(*key), atom(*value)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// The spec example: S = { {a↦1, b↦0}, {a↦0, b↦0} } with a = 0, b = 1.
fn example_spec() -> BTreeSet<Value> {
    [map_of(&[(0, 1), (1, 0)]), map_of(&[(0, 0), (1, 0)])]
        .into_iter()
        .collect()
}

#[test]
fn test_count_example_spec() {
    assert_eq!(count(&example_spec()), 2);
}

#[test]
fn test_lookup_example_spec() {
    let result = lookup(&example_spec(), &atom(0));
    assert_eq!(result.len(), 2);
    assert_eq!(result[&map_of(&[(0, 1), (1, 0)])], Some(atom(1)));
    assert_eq!(result[&map_of(&[(0, 0), (1, 0)])], Some(atom(0)));
}

#[test]
fn test_lookup_key_absent() {
    let result = lookup(&example_spec(), &atom(9));
    assert!(result.values().all(|value| value.is_none()));
}

#[test]
fn test_filter_example_spec() {
    // p(m) = (m(a) = 1 ∨ a missing), spec § 4.
    let keeps = filter(&example_spec(), |member| match member {
        Value::Map(map) => map.get(&atom(0)) != Some(&atom(0)),
        _ => false,
    });
    assert_eq!(count(&keeps), 1);
    assert!(member(&keeps, &map_of(&[(0, 1), (1, 0)])));
}

#[test]
fn test_exists_and_map() {
    let s = example_spec();
    assert!(exists(&s, |m| m == &map_of(&[(0, 0), (1, 0)])));
    assert!(!exists(&s, |m| m == &map_of(&[(0, 1), (1, 1)])));
    let image = map(&s, |m| m.clone());
    assert_eq!(count(&image), 2);
}

#[test]
fn test_saturate_terminates_and_idempotent() {
    // step: adds the map {a↦1, b↦1} if missing.
    let step_ = |current: &BTreeSet<Value>| {
        let mut next = current.clone();
        next.insert(map_of(&[(0, 1), (1, 1)]));
        next
    };
    let init: BTreeSet<Value> = BTreeSet::new();
    let fixpoint = saturate(step_, init.clone());
    assert_eq!(count(&fixpoint), 1);
    assert_eq!(saturate(step_, fixpoint.clone()), fixpoint);
}

#[test]
fn test_union_intersection_difference() {
    let s = example_spec();
    let complementary: BTreeSet<Value> = [map_of(&[(0, 1), (1, 1)])].into_iter().collect();
    let union_all = union(&s, &complementary);
    assert_eq!(count(&union_all), 3);
    assert!(member(&union_all, &map_of(&[(0, 1), (1, 1)])));
    let intersection_s = intersection(&s, &complementary);
    assert_eq!(count(&intersection_s), 0);
    let difference_s = difference(&s, &complementary);
    assert_eq!(difference_s, s);
    let difference_complementary = difference(&complementary, &s);
    assert_eq!(difference_complementary, complementary);
}

#[test]
fn test_hof_apply_compose() {
    let double = |v: &u64| v * 2;
    let square = |v: &u64| v * v;
    assert_eq!(apply(double, &21), 42);
    let double_square = composed(&double, &square);
    assert_eq!(double_square(&3), 18);
    let square_double = composed(&square, &double);
    assert_eq!(square_double(&3), 36);
}

#[test]
fn test_hof_iterate() {
    let double = |v: &u64| v * 2;
    assert_eq!(iterate(&double, 0, 1), 1);
    assert_eq!(iterate(&double, 1, 1), 2);
    assert_eq!(iterate(&double, 10, 1), 1024);
}

#[test]
fn test_hof_iterate_property_lcg() {
    // Property: iterate(f, n, x) == n manual applications of f,
    // for n drawn from a deterministic LCG (same constants as lisage-plan).
    const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
    const MULTIPLIER_LCG: u64 = 6364136223846793005;
    const INCREMENT_LCG: u64 = 1442695040888963407;
    const NUMBER_CASES: u64 = 50;
    const BOUND_N: u64 = 12;
    const BOUND_X: u64 = 100;
    let mut seed = SEED_LCG;
    let mut next = move || {
        seed = seed
            .wrapping_mul(MULTIPLIER_LCG)
            .wrapping_add(INCREMENT_LCG);
        seed >> 33
    };
    let increment = |v: &u64| (v + 1) % BOUND_X;
    for _case in 0..NUMBER_CASES {
        let n = next() % BOUND_N;
        let x = next() % BOUND_X;
        let mut expected = x;
        for _ in 0..n {
            expected = increment(&expected);
        }
        assert_eq!(iterate(&increment, n, x), expected);
    }
}

#[test]
fn test_hof_composition_associative() {
    // Property: (f ∘ g) ∘ h == f ∘ (g ∘ h).
    let f = |v: &u64| v.wrapping_add(1);
    let g = |v: &u64| v.wrapping_mul(3);
    let h = |v: &u64| v.wrapping_sub(2);
    let f_after_g = composed(&f, &g);
    let g_after_h = composed(&g, &h);
    let left = composed(&f_after_g, &h);
    let right = composed(&f, &g_after_h);
    for x in 0..20 {
        assert_eq!(left(&x), right(&x));
    }
}
