//! Interprète de référence (M1-7) : la vérité sémantique, en pur `lang/`
//! sans aucun diagramme de décision. C'est la voie A du triptyque : toute
//! voie DD (B, C) est différentielle contre cette sémantique explicite.

use std::collections::{BTreeMap, BTreeSet};

use crate::value::Value;

/// Compte les membres d'un ensemble — pli sur le semi-anneau (ℕ, +, ×),
/// exact ici car la construction explicite a préservé les multiplicités.
pub fn count(ensemble: &BTreeSet<Value>) -> u64 {
    ensemble.len() as u64
}

/// Teste l'existence d'un membre satisfaisant le prédicat — pli sur
/// (Bool, ∨, ∧), spec § 5.
pub fn exists(ensemble: &BTreeSet<Value>, predicat: impl Fn(&Value) -> bool) -> bool {
    ensemble.iter().any(predicat)
}

/// Garde les membres satisfaisant le prédicat (image réciproque, spec § 4).
pub fn filter(ensemble: &BTreeSet<Value>, predicat: impl Fn(&Value) -> bool) -> BTreeSet<Value> {
    ensemble
        .iter()
        .filter(|valeur| predicat(valeur))
        .cloned()
        .collect()
}

/// Image d'un ensemble par une fonction (transduction à canal hérité seul).
pub fn map(ensemble: &BTreeSet<Value>, fonction: impl Fn(&Value) -> Value) -> BTreeSet<Value> {
    ensemble.iter().map(fonction).collect()
}

/// Adhésion d'une valeur explicite à un ensemble.
pub fn member(ensemble: &BTreeSet<Value>, valeur: &Value) -> bool {
    ensemble.contains(valeur)
}

/// `lookup(k)` (spec § 6) : chaque map membre `m` reçoit en payload `m(k)`.
/// Le résultat est fonctionnel ; `None` quand la clé est absente (l'`Option`
/// est l'algèbre de défauts du résultat, question ouverte 3 de la spec).
pub fn lookup(ensemble: &BTreeSet<Value>, cle: &Value) -> BTreeMap<Value, Option<Value>> {
    ensemble
        .iter()
        .map(|membre| {
            let valeur = match membre {
                Value::Map(map) => map.get(cle).cloned(),
                _ => None,
            };
            (membre.clone(), valeur)
        })
        .collect()
}

/// Point fixe croissant `saturate(pas, init)` (spec § 7) : termine sur schéma
/// borné (T2) car l'ensemble des fonctions Δ → Y y est fini. Le merge de
/// construction est l'union idempotente (choix Bool, A5).
pub fn saturate(
    pas: impl Fn(&BTreeSet<Value>) -> BTreeSet<Value>,
    initial: BTreeSet<Value>,
) -> BTreeSet<Value> {
    let mut courant = initial;
    loop {
        let suivant: BTreeSet<Value> = courant.union(&pas(&courant)).cloned().collect();
        if suivant == courant {
            return courant;
        }
        courant = suivant;
    }
}

/// Union de deux ensembles explicites (algèbre des ensembles, niveau 1).
pub fn union(a: &BTreeSet<Value>, b: &BTreeSet<Value>) -> BTreeSet<Value> {
    a.union(b).cloned().collect()
}

/// Intersection de deux ensembles explicites.
pub fn intersection(a: &BTreeSet<Value>, b: &BTreeSet<Value>) -> BTreeSet<Value> {
    a.intersection(b).cloned().collect()
}

/// Différence ensembliste `a \ b`.
pub fn difference(a: &BTreeSet<Value>, b: &BTreeSet<Value>) -> BTreeSet<Value> {
    a.difference(b).cloned().collect()
}

/// **Composition** de fonctions (HOF, M5-1) : `composer(f, g)(x) = f(g(x))`.
pub fn composer<F, G, V>(f: F, g: G) -> impl Fn(&V) -> V
where
    F: Fn(&V) -> V,
    G: Fn(&V) -> V,
    V: Clone,
{
    move |x| f(&g(x))
}

/// **Application** (HOF) : `appliquer(f, x) = f(x)` — la fonction passée en
/// argument, appliquée à la valeur.
pub fn appliquer<F, V>(f: F, x: &V) -> V
where
    F: Fn(&V) -> V,
    V: Clone,
{
    f(x)
}

/// **Itération** (HOF) : `iterer(f, n, x) = f^n(x)` — n applications de f.
pub fn iterer<F, V>(f: &F, n: u64, x: V) -> V
where
    F: Fn(&V) -> V,
    V: Clone,
{
    let mut courant = x;
    for _ in 0..n {
        courant = f(&courant);
    }
    courant
}
