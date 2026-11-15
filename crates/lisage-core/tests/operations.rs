//! Tests for the inherited-channel-only operations (M1-6, spec § 4):
//! `exists` (fold `(Bool,∨,∧)`), `filter` (guard), `map` (atom relabeling).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::observer::NoOpObserver;
use lisage_core::operations::{KeyPresentes, existe, filtrer, image};
use lisage_core::pli::union;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

fn atome(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(key, value)| (atome(*key), atome(*value)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

fn pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

fn compiler_set(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    set: &BTreeSet<Value>,
) -> NodeId {
    let keys = SchemaKeys::new(schema.clone());
    let mut result = pool.intern(anchor, Form::Terminal(false));
    for membre in set {
        let n = compiler(pool, anchor, schema, membre).unwrap();
        result = union(pool, &keys, anchor, result, n).unwrap();
    }
    result
}

fn schema_map() -> Schema {
    Schema::Map(vec![Atom(0), Atom(1)], Domain::new(vec![Atom(0), Atom(1)]))
}

/// Presence predicate: `p(m) = present(a)`.
fn predicat_exemple_spec(keys: &KeyPresentes) -> bool {
    keys.contains(&0)
}

/// `exists(p)`: fold over `(Bool, ∨, ∧)`, verdict resolved at the terminal.
#[test]
fn test_existe_predicat_exemple_spec() {
    let schema = schema_map();
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_de(&[(0, 1)]), map_de(&[(1, 0)])]);
    let root = compiler_set(&mut input, anchor, &schema, &set);

    let verdict = existe(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(predicat_exemple_spec),
    )
    .unwrap();
    assert!(verdict, "a↦1 satisfies p (present(a))");

    let aucun = existe(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(|keys: &KeyPresentes| keys.contains(&0) && keys.contains(&1)),
    )
    .unwrap();
    assert!(!aucun, "no member has both a and b present");
}

/// `filter(p)`: guard, the output diagram is the filtered set (canonical:
/// same set ⇒ same node, C1).
#[test]
fn test_filtre_guard() {
    let schema = schema_map();
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_de(&[(0, 1)]), map_de(&[(1, 0)])]);
    let root = compiler_set(&mut input, anchor, &schema, &set);

    // Guard "key 0 present".
    let filtre = filtrer(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(|keys: &KeyPresentes| keys.contains(&0)),
    )
    .unwrap();
    let expected = compiler_set(
        &mut output,
        anchor,
        &schema,
        &BTreeSet::from([map_de(&[(0, 1)])]),
    );
    assert_eq!(filtre, expected, "C1: same filtered set ⇒ same node");
}

/// `map(f)`: relabeling of the atoms of a set (schema `Atom`).
#[test]
fn test_image_reetiquetage_atomes() {
    let schema = Schema::Atom(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([atome(0), atome(2)]);
    let root = compiler_set(&mut input, anchor, &schema, &set);

    // f(v) = (v + 1) mod 3 : {0, 2} ↦ {1, 0}.
    let image = image(&input, &mut output, anchor, root, Box::new(|v| (v + 1) % 3)).unwrap();
    let expected = compiler_set(
        &mut output,
        anchor,
        &schema,
        &BTreeSet::from([atome(1), atome(0)]),
    );
    assert_eq!(image, expected, "C1: same image ⇒ same node");
}

/// `exists` on a non-Map schema: typed error `SchemaInexpected`, no panic at
/// the public boundary.
#[test]
fn test_existe_schema_non_map_error_typee() {
    let schema = Schema::Atom(Domain::new(vec![Atom(0)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let root = input.intern(anchor, Form::Terminal(false));

    let result = existe(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(|_: &KeyPresentes| true),
    );
    assert_eq!(
        result.unwrap_err(),
        CoreError::SchemaInexpected {
            expected: "Map",
            recu: "Atom"
        }
    );
}

/// `filter` on a node whose level exceeds the schema: typed error
/// `LevelHorsSchema` (instead of an out-of-bounds index).
#[test]
fn test_filtre_level_hors_schema_error_typee() {
    let schema = schema_map();
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let terminal_faux = input.intern(anchor, Form::Terminal(false));
    let terminal_vrai = input.intern(anchor, Form::Terminal(true));
    // Level 4 outside the schema (2 keys ⇒ levels 0..3); distinct arcs to
    // avoid any collapse at interning.
    let malformed = input.intern(
        anchor,
        Form::Node {
            level: Level(4),
            arcs: BTreeMap::from([(0, terminal_faux), (1, terminal_vrai)]),
        },
    );

    let result = filtrer(
        &input,
        &mut output,
        anchor,
        &schema,
        malformed,
        Box::new(|_: &KeyPresentes| true),
    );
    assert_eq!(result.unwrap_err(), CoreError::LevelHorsSchema(Level(4)));
}
