//! Reference interpreter (M1-7): the semantic truth, in pure `lang/`
//! without any decision diagram. It is path A of the triptych: every
//! DD path (B, C) is differential against this explicit semantics.

use std::collections::{BTreeMap, BTreeSet};

use crate::value::Value;

/// Counts the members of a set — fold over the semiring (ℕ, +, ×),
/// exact here because the explicit construction preserved multiplicities.
pub fn count(set: &BTreeSet<Value>) -> u64 {
    set.len() as u64
}

/// Tests the existence of a member satisfying the predicate — fold over
/// (Bool, ∨, ∧), spec § 5.
pub fn exists(set: &BTreeSet<Value>, predicate: impl Fn(&Value) -> bool) -> bool {
    set.iter().any(predicate)
}

/// Keeps the members satisfying the predicate (preimage, spec § 4).
pub fn filter(set: &BTreeSet<Value>, predicate: impl Fn(&Value) -> bool) -> BTreeSet<Value> {
    set.iter()
        .filter(|value| predicate(value))
        .cloned()
        .collect()
}

/// Image of a set under a function (transduction with inherited channel only).
pub fn map(set: &BTreeSet<Value>, function: impl Fn(&Value) -> Value) -> BTreeSet<Value> {
    set.iter().map(function).collect()
}

/// Membership of an explicit value in a set.
pub fn member(set: &BTreeSet<Value>, value: &Value) -> bool {
    set.contains(value)
}

/// `lookup(k)` (spec § 6): each member map `m` receives payload `m(k)`.
/// The result is functional; `None` when the key is absent (the `Option`
/// is the default algebra of the result, open question 3 of the spec).
pub fn lookup(set: &BTreeSet<Value>, key: &Value) -> BTreeMap<Value, Option<Value>> {
    set.iter()
        .map(|membre| {
            let value = match membre {
                Value::Map(map) => map.get(key).cloned(),
                _ => None,
            };
            (membre.clone(), value)
        })
        .collect()
}

/// Increasing fixed point `saturate(step, init)` (spec § 7): terminates on a
/// bounded schema (T2) because the set of functions Δ → Y is finite there.
/// The construction merge is the idempotent union (Bool choice, A5).
pub fn saturate(
    not: impl Fn(&BTreeSet<Value>) -> BTreeSet<Value>,
    initial: BTreeSet<Value>,
) -> BTreeSet<Value> {
    let mut current = initial;
    loop {
        let next: BTreeSet<Value> = current.union(&not(&current)).cloned().collect();
        if next == current {
            return current;
        }
        current = next;
    }
}

/// Union of two explicit sets (set algebra, level 1).
pub fn union(a: &BTreeSet<Value>, b: &BTreeSet<Value>) -> BTreeSet<Value> {
    a.union(b).cloned().collect()
}

/// Intersection of two explicit sets.
pub fn intersection(a: &BTreeSet<Value>, b: &BTreeSet<Value>) -> BTreeSet<Value> {
    a.intersection(b).cloned().collect()
}

/// Set difference `a \ b`.
pub fn difference(a: &BTreeSet<Value>, b: &BTreeSet<Value>) -> BTreeSet<Value> {
    a.difference(b).cloned().collect()
}

/// **Composition** of functions (HOF, M5-1): `composed(f, g)(x) = f(g(x))`.
pub fn composed<F, G, V>(f: F, g: G) -> impl Fn(&V) -> V
where
    F: Fn(&V) -> V,
    G: Fn(&V) -> V,
    V: Clone,
{
    move |x| f(&g(x))
}

/// **Application** (HOF): `apply(f, x) = f(x)` — the function passed as
/// argument, applied to the value.
pub fn apply<F, V>(f: F, x: &V) -> V
where
    F: Fn(&V) -> V,
    V: Clone,
{
    f(x)
}

/// **Iteration** (HOF): `iterate(f, n, x) = f^n(x)` — n applications of f.
pub fn iterate<F, V>(f: &F, n: u64, x: V) -> V
where
    F: Fn(&V) -> V,
    V: Clone,
{
    let mut current = x;
    for _ in 0..n {
        current = f(&current);
    }
    current
}
