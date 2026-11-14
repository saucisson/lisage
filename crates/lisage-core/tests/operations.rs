//! Tests for the inherited-channel-only operations (M1-6, spec § 4): `exists`
//! (fold `(Bool,∨,∧)`), `filter` (guard), `map` (atom relabeling).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
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

/// Presence predicate: `p(m) = present(a)`.
fn predicat_exemple_spec(cles: &ClePresentes) -> bool {
    cles.contains(&0)
}

/// `exists(p)`: fold over `(Bool, ∨, ∧)`, verdict resolved at the terminal.
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
    assert!(verdict, "a↦1 satisfies p (present(a))");

    let aucun = existe(
        &entree,
        &mut sortie,
        anchor,
        &schema,
        racine,
        Box::new(|cles: &ClePresentes| cles.contains(&0) && cles.contains(&1)),
    )
    .unwrap();
    assert!(!aucun, "no member has both a and b present");
}

/// `filter(p)`: guard, the output diagram is the filtered set (canonical:
/// same set ⇒ same node, C1).
#[test]
fn test_filtre_garde() {
    let schema = schema_map();
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([map_de(&[(0, 1)]), map_de(&[(1, 0)])]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);

    // Guard "key 0 present".
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
    assert_eq!(filtre, attendu, "C1: same filtered set ⇒ same node");
}

/// `map(f)`: relabeling of the atoms of a set (schema `Atom`).
#[test]
fn test_image_reetiquetage_atomes() {
    let schema = Schema::Atom(Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let ensemble = BTreeSet::from([atome(0), atome(2)]);
    let racine = compiler_ensemble(&mut entree, anchor, &schema, &ensemble);

    // f(v) = (v + 1) mod 3: {0, 2} ↦ {1, 0}.
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
    assert_eq!(image, attendu, "C1: same image ⇒ same node");
}

/// `exists` on a non-Map schema: typed error `SchemaInattendu`, no panic at
/// the public boundary.
#[test]
fn test_existe_schema_non_map_erreur_typee() {
    let schema = Schema::Atom(Domaine::nouveau(vec![Atom(0)]));
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let racine = entree.intern(anchor, Form::Terminal(false));

    let resultat = existe(
        &entree,
        &mut sortie,
        anchor,
        &schema,
        racine,
        Box::new(|_: &ClePresentes| true),
    );
    assert_eq!(
        resultat.unwrap_err(),
        CoreError::SchemaInattendu {
            attendu: "Map",
            recu: "Atom"
        }
    );
}

/// `filter` on a node whose level exceeds the schema: typed error
/// `NiveauHorsSchema` (instead of an out-of-bounds index).
#[test]
fn test_filtre_niveau_hors_schema_erreur_typee() {
    let schema = schema_map();
    let mut entree = pool(&schema);
    let mut sortie = pool(&schema);
    let anchor = Anchor(0);
    let terminal_faux = entree.intern(anchor, Form::Terminal(false));
    let terminal_vrai = entree.intern(anchor, Form::Terminal(true));
    // Level 4 outside the schema (2 keys ⇒ levels 0..3); distinct arcs to
    // avoid any collapse at interning.
    let malforme = entree.intern(
        anchor,
        Form::Node {
            level: Level(4),
            arcs: BTreeMap::from([(0, terminal_faux), (1, terminal_vrai)]),
        },
    );

    let resultat = filtrer(
        &entree,
        &mut sortie,
        anchor,
        &schema,
        malforme,
        Box::new(|_: &ClePresentes| true),
    );
    assert_eq!(resultat.unwrap_err(), CoreError::NiveauHorsSchema(Level(4)));
}
