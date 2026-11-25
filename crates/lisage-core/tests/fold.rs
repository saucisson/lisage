//! Tests for the fold and semirings (M1-6, spec § 5, A4/A5): `Fold<S>`
//! (count over `(ℕ,+,×)`, Bool fold over `(Bool,∨,∧)`), A4 counting
//! expansion on collapsed levels, and `union` (Bool construction merge).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{BoolOr, Fold, count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::Transducer;
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

fn atom(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_from(pairs: &[(u64, u64)]) -> Value {
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

/// Spec § 5 example: `S = {{a↦0}, {a↦1}}` on `Map([a], {0,1})` — the
/// `val(a)` level is collapsed, the explicit path is unique but `|S| = 2`
/// (A4: the level cardinality enters the aggregation).
#[test]
fn test_count_level_collapsed_a4_exemple_spec() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_from(&[(0, 0)]), map_from(&[(0, 1)])]);
    let root = compiler_set(&mut input, anchor, &schema, &set);
    // The val(a) level was collapsed: a single explicit arc.
    let Form::Node { arcs, .. } = input.resolve(root).unwrap() else {
        panic!("presence node expected")
    };
    assert_eq!(arcs.len(), 1, "a single explicit path");
    let total = count(&input, &mut output, anchor, &schema, root).unwrap();
    assert_eq!(total, 2, "A4: |Val(val(a))| × 1 = 2");
}

/// Counting on a set of two-key maps (no collapse here).
#[test]
fn test_count_set_maps() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);
    let set = BTreeSet::from([map_from(&[(0, 1), (1, 0)]), map_from(&[(0, 0)])]);
    let root = compiler_set(&mut input, anchor, &schema, &set);
    let total = count(&input, &mut output, anchor, &schema, root).unwrap();
    assert_eq!(total, 2);
}

/// Empty set: the fold is zero. Full set (all maps): the count is the
/// product of the cardinalities (every level collapsed).
#[test]
fn test_count_set_empty_and_plein() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);

    let empty = BTreeSet::new();
    let root_empty = compiler_set(&mut input, anchor, &schema, &empty);
    assert_eq!(
        count(&input, &mut output, anchor, &schema, root_empty).unwrap(),
        0
    );

    // All maps over one key with values {0,1}: {a↦0} and {a↦1}.
    let plein = BTreeSet::from([map_from(&[(0, 0)]), map_from(&[(0, 1)])]);
    let root_plein = compiler_set(&mut input, anchor, &schema, &plein);
    assert_eq!(
        count(&input, &mut output, anchor, &schema, root_plein).unwrap(),
        2
    );
}

/// Fold over `(Bool, ∨, ∧)`: "the set is non-empty" (∃-summary).
#[test]
fn test_fold_bool_or_not_empty() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let mut output = pool(&schema);
    let anchor = Anchor(0);

    let mut runner = Transducer::new(Fold::<BoolOr>::new(schema.clone()));
    let empty = BTreeSet::new();
    let root_empty = compiler_set(&mut input, anchor, &schema, &empty);
    let (_, s_empty) = runner.run(&input, &mut output, anchor, root_empty).unwrap();
    assert_eq!(s_empty, BoolOr(false), "empty set ⇒ false");

    let not_empty = BTreeSet::from([map_from(&[(0, 1)])]);
    let root = compiler_set(&mut input, anchor, &schema, &not_empty);
    let (_, s) = runner.run(&input, &mut output, anchor, root).unwrap();
    assert_eq!(s, BoolOr(true), "non-empty set ⇒ true");
}

/// Union (Bool construction merge, A5): idempotent, ⊥ neutral; ⊤ is the
/// "all by default" member (the empty map), not an absorbing element.
#[test]
fn test_union_merge_construction_bool() {
    let schema = Schema::Map(vec![Atom(0)], Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = pool(&schema);
    let anchor = Anchor(0);
    let keys = SchemaKeys::new(schema.clone());
    let m0 = compiler(&mut input, anchor, &schema, &map_from(&[(0, 0)])).unwrap();
    let m1 = compiler(&mut input, anchor, &schema, &map_from(&[(0, 1)])).unwrap();
    let false_node = input.intern(anchor, Form::Terminal(false));
    let empty = compiler(&mut input, anchor, &schema, &map_from(&[])).unwrap();

    assert_eq!(
        union(&mut input, &keys, anchor, m0, m0).unwrap(),
        m0,
        "idempotent"
    );
    assert_eq!(
        union(&mut input, &keys, anchor, m0, false_node).unwrap(),
        m0,
        "⊥ neutral"
    );
    let both = union(&mut input, &keys, anchor, m0, m1).unwrap();
    let all_three = union(&mut input, &keys, anchor, both, empty).unwrap();
    let mut output = pool(&schema);
    assert_eq!(
        count(&input, &mut output, anchor, &schema, both).unwrap(),
        2,
        "the union of two members counts 2"
    );
    assert_eq!(
        count(&input, &mut output, anchor, &schema, all_three).unwrap(),
        3,
        "⊤ = empty map: a third member, no absorption"
    );
}
