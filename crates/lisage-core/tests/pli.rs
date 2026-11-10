//! Tests du pli et des semi-anneaux (M1-6, spec § 5, A4/A5) : `Pli<S>`
//! (count sur `(ℕ,+,×)`, pli Bool sur `(Bool,∨,∧)`), expansion-comptage
//! A4 sur niveaux collapsés, et `union` (merge de construction Bool).

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

/// Compile un ensemble de maps en un seul diagramme (union des membres).
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

/// Exemple de la spec § 5 : `S = {{a↦0}, {a↦1}}` sur `Map([a], {0,1})` —
/// le niveau `val(a)` est collapsé, le chemin explicite est unique mais
/// `|S| = 2` (A4 : le cardinal du niveau entre dans l'agrégation).
#[test]
fn test_count_niveau_collapsed_a4_exemple_spec() {
    let schema = Schema::Map(vec![Atom(0)], Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([map_de(&[(0, 0)]), map_de(&[(0, 1)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);
    // Le niveau val(a) a été collapsé : un seul arc explicite.
    let Form::Node { arcs, .. } = entree.resolve(racine).unwrap() else {
        panic!("nœud de présence attendu")
    };
    assert_eq!(arcs.len(), 1, "un seul chemin explicite");
    let total = compter(&entree, &mut sortie, anchor, &schema, racine).unwrap();
    assert_eq!(total, 2, "A4 : |Val(val(a))| × 1 = 2");
}

/// Comptage sur un ensemble de maps à deux clés (pas de collapse ici).
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

/// Ensemble vide : le pli vaut zéro. Ensemble plein (toutes les maps) :
/// le comptage vaut le produit des cardinalités (tout niveau collapsé).
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

    // Toutes les maps sur une clé à valeurs {0,1} : {a↦0} et {a↦1}.
    let plein = BTreeSet::from([map_de(&[(0, 0)]), map_de(&[(0, 1)])]);
    let racine_plein = compiler_ensemble(&mut entree, anchor, &schema, &plein);
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, racine_plein).unwrap(),
        2
    );
}

/// Pli sur `(Bool, ∨, ∧)` : « l'ensemble est non vide » (∃-summary).
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
    assert_eq!(s_vide, BoolOu(false), "ensemble vide ⇒ faux");

    let non_vide = BTreeSet::from([map_de(&[(0, 1)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &non_vide);
    let (_, s) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
    assert_eq!(s, BoolOu(true), "ensemble non vide ⇒ vrai");
}

/// Union (merge de construction Bool, A5) : idempotente, ⊥ neutre ; ⊤ est
/// le membre « tout par défaut » (la map vide), pas un absorbant.
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
        "idempotente"
    );
    assert_eq!(
        union(&mut entree, &keys, anchor, m0, faux).unwrap(),
        m0,
        "⊥ neutre"
    );
    let les_deux = union(&mut entree, &keys, anchor, m0, m1).unwrap();
    let les_trois = union(&mut entree, &keys, anchor, les_deux, vide).unwrap();
    let mut sortie = pool(&schema);
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, les_deux).unwrap(),
        2,
        "l'union de deux membres compte 2"
    );
    assert_eq!(
        compter(&entree, &mut sortie, anchor, &schema, les_trois).unwrap(),
        3,
        "⊤ = map vide : un troisième membre, pas d'absorption"
    );
}
