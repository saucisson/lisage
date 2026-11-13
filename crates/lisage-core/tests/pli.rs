//! Tests of the fold and the semirings (M1-6, spec § 5, A4/A5): `Fold<S>`
//! (count over `(ℕ,+,×)`, Bool fold over `(Bool,∨,∧)`), expansion-counting
//! A4 on collapsed levels, and `union` (Bool construction merge).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{BoolOr, Fold, count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_core::transduction::Transducer;
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

/// Compiles a set of maps into a single diagram (union of the members).
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

/// Example from spec § 5: `S = {{a↦0}, {a↦1}}` on `Map([a], {0,1})` —
/// the level `val(a)` is collapsed, the explicit path is unique but
/// `|S| = 2` (A4: the cardinality of the level enters the aggregation).
#[test]
fn test_count_level_collapsed_a4_example_spec() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_of(&[(0, 0)]), map_of(&[(0, 1)])]);
    let root = compile_set(&mut input, anchor, &schema, &set);
    // The level val(a) has been collapsed: a single explicit edge.
    let Form::Node { edges, .. } = input.resolve(root).unwrap() else {
        panic!("presence node expected")
    };
    assert_eq!(edges.len(), 1, "a single explicit path");
    let total = count(&input, &mut output, anchor, &schema, root).unwrap();
    assert_eq!(total, 2, "A4 : |Val(val(a))| × 1 = 2");
}

/// Counting over a set of maps with two keys (no collapse here).
#[test]
fn test_count_set_maps() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_of(&[(0, 1), (1, 0)]), map_of(&[(0, 0)])]);
    let root = compile_set(&mut input, anchor, &schema, &set);
    let total = count(&input, &mut output, anchor, &schema, root).unwrap();
    assert_eq!(total, 2);
}

/// Empty set: the fold is zero. Full set (all maps):
/// the count is the product of the cardinalities (every level collapsed).
#[test]
fn test_count_set_empty_and_full() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);

    let empty = BTreeSet::new();
    let root_empty = compile_set(&mut input, anchor, &schema, &empty);
    assert_eq!(
        count(&input, &mut output, anchor, &schema, root_empty).unwrap(),
        0
    );

    // All maps over one key with values {0,1}: {a↦0} and {a↦1}.
    let full = BTreeSet::from([map_of(&[(0, 0)]), map_of(&[(0, 1)])]);
    let root_full = compile_set(&mut input, anchor, &schema, &full);
    assert_eq!(
        count(&input, &mut output, anchor, &schema, root_full).unwrap(),
        2
    );
}

/// Fold over `(Bool, ∨, ∧)`: "the set is not-empty" (∃-summary).
#[test]
fn test_fold_boolou_not_empty() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);

    let mut runner = Transducer::new(Fold::<BoolOr>::new(schema.clone()));
    let empty = BTreeSet::new();
    let root_empty = compile_set(&mut input, anchor, &schema, &empty);
    let (_, s_empty) = runner.run(&input, &mut output, anchor, root_empty).unwrap();
    assert_eq!(s_empty, BoolOr(false), "empty set ⇒ false");

    let not_empty = BTreeSet::from([map_of(&[(0, 1)])]);
    let root = compile_set(&mut input, anchor, &schema, &not_empty);
    let (_, s) = runner.run(&input, &mut output, anchor, root).unwrap();
    assert_eq!(s, BoolOr(true), "not-empty set ⇒ true");
}

/// Union (Bool construction merge, A5): idempotent, ⊥ neutral; ⊤ is
/// the "all default" member (the empty map), not an absorbing element.
#[test]
fn test_union_merge_construction_bool() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let anchor = Anchor(0);
    let keys = SchemaKeys::new(schema.clone());
    let m0 = compiled(&mut input, anchor, &schema, &map_of(&[(0, 0)])).unwrap();
    let m1 = compiled(&mut input, anchor, &schema, &map_of(&[(0, 1)])).unwrap();
    let false_node = input.intern(anchor, Form::Terminal(false));
    let empty = compiled(&mut input, anchor, &schema, &map_of(&[])).unwrap();

    assert_eq!(
        union(&mut input, &keys, anchor, m0, m0).unwrap(),
        m0,
        "idempotent"
    );
    assert_eq!(
        union(&mut input, &keys, anchor, m0, false_node).unwrap(),
        m0,
        "⊥ neutre"
    );
    let the_two = union(&mut input, &keys, anchor, m0, m1).unwrap();
    let the_three = union(&mut input, &keys, anchor, the_two, empty).unwrap();
    let mut output = pool(&schema);
    assert_eq!(
        count(&input, &mut output, anchor, &schema, the_two).unwrap(),
        2,
        "the union of two members counts 2"
    );
    assert_eq!(
        count(&input, &mut output, anchor, &schema, the_three).unwrap(),
        3,
        "⊤ = empty map: a third member, no absorption"
    );
}
