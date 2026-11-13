//! Differentials of the operations on the inherited channel only (M1-6,
//! spec § 4): lane A = reference interpreter (`lisage_lang::interpreter`),
//! DD lane = `exists`/`filter`/`map` transductions of `lisage-core`.
//! Triptych C1. For filter/map, equality is structural (C1: same set ⇒
//! same node in the same pool).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::union;
use lisage_core::observer::NoOpObserver;
use lisage_core::operations::{KeyPresent, exists as exists_dd, filter as filter_dd, image};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_lang::interpreter::{exists, filter, map};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn atom(value: u64) -> Value {
    Value::Atom(Atom(value))
}

/// Small set of maps generated randomly (keys {0,1} × values {0,1}).
fn set_maps_random(maps: Vec<(u64, u64, u64, u64)>) -> BTreeSet<Value> {
    maps.into_iter()
        .map(|(a, b, c, d)| {
            let mut map = BTreeMap::new();
            if a == 1 {
                map.insert(atom(0), atom(b));
            }
            if c == 1 {
                map.insert(atom(1), atom(d));
            }
            Value::Map(map)
        })
        .collect()
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

/// Presence predicate, DD side: `p(m) = present(a)`.
fn predicate_dd(keys: &KeyPresent) -> bool {
    keys.contains(&0)
}

/// Same predicate, lane A side (reference interpreter).
fn predicate_lang(value: &Value) -> bool {
    match value {
        Value::Map(map) => map.contains_key(&atom(0)),
        _ => false,
    }
}

proptest! {
    /// C1 differential: exists DD (fold `(Bool,∨,∧)`) == exists interpreter.
    #[test]
    fn test_c1_differential_exists_lane_a_vs_dd(
        maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)
    ) {
        let schema = Schema::Map(
            vec![Atom(0), Atom(1)],
            Domain::new(vec![Atom(0), Atom(1)]),
        );
        let set = set_maps_random(maps);
        let mut input = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut output = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let root = compile_set(&mut input, anchor, &schema, &set);
        let dd = exists_dd(
            &input,
            &mut output,
            anchor,
            &schema,
            root,
            Box::new(predicate_dd),
        )
        .unwrap();
        prop_assert_eq!(dd, exists(&set, predicate_lang), "DD lane != lane A (exists)");
    }

    /// C1 differential: filter DD == set filtered by lane A,
    /// compared structurally (C1: same set ⇒ same node).
    #[test]
    fn test_c1_differential_filter_lane_a_vs_dd(
        maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)
    ) {
        let schema = Schema::Map(
            vec![Atom(0), Atom(1)],
            Domain::new(vec![Atom(0), Atom(1)]),
        );
        let set = set_maps_random(maps);
        let mut input = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut output = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let root = compile_set(&mut input, anchor, &schema, &set);
        let dd = filter_dd(
            &input,
            &mut output,
            anchor,
            &schema,
            root,
            Box::new(predicate_dd),
        )
        .unwrap();
        let expected_set = filter(&set, predicate_lang);
        let expected = compile_set(&mut output, anchor, &schema, &expected_set);
        prop_assert_eq!(dd, expected, "DD lane != lane A (filter), C1 structural");
    }

    /// C1 differential: map(f) DD (relabeling) == map interpreter on a
    /// random set of atoms, compared structurally.
    #[test]
    fn test_c1_differential_map_lane_a_vs_dd(
        atoms in prop::collection::btree_set(0u64..3, 0..4)
    ) {
        let schema = Schema::Atom(Domain::new(vec![Atom(0), Atom(1), Atom(2)]));
        let set: BTreeSet<Value> = atoms.iter().map(|v| atom(*v)).collect();
        let mut input = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut output = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let root = compile_set(&mut input, anchor, &schema, &set);
        let dd = image(&input, &mut output, anchor, root, Box::new(|v| (v + 1) % 3)).unwrap();
        let expected_set = map(&set, |v| match v {
            Value::Atom(a) => atom((a.0 + 1) % 3),
            other => other.clone(),
        });
        let expected = compile_set(&mut output, anchor, &schema, &expected_set);
        prop_assert_eq!(dd, expected, "DD lane != lane A (map), C1 structural");
    }
}
