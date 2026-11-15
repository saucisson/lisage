//! PBT of non-local events (M2-5, T3): for any random map and
//! any pair of keys source < destination, the transfer compiled as a
//! level-memory transduction checks the semantics `m(a) -= 1;
//! m(b) += 1` (if `m(a) ≥ 1` and `m(b)+1` in the domain), and leaves the
//! other keys unchanged.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, MemoryPolicy, NodeId};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiler, decompiler};
use lisage_core::transduction::Transducteur;
use lisage_core::transfert::TransfertNonLocal;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

/// Number of keys of the random map.
const NUMBER_CLES: u64 = 3;
/// Size of the value domain.
const SIZE_DOMAIN: u64 = 4;

type PoolRef = Arena<ReferenceController>;

/// `Map` schema with `NUMBER_CLES` keys over the domain `{0..SIZE_DOMAIN}`.
fn schema_test() -> Schema {
    Schema::Map(
        (0..NUMBER_CLES).map(Atom).collect(),
        Domain::new((0..SIZE_DOMAIN).map(Atom).collect()),
    )
}

fn new_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a map `key ↦ value` into the pool.
fn compiler_map(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    inputs: &BTreeMap<u64, u64>,
) -> NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Map(
            inputs
                .iter()
                .map(|(k, v)| (Value::Atom(Atom(*k)), Value::Atom(Atom(*v))))
                .collect(),
        ),
    )
    .unwrap()
}

/// Decompiles a node into a map `key ↦ value`.
fn decompiler_map(pool: &PoolRef, schema: &Schema, node: NodeId) -> BTreeMap<u64, u64> {
    match decompiler(pool, schema, node).unwrap() {
        Value::Map(map) => map
            .into_iter()
            .map(|(k, v)| match (k, v) {
                (Value::Atom(Atom(k)), Value::Atom(Atom(v))) => (k, v),
                _ => panic!("atoms expected"),
            })
            .collect(),
        _ => panic!("map expected"),
    }
}

proptest! {
    /// Arc semantics of the non-local transfer (T3) on a complete map
    /// (all keys present): `m(a) -= 1` if `m(a) ≥ 1`; `m(b) += 1`
    /// if moreover `m(b) + 1` is in the domain, otherwise `m(b)` unchanged.
    /// The other keys are always unchanged.
    #[test]
    fn test_transfert_semantique(
        values in prop::collection::vec(0u64..SIZE_DOMAIN, NUMBER_CLES as usize),
        source in 0u64..NUMBER_CLES,
        destination in 0u64..NUMBER_CLES,
    ) {
        prop_assume!(source < destination);
        let inputs: BTreeMap<u64, u64> = (0..NUMBER_CLES).zip(values).collect();
        let schema = schema_test();
        let anchor = Anchor(0);
        let mut pool = new_pool(&schema);
        let initial = compiler_map(&mut pool, anchor, &schema, &inputs);

        let transfert = TransfertNonLocal::new(
            &schema,
            usize::try_from(source).unwrap(),
            usize::try_from(destination).unwrap(),
        );
        let mut runner = Transducteur::with_politique(transfert, MemoryPolicy::PerLevel);
        let (result, _) = runner.run_same_pool(&mut pool, anchor, initial).unwrap();

        let obtenu = decompiler_map(&pool, &schema, result);
        let mut expected = inputs.clone();
        let value_source = inputs[&source];
        let value_destination = inputs[&destination];
        // The transfer is active only if the source has at least 1 unit.
        if value_source >= 1 {
            expected.insert(source, value_source - 1);
            if value_destination + 1 < SIZE_DOMAIN {
                expected.insert(destination, value_destination + 1);
            }
        }
        prop_assert_eq!(obtenu, expected);
    }
}
