//! Reference schema tests (M1-3): universe sweep compilation
//! (one presence level per key, δ = absent, no marker).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{
    ABSENT, Domain, PRESENT, ReferenceController, Schema, SchemaError, compiled, decompile,
};
use lisage_lang::value::{Atom, Value};

fn domain(values: &[u64]) -> Domain {
    Domain::new(values.iter().map(|&v| Atom(v)).collect())
}

fn arena(schema: Schema) -> Arena<ReferenceController> {
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

fn atom(v: u64) -> Value {
    Value::Atom(Atom(v))
}

/// Example from spec § 1: K = {a, b}, V = {0, 1}, the map {a↦1, b↦0}
/// compiles into a chain of presences: k_a? → val(a)=1 → k_b? → val(b)=0 → ⊤.
#[test]
fn test_compilation_map_example_spec() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let map = Value::Map(BTreeMap::from([(atom(0), atom(1)), (atom(1), atom(0))]));
    let root = compiled(&mut pool, anchor, &schema, &map).unwrap();

    let terminal = pool.intern(anchor, Form::Terminal(true));
    let Form::Node { level, edges } = pool.resolve(root).unwrap() else {
        panic!("the root must be a presence node");
    };
    assert_eq!(*level, Level(0));
    let node_value_a = edges[&PRESENT];
    let Form::Node { level, edges } = pool.resolve(node_value_a).unwrap() else {
        panic!("value node expected");
    };
    assert_eq!(*level, Level(1));
    assert_eq!(edges.len(), 1);
    let node_presence_b = edges[&PRESENT];
    let Form::Node { level, edges } = pool.resolve(node_presence_b).unwrap() else {
        panic!("presence node expected");
    };
    assert_eq!(*level, Level(2));
    let node_value_b = edges[&PRESENT];
    let Form::Node { level, edges } = pool.resolve(node_value_b).unwrap() else {
        panic!("value node expected");
    };
    assert_eq!(*level, Level(3));
    assert_eq!(edges, &BTreeMap::from([(ABSENT, terminal)]));
}

/// R-collapse skips the presence levels of absent elements (ZDD
/// style): {a} over {a, b} compiles into a single node k_a? → present → ⊤.
#[test]
fn test_set_absent_collapses() {
    let schema = Schema::Set(domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let set = Value::Set(BTreeSet::from([atom(0)]));
    let root = compiled(&mut pool, anchor, &schema, &set).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    assert_eq!(
        pool.resolve(root).unwrap(),
        &Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(PRESENT, terminal)]),
        }
    );
}

/// The empty map compiles to ⊤; an absent key skips its presence level.
#[test]
fn test_map_empty_and_key_absent() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let empty = Value::Map(BTreeMap::new());
    let root = compiled(&mut pool, anchor, &schema, &empty).unwrap();
    assert_eq!(pool.resolve(root).unwrap(), &Form::Terminal(true));

    // {b ↦ 0}: the presence level of a is skipped (R-collapse).
    let map = Value::Map(BTreeMap::from([(atom(1), atom(0))]));
    let root = compiled(&mut pool, anchor, &schema, &map).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    let node_value_b = pool.intern(
        anchor,
        Form::Node {
            level: Level(3),
            edges: BTreeMap::from([(ABSENT, terminal)]),
        },
    );
    assert_eq!(
        pool.resolve(root).unwrap(),
        &Form::Node {
            level: Level(2),
            edges: BTreeMap::from([(PRESENT, node_value_b)]),
        }
    );
}

/// Typed errors: incompatible type, atom outside the domain.
#[test]
fn test_errors_typed() {
    let schema = Schema::Set(domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let error = compiled(&mut pool, anchor, &schema, &atom(0)).unwrap_err();
    assert_eq!(
        error,
        SchemaError::TypeMismatch {
            expected: "set",
            recu: "atom"
        }
    );
    let out = Value::Set(BTreeSet::from([atom(9)]));
    let error = compiled(&mut pool, anchor, &schema, &out).unwrap_err();
    assert_eq!(error, SchemaError::OutDomain(Atom(9)));
}

/// Atom(D) schema: a single level, Val = D.
#[test]
fn test_schema_atom() {
    let schema = Schema::Atom(domain(&[0, 1, 2]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let root = compiled(&mut pool, anchor, &schema, &atom(2)).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    assert_eq!(
        pool.resolve(root).unwrap(),
        &Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(2, terminal)]),
        }
    );
}

/// The empty set compiles to the acceptance terminal ⊤ (empty path),
/// and the full set to the complete chain of presences.
#[test]
fn test_set_empty_and_full() {
    let schema = Schema::Set(domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let empty = Value::Set(BTreeSet::new());
    let root_empty = compiled(&mut pool, anchor, &schema, &empty).unwrap();
    assert_eq!(pool.resolve(root_empty).unwrap(), &Form::Terminal(true));
    let full = Value::Set(BTreeSet::from([atom(0), atom(1)]));
    let root_full = compiled(&mut pool, anchor, &schema, &full).unwrap();
    let terminal = pool.intern(anchor, Form::Terminal(true));
    let node_presence_b = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            edges: BTreeMap::from([(PRESENT, terminal)]),
        },
    );
    assert_eq!(
        pool.resolve(root_full).unwrap(),
        &Form::Node {
            level: Level(0),
            edges: BTreeMap::from([(PRESENT, node_presence_b)]),
        }
    );
}

/// Map validation distinguishes key outside the domain from value outside
/// domain (the error carries the offending atom).
#[test]
fn test_error_value_out_domain_map() {
    let schema = Schema::Map(vec![Atom(0)], domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let map = Value::Map(BTreeMap::from([(atom(0), atom(9))]));
    let error = compiled(&mut pool, anchor, &schema, &map).unwrap_err();
    assert_eq!(error, SchemaError::OutDomain(Atom(9)));
    let map = Value::Map(BTreeMap::from([(atom(7), atom(0))]));
    let error = compiled(&mut pool, anchor, &schema, &map).unwrap_err();
    assert_eq!(error, SchemaError::OutDomain(Atom(7)));
}

/// Round-trip: decompile(compiled(v)) = v.
#[test]
fn test_round_trip() {
    let schema = Schema::Map(vec![Atom(0), Atom(1)], domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let map = Value::Map(BTreeMap::from([(atom(0), atom(1))]));
    let root = compiled(&mut pool, anchor, &schema, &map).unwrap();
    assert_eq!(decompile(&pool, &schema, root).unwrap(), map);

    let schema = Schema::Set(domain(&[0, 1, 2]));
    let mut pool = arena(schema.clone());
    let set = Value::Set(BTreeSet::from([atom(0), atom(2)]));
    let root = compiled(&mut pool, anchor, &schema, &set).unwrap();
    assert_eq!(decompile(&pool, &schema, root).unwrap(), set);

    let schema = Schema::Atom(domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let root = compiled(&mut pool, anchor, &schema, &atom(1)).unwrap();
    assert_eq!(decompile(&pool, &schema, root).unwrap(), atom(1));
}

/// C1 applied to schemas: two distinct values compiled to distinct
/// nodes; the same value compiles twice to the same node.
#[test]
fn test_canonicity_compilation() {
    let schema = Schema::Set(domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let a = Value::Set(BTreeSet::from([atom(0)]));
    let b = Value::Set(BTreeSet::from([atom(1)]));
    let root_a1 = compiled(&mut pool, anchor, &schema, &a).unwrap();
    let root_a2 = compiled(&mut pool, anchor, &schema, &a).unwrap();
    let root_b = compiled(&mut pool, anchor, &schema, &b).unwrap();
    assert_eq!(root_a1, root_a2);
    assert_ne!(root_a1, root_b);
}

/// M2-1: Seq — compilation by positions and round-trip.
#[test]
fn test_seq_compilation_round_trip() {
    let schema = Schema::Seq(domain(&[0, 1, 2]), 3);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);

    // Empty sequence.
    let seq_empty = Value::Seq(vec![]);
    let root = compiled(&mut pool, anchor, &schema, &seq_empty).unwrap();
    assert_eq!(decompile(&pool, &schema, root).unwrap(), seq_empty);

    // Sequence [1, 2].
    let seq = Value::Seq(vec![atom(1), atom(2)]);
    let root = compiled(&mut pool, anchor, &schema, &seq).unwrap();
    assert_eq!(decompile(&pool, &schema, root).unwrap(), seq);

    // Full sequence [0, 1, 2].
    let seq_full = Value::Seq(vec![atom(0), atom(1), atom(2)]);
    let root = compiled(&mut pool, anchor, &schema, &seq_full).unwrap();
    assert_eq!(decompile(&pool, &schema, root).unwrap(), seq_full);
}

/// M2-1: Seq — canonicity (same sequence = same node).
#[test]
fn test_seq_canonicity() {
    let schema = Schema::Seq(domain(&[0, 1]), 2);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let seq = Value::Seq(vec![atom(0), atom(1)]);
    let r1 = compiled(&mut pool, anchor, &schema, &seq).unwrap();
    let r2 = compiled(&mut pool, anchor, &schema, &seq).unwrap();
    assert_eq!(r1, r2);
    let other = Value::Seq(vec![atom(1), atom(0)]);
    let r3 = compiled(&mut pool, anchor, &schema, &other).unwrap();
    assert_ne!(r1, r3);
}

/// M2-1: Seq — error if length > max.
#[test]
fn test_seq_too_long() {
    let schema = Schema::Seq(domain(&[0, 1]), 2);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let seq = Value::Seq(vec![atom(0), atom(1), atom(0)]);
    assert!(compiled(&mut pool, anchor, &schema, &seq).is_err());
}

/// M2-1: Seq — error if atom outside domain.
#[test]
fn test_seq_out_domain() {
    let schema = Schema::Seq(domain(&[0, 1]), 2);
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let seq = Value::Seq(vec![atom(5)]);
    assert!(matches!(
        compiled(&mut pool, anchor, &schema, &seq),
        Err(SchemaError::OutDomain(_))
    ));
}

/// M2-1: Seq — the end marker is a required key: `default()` returns
/// `None` for all the levels of a Seq schema (no absorbable default).
#[test]
fn test_seq_marker_end_key_required() {
    use lisage_algebra::interface::{KeyAlgebra, Level};
    use lisage_core::schema::SchemaKeys;

    let schema = Schema::Seq(domain(&[0, 1, 2]), 3);
    let keys = SchemaKeys::new(schema);
    for level in 0..4 {
        assert_eq!(
            keys.default(Level(level)),
            None,
            "level {level}: no default (required key)"
        );
    }
    // Cardinality: |D|+1 per position, 2 at the marker level.
    assert_eq!(keys.cardinality(Level(0)), 4);
    assert_eq!(keys.cardinality(Level(3)), 2);
}

/// M2-2: Tag — typed union compilation and round-trip (C2).
#[test]
fn test_tag_compilation_round_trip() {
    let schema = Schema::Tag(vec![Atom(0), Atom(1), Atom(2)], domain(&[10, 20]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);

    for discriminant in [0u64, 1, 2] {
        for value in [10u64, 20] {
            let tag = Value::Tag(Atom(discriminant), Box::new(atom(value)));
            let root = compiled(&mut pool, anchor, &schema, &tag).unwrap();
            assert_eq!(decompile(&pool, &schema, root).unwrap(), tag);
        }
    }
}

/// M2-2: Tag — canonicity and a single useful edge (C2).
#[test]
fn test_tag_canonicity_c2() {
    use lisage_algebra::interface::Pool;
    let schema = Schema::Tag(vec![Atom(0), Atom(1)], domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let tag = Value::Tag(Atom(0), Box::new(atom(1)));
    let r1 = compiled(&mut pool, anchor, &schema, &tag).unwrap();
    let r2 = compiled(&mut pool, anchor, &schema, &tag).unwrap();
    assert_eq!(r1, r2);
    // A single edge on the discriminant node.
    if let Ok(Form::Node { edges, .. }) = pool.resolve(r1) {
        assert_eq!(edges.len(), 1, "C2: a single useful edge per node");
    } else {
        panic!("node expected");
    }
}

/// M2-2: Tag — error discriminant outside list.
#[test]
fn test_tag_discriminant_invalid() {
    let schema = Schema::Tag(vec![Atom(0), Atom(1)], domain(&[0, 1]));
    let mut pool = arena(schema.clone());
    let anchor = Anchor(0);
    let tag = Value::Tag(Atom(5), Box::new(atom(0)));
    assert!(matches!(
        compiled(&mut pool, anchor, &schema, &tag),
        Err(SchemaError::OutDomain(_))
    ));
}
