//! Tests for the fold and semirings (M1-6, spec § 5, A4/A5): `Pli<S>`
//! (count over `(ℕ,+,×)`, Bool fold over `(Bool,∨,∧)`), A4 counting
//! expansion on collapsed levels, and `union` (Bool construction merge).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{BoolOu, Pli, compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::Transducteur;
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

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

fn pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a set of maps into a single diagram (union of the members).
fn compiler_ensemble(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    ensemble: &BTreeSet<Value>,
) -> NodeId {
    let keys = SchemaKeys::new(schema.clone());
    let mut resultat = pool.intern(anchor, Form::Terminal(false));
    for membre in ensemble {
        let n = compiler(pool, anchor, schema, membre).unwrap();
        resultat = union(pool, &keys, anchor, resultat, n).unwrap();
    }
    resultat
}

/// Spec § 5 example: `S = {{a↦0}, {a↦1}}` on `Map([a], {0,1})` — the
/// `val(a)` level is collapsed, the explicit path is unique but `|S| = 2`
/// (A4: the level cardinal enters the aggregation).
#[test]
fn test_count_niveau_collapsed_a4_exemple_spec() {
    let schema = Schema::Map(vec![Atom(0)], Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([map_de(&[(0, 0)]), map_de(&[(0, 1)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);
    // The val(a) level was collapsed: a single explicit arc.
    let Form::Node { arcs, .. } = entree.resolve(racine).unwrap() else {
        panic!("presence node expected")
    };
    assert_eq!(arcs.len(), 1, "a single explicit path");
    let total = compter(&entree, &mut sortie, anchor, &schema, racine).unwrap();
    assert_eq!(total, 2, "A4: |Val(val(a))| × 1 = 2");
}

/// Counting on a set of two-key maps (no collapse here).
#[test]
fn test_count_ensemble_maps() {
    let schema = Schema::Map(
        vec![Atom(0), Atom(1)],
        Domaine::nouveau(vec![Atom(0), Atom(1)]),
    );
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([map_de(&[(0, 1), (1, 0)]), map_de(&[(0, 0)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);
    let total = compter(&entree, &mut sortie, anchor, &schema, racine).unwrap();
    assert_eq!(total, 2);
}

/// Empty set: the fold is zero. Full set (all maps): the count is the
/// product of the cardinalities (every level collapsed).
#[test]
fn test_count_ensemble_vide_et_plein() {
    let schema = Schema::Map(vec![Atom(0)], Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);

    let vide = BTreeSet::new();
    let racine_vide = compiler_ensemble(&mut entree, anchor, &schema, &vide);
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, racine_vide).unwrap(),
        0
    );

    // All maps over one key with values {0,1}: {a↦0} and {a↦1}.
    let plein = BTreeSet::from([map_de(&[(0, 0)]), map_de(&[(0, 1)])]);
    let racine_plein = compiler_ensemble(&mut entree, anchor, &schema, &plein);
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, racine_plein).unwrap(),
        2
    );
}

/// Fold over `(Bool, ∨, ∧)`: "the set is non-empty" (∃-summary).
#[test]
fn test_pli_boolou_non_vide() {
    let schema = Schema::Map(vec![Atom(0)], Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);

    let mut runner = Transducteur::new(Pli::<BoolOu>::new(schema.clone()));
    let vide = BTreeSet::new();
    let racine_vide = compiler_ensemble(&mut entree, anchor, &schema, &vide);
    let (_, s_vide) = runner
        .run(&entree, &mut sortie, anchor, racine_vide)
        .unwrap();
    assert_eq!(s_vide, BoolOu(false), "empty set ⇒ false");

    let non_vide = BTreeSet::from([map_de(&[(0, 1)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &non_vide);
    let (_, s) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
    assert_eq!(s, BoolOu(true), "non-empty set ⇒ true");
}

/// Union (Bool construction merge, A5): idempotent, ⊥ neutral; ⊤ is the
/// "all by default" member (the empty map), not an absorbing element.
#[test]
fn test_union_merge_construction_bool() {
    let schema = Schema::Map(vec![Atom(0)], Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let mut entree = pool(&schema);
    let anchor = Anchor(0);
    let keys = SchemaKeys::new(schema.clone());
    let m0 = compiler(&mut entree, anchor, &schema, &map_de(&[(0, 0)])).unwrap();
    let m1 = compiler(&mut entree, anchor, &schema, &map_de(&[(0, 1)])).unwrap();
    let faux = entree.intern(anchor, Form::Terminal(false));
    let vide = compiler(&mut entree, anchor, &schema, &map_de(&[])).unwrap();

    assert_eq!(
        union(&mut entree, &keys, anchor, m0, m0).unwrap(),
        m0,
        "idempotent"
    );
    assert_eq!(
        union(&mut entree, &keys, anchor, m0, faux).unwrap(),
        m0,
        "⊥ neutral"
    );
    let les_deux = union(&mut entree, &keys, anchor, m0, m1).unwrap();
    let les_trois = union(&mut entree, &keys, anchor, les_deux, vide).unwrap();
    let mut sortie = pool(&schema);
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, les_deux).unwrap(),
        2,
        "the union of two members counts 2"
    );
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, les_trois).unwrap(),
        3,
        "⊤ = empty map: a third member, no absorption"
    );
}
