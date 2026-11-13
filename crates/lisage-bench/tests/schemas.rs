//! PBT of the reference schemas (M1-3): round-trip compilation →
//! decompilation, and canonicity (distinct values ⇒ distinct nodes).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::Anchor;
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiled, decompile};
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;

fn domain(values: Vec<u64>) -> Domain {
    Domain::new(values.into_iter().map(Atom).collect())
}

fn atom(v: u64) -> Value {
    Value::Atom(Atom(v))
}

fn schema_set() -> BoxedStrategy<(Schema, Value)> {
    (1u8..5, prop::collection::vec(0u8..5, 0..6))
        .prop_map(|(size, elements)| {
            let domain = domain((0..u64::from(size)).collect());
            let set: BTreeSet<Value> = elements
                .into_iter()
                .filter(|&e| u64::from(e) < u64::from(size))
                .map(|e| atom(u64::from(e)))
                .collect();
            (Schema::Set(domain), Value::Set(set))
        })
        .boxed()
}

fn schema_map() -> BoxedStrategy<(Schema, Value)> {
    (
        prop::collection::vec(0u8..4, 1..4),
        prop::collection::vec((0u8..4, 0u8..3), 0..4),
    )
        .prop_map(|(keys, pairs)| {
            let keys: Vec<Atom> = keys.into_iter().map(|c| Atom(u64::from(c))).collect();
            let domain_v = domain(vec![0, 1, 2]);
            let map: BTreeMap<Value, Value> = pairs
                .into_iter()
                .filter(|(c, _)| keys.contains(&Atom(u64::from(*c))))
                .map(|(c, v)| (atom(u64::from(c)), atom(u64::from(v))))
                .collect();
            (Schema::Map(keys, domain_v), Value::Map(map))
        })
        .boxed()
}

fn schema_atom() -> BoxedStrategy<(Schema, Value)> {
    (1u8..4, 0u8..4)
        .prop_map(|(size, v)| {
            let domain = domain((0..u64::from(size)).collect());
            let value = atom(u64::from(v) % u64::from(size));
            (Schema::Atom(domain), value)
        })
        .boxed()
}

/// Two valid values for the same schema.
fn schema_and_two_values() -> BoxedStrategy<(Schema, Value, Value)> {
    prop_oneof![
        (
            1u8..5,
            prop::collection::vec(0u8..5, 0..6),
            prop::collection::vec(0u8..5, 0..6)
        )
            .prop_map(|(size, ea, eb)| {
                let domain = domain((0..u64::from(size)).collect());
                let mk = |elements: Vec<u8>| {
                    Value::Set(
                        elements
                            .into_iter()
                            .filter(|&e| u64::from(e) < u64::from(size))
                            .map(|e| atom(u64::from(e)))
                            .collect(),
                    )
                };
                (Schema::Set(domain), mk(ea), mk(eb))
            }),
        (
            prop::collection::vec(0u8..4, 1..4),
            prop::collection::vec((0u8..4, 0u8..3), 0..4),
            prop::collection::vec((0u8..4, 0u8..3), 0..4),
        )
            .prop_map(|(keys, pa, pb)| {
                let keys: Vec<Atom> = keys.into_iter().map(|c| Atom(u64::from(c))).collect();
                let domain_v = domain(vec![0, 1, 2]);
                let keys_filter = keys.clone();
                let mk = move |pairs: Vec<(u8, u8)>| {
                    Value::Map(
                        pairs
                            .into_iter()
                            .filter(|(c, _)| keys_filter.contains(&Atom(u64::from(*c))))
                            .map(|(c, v)| (atom(u64::from(c)), atom(u64::from(v))))
                            .collect(),
                    )
                };
                (Schema::Map(keys, domain_v), mk(pa), mk(pb))
            }),
        (1u8..4, 0u8..4, 0u8..4).prop_map(|(size, va, vb)| {
            let domain = domain((0..u64::from(size)).collect());
            (
                Schema::Atom(domain),
                atom(u64::from(va) % u64::from(size)),
                atom(u64::from(vb) % u64::from(size)),
            )
        }),
    ]
    .boxed()
}

proptest! {
    /// Round-trip: decompile(compiled(v)) = v for the three bounded schemas.
    #[test]
    fn test_round_trip_compilation((schema, value) in prop_oneof![schema_set(), schema_map(), schema_atom()]) {
        let mut pool = Arena::new(ReferenceController::new(schema.clone()), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let root = compiled(&mut pool, anchor, &schema, &value).unwrap();
        prop_assert_eq!(decompile(&pool, &schema, root).unwrap(), value);
    }

    /// Canonicity (C1): two distinct values compiled to distinct nodes;
    /// the same value compiles twice to the same node.
    #[test]
    fn test_canonicity_compilation((schema, value_a, value_b) in schema_and_two_values()) {
        let mut pool = Arena::new(ReferenceController::new(schema.clone()), Box::new(NoOpObserver));
        let anchor = Anchor(0);
        let root_a1 = compiled(&mut pool, anchor, &schema, &value_a).unwrap();
        let root_a2 = compiled(&mut pool, anchor, &schema, &value_a).unwrap();
        let root_b = compiled(&mut pool, anchor, &schema, &value_b).unwrap();
        prop_assert_eq!(root_a1, root_a2);
        if value_a != value_b {
            prop_assert_ne!(root_a1, root_b, "distinct values ⇒ distinct nodes (C1)");
        }
    }
}

/// M2-1, T5: property — for any random sequence over a bounded domain,
/// compiled then decompile gives the sequence back (round-trip), and two
/// equal sequences compiled to the same node (canonicity).
#[test]
fn test_t5_seq_round_trip_property() {
    use lisage_core::arena::Arena;
    use lisage_core::observer::NoOpObserver;
    use lisage_core::schema::{Domain, ReferenceController, Schema, compiled, decompile};
    use lisage_lang::value::{Atom, Value};
    use proptest::prelude::*;

    proptest!(|(domain_size in 1u64..=4,
                 length_max in 1u32..=4,
                 elements in prop::collection::vec(0u64..4, 0..5))| {
        let domain = Domain::new((0..domain_size).map(Atom).collect());
        let schema = Schema::Seq(domain.clone(), length_max);
        let mut pool = Arena::new(ReferenceController::new(schema.clone()), Box::new(NoOpObserver));
        let anchor = lisage_algebra::interface::Anchor(0);
        // Truncate to the max length and filter into the domain.
        let seq: Vec<Value> = elements.iter()
            .take(length_max as usize)
            .filter(|&&v| v < domain_size)
            .map(|&v| Value::Atom(Atom(v)))
            .collect();
        let value = Value::Seq(seq.clone());
        let root = compiled(&mut pool, anchor, &schema, &value).unwrap();
        let result = decompile(&pool, &schema, root).unwrap();
        prop_assert_eq!(result, value.clone(), "round-trip Seq");
        // Canonicity.
        let root2 = compiled(&mut pool, anchor, &schema, &value).unwrap();
        prop_assert_eq!(root, root2, "canonicity Seq");
    });
}
