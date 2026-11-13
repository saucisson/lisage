//! Differential of the fold (M1-6, spec § 5, A4): lane A = reference
//! interpreter (`lisage_lang::interpreter::count`), DD lane = `Fold<Naturel>`
//! (count over `(ℕ,+,×)` with A4 expansion of collapsed levels) on the
//! set of maps compiled by union. Triptych C1.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_lang::interpreter::count as count_reference;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn atom(value: u64) -> Value {
    Value::Atom(Atom(value))
}

/// Small set of maps generated randomly (keys {0,1} × values {0,1}).
fn set_random(maps: Vec<(u64, u64, u64, u64)>) -> BTreeSet<Value> {
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

proptest! {
    /// C1 differential: DD count (lane B, fold `(ℕ,+,×)` + A4) == count
    /// interpreter (lane A) on a random set of maps. The random sets produce
    /// collapsed levels: the A4 path is covered by the PBT.
    #[test]
    fn test_c1_differential_count_lane_a_vs_dd(
        maps in prop::collection::vec((0u64..2, 0u64..2, 0u64..2, 0u64..2), 0..8)
    ) {
        let schema = Schema::Map(
            vec![Atom(0), Atom(1)],
            Domain::new(vec![Atom(0), Atom(1)]),
        );
        let set = set_random(maps);
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
        let dd = count(&input, &mut output, anchor, &schema, root).unwrap();
        prop_assert_eq!(dd, count_reference(&set), "DD lane (fold A4) != lane A (count)");
    }
}
