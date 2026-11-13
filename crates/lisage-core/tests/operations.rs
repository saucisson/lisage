//! Tests of the inherited-channel-only operations (M1-6, spec § 4): `exists`
//! (fold `(Bool,∨,∧)`), `filter` (guard), `map` (atom relabeling).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::fold::union;
use lisage_core::observer::NoOpObserver;
use lisage_core::operations::{KeyPresent, exists, filter, image};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

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

fn pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

fn compile_set(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    set: &BTreeSet<Value>,
) -> NodeId {
    let keys = SchemaKeys::new(schema.clone());
    let mut result = pool.intern(anchor, Form::Terminal(false));
    for membre in set {
        let n = compiled(pool, anchor, schema, membre).unwrap();
        result = union(pool, &keys, anchor, result, n).unwrap();
    }
    result
}

fn schema_map() -> Schema {
    Schema::Map(vec![Atom(0), Atom(1)], Domain::new(vec![Atom(0), Atom(1)]))
}

/// Presence predicate: `p(m) = present(a)`.
fn predicate_example_spec(keys: &KeyPresent) -> bool {
    keys.contains(&0)
}

/// `exists(p)`: fold over `(Bool, ∨, ∧)`, verdict resolved at the terminal.
#[test]
fn test_exists_predicate_example_spec() {
    let schema = schema_map();
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_of(&[(0, 1)]), map_of(&[(1, 0)])]);
    let root = compile_set(&mut input, anchor, &schema, &set);

    let verdict = exists(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(predicate_example_spec),
    )
    .unwrap();
    assert!(verdict, "a↦1 satisfies p (present(a))");

    let none = exists(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(|keys: &KeyPresent| keys.contains(&0) && keys.contains(&1)),
    )
    .unwrap();
    assert!(!none, "no member has both a and b present");
}

/// `filter(p)`: guard, the output diagram is the filtered set
/// (canonical: same set ⇒ same node, C1).
#[test]
fn test_filter_keeps() {
    let schema = schema_map();
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_of(&[(0, 1)]), map_of(&[(1, 0)])]);
    let root = compile_set(&mut input, anchor, &schema, &set);

    // Keeps "key 0 present".
    let filter = filter(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(|keys: &KeyPresent| keys.contains(&0)),
    )
    .unwrap();
    let expected = compile_set(
        &mut output,
        anchor,
        &schema,
        &BTreeSet::from([map_of(&[(0, 1)])]),
    );
    assert_eq!(filter, expected, "C1: same filtered set ⇒ same node");
}

/// `map(f)`: relabeling of the atoms of a set (schema `Atom`).
#[test]
fn test_image_relabeling_atoms() {
    let schema = Schema::Atom(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([atom(0), atom(2)]);
    let root = compile_set(&mut input, anchor, &schema, &set);

    // f(v) = (v + 1) mod 3 : {0, 2} ↦ {1, 0}.
    let image = image(&input, &mut output, anchor, root, Box::new(|v| (v + 1) % 3)).unwrap();
    let expected = compile_set(
        &mut output,
        anchor,
        &schema,
        &BTreeSet::from([atom(1), atom(0)]),
    );
    assert_eq!(image, expected, "C1: same image ⇒ same node");
}

/// `exists` on a not-Map schema: typed error `SchemaUnexpected`, no
/// panic at the public boundary.
#[test]
fn test_exists_schema_not_map_error_typed() {
    let schema = Schema::Atom(Domain::new(vec![Atom(0)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let root = input.intern(anchor, Form::Terminal(false));

    let result = exists(
        &input,
        &mut output,
        anchor,
        &schema,
        root,
        Box::new(|_: &KeyPresent| true),
    );
    assert_eq!(
        result.unwrap_err(),
        CoreError::SchemaUnexpected {
            expected: "Map",
            recu: "Atom"
        }
    );
}

/// `filter` on a node whose level exceeds the schema: typed error
/// `LevelOutSchema` (instead of an out-of-bounds index).
#[test]
fn test_filter_level_out_schema_error_typed() {
    let schema = schema_map();
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let terminal_false = input.intern(anchor, Form::Terminal(false));
    let terminal_true = input.intern(anchor, Form::Terminal(true));
    // Level 4 outside the schema (2 keys ⇒ levels 0..3); distinct edges to
    // avoid any collapse at interning.
    let malforme = input.intern(
        anchor,
        Form::Node {
            level: Level(4),
            edges: BTreeMap::from([(0, terminal_false), (1, terminal_true)]),
        },
    );

    let result = filter(
        &input,
        &mut output,
        anchor,
        &schema,
        malforme,
        Box::new(|_: &KeyPresent| true),
    );
    assert_eq!(result.unwrap_err(), CoreError::LevelOutSchema(Level(4)));
}
