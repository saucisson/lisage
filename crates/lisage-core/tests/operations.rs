//! Tests des opérations à canal hérité seul (M1-6, spec § 4) : `exists`
//! (pli `(Bool,∨,∧)`), `filter` (garde), `map` (réétiquetage d'atomes).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::operations::{ClePresentes, existe, filtrer, image};
use lisage_core::pli::union;
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
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

fn schema_map() -> Schema {
    Schema::Map(
        vec![Atom(0), Atom(1)],
        Domaine::nouveau(vec![Atom(0), Atom(1)]),
    )
}

/// Prédicat de présence : `p(m) = présent(a)`.
fn predicat_exemple_spec(cles: &ClePresentes) -> bool {
    cles.contains(&0)
}

/// `exists(p)` : pli sur `(Bool, ∨, ∧)`, verdict résolu au terminal.
#[test]
fn test_existe_predicat_exemple_spec() {
    let schema = schema_map();
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([map_de(&[(0, 1)]), map_de(&[(1, 0)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);

    let verdict = existe(
        &entree,
        &mut sortie,
        anchor,
        &schema,
        racine,
        Box::new(predicat_exemple_spec),
    )
    .unwrap();
    assert!(verdict, "a↦1 satisfait p (présent(a))");

    let aucun = existe(
        &entree,
        &mut sortie,
        anchor,
        &schema,
        racine,
        Box::new(|cles: &ClePresentes| cles.contains(&0) && cles.contains(&1)),
    )
    .unwrap();
    assert!(!aucun, "aucun membre n'a a et b présents");
}

/// `filter(p)` : garde, le diagramme de sortie est l'ensemble filtré
/// (canonique : même ensemble ⇒ même nœud, C1).
#[test]
fn test_filtre_garde() {
    let schema = schema_map();
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([map_de(&[(0, 1)]), map_de(&[(1, 0)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);

    // Garde « clé 0 présente ».
    let filtre = filtrer(
        &entree,
        &mut sortie,
        anchor,
        &schema,
        racine,
        Box::new(|cles: &ClePresentes| cles.contains(&0)),
    )
    .unwrap();
    let attendu = compiler_ensemble(
        &mut sortie,
        anchor,
        &schema,
        &BTreeSet::from([map_de(&[(0, 1)])]),
    );
    assert_eq!(filtre, attendu, "C1 : même ensemble filtré ⇒ même nœud");
}

/// `map(f)` : réétiquetage des atomes d'un ensemble (schéma `Atom`).
#[test]
fn test_image_reetiquetage_atomes() {
    let schema = Schema::Atom(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([atome(0), atome(2)]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);

    // f(v) = (v + 1) mod 3 : {0, 2} ↦ {1, 0}.
    let image = image(
        &entree,
        &mut sortie,
        anchor,
        racine,
        Box::new(|v| (v + 1) % 3),
    )
    .unwrap();
    let attendu = compiler_ensemble(
        &mut sortie,
        anchor,
        &schema,
        &BTreeSet::from([atome(1), atome(0)]),
    );
    assert_eq!(image, attendu, "C1 : même image ⇒ même nœud");
}
