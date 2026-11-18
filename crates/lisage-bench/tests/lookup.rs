//! Lookup differential (M1-5): path A = reference interpreter
//! (`lisage_lang::interpreter::lookup`), DD path = `lookup` transduction
//! of `lisage-core` on the compiled map. Triptych C1, spec § 6.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{ABSENT, Domain, PRESENT, ReferenceController, Schema, compiler};
use lisage_core::transduction::Transducteur;
use lisage_lang::interpreter::lookup;
use lisage_lang::value::{Atom, Value};
use proptest::prelude::*;

type PoolOutput = Arena<LookupController>;

fn atome(value: u64) -> Value {
    Value::Atom(Atom(value))
}

/// Follows the map's path in the output diagram and returns the
/// payload of the reached terminal.
fn payload_terminal(
    pool: &PoolOutput,
    mut node: NodeId,
    list_keys: &[Atom],
    map: &BTreeMap<Value, Value>,
) -> Option<u64> {
    loop {
        match pool.resolve(node).unwrap() {
            Form::Terminal(payload) => return payload,
            Form::Node { level, arcs } => {
                let index = (level.0 / 2) as usize;
                let key = Value::Atom(list_keys[index]);
                if level.0.is_multiple_of(2) {
                    match map.get(&key) {
                        Some(_) => node = *arcs.get(&PRESENT).expect("arc present"),
                        // Implicit default arc ⇒ D_δ = ε(None); explicit
                        // arc (grafted subtree) ⇒ follow it.
                        None => match arcs.get(&ABSENT) {
                            Some(&suite) => node = suite,
                            None => return None,
                        },
                    }
                } else {
                    let Some(Value::Atom(value)) = map.get(&key) else {
                        panic!("key present with atomic value")
                    };
                    node = *arcs.get(&value.0).expect("arc value");
                }
            }
        }
    }
}

proptest! {
    /// C1 differential: DD lookup (path B) == interpreter lookup (path A)
    /// on a random map with keys {0,1} and values {0,1}.
    #[test]
    fn test_c1_differentiel_lookup_lane_a_vs_dd(
        paires in prop::collection::vec((0u64..2, 0u64..2), 0..5),
        key in 0u64..2,
    ) {
        let list_keys = vec![Atom(0), Atom(1)];
        let domain_values = Domain::new(vec![Atom(0), Atom(1)]);
        let schema = Schema::Map(list_keys.clone(), domain_values.clone());
        let map: BTreeMap<Value, Value> = paires
            .iter()
            .map(|(k, v)| (atome(*k), atome(*v)))
            .collect();
        let value = Value::Map(map.clone());

        let mut input = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut output = Arena::new(
            LookupController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let root = compiler(&mut input, anchor, &schema, &value).unwrap();
        let key_atome = Atom(key);
        let mut runner = Transducteur::new(Lookup::new(
            key_atome,
            list_keys.clone(),
            domain_values.clone(),
        ));
        let (result, _) = runner.run(&input, &mut output, anchor, root).unwrap();
        let dd = payload_terminal(&output, result, &list_keys, &map);

        // Path A: reference interpreter on the set {map}.
        let set = BTreeSet::from([value.clone()]);
        let expected = lookup(&set, &atome(key))[&value].clone();
        let expected_opt = match expected {
            Some(Value::Atom(a)) => Some(a.0),
            _ => None,
        };
        prop_assert_eq!(dd, expected_opt, "DD path != path A for lookup");
    }

    /// C1 differential, collapsed case: value domain of cardinality 1 ⇒
    /// the val(k) level is R-collapsed at compilation, `φ_out`
    /// re-explicits it (T-EXPAND + graft). The DD path must still equal
    /// path A.
    #[test]
    fn test_c1_differentiel_lookup_collapsed_lane_a_vs_dd(
        paires in prop::collection::vec((0u64..2, 0u64..1), 0..5),
        key in 0u64..2,
    ) {
        let list_keys = vec![Atom(0), Atom(1)];
        let domain_values = Domain::new(vec![Atom(0)]);
        let schema = Schema::Map(list_keys.clone(), domain_values.clone());
        let map: BTreeMap<Value, Value> = paires
            .iter()
            .map(|(k, v)| (atome(*k), atome(*v)))
            .collect();
        let value = Value::Map(map.clone());

        let mut input = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let mut output = Arena::new(
            LookupController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let anchor = Anchor(0);
        let root = compiler(&mut input, anchor, &schema, &value).unwrap();
        let key_atome = Atom(key);
        let mut runner = Transducteur::new(Lookup::new(
            key_atome,
            list_keys.clone(),
            domain_values.clone(),
        ));
        let (result, _) = runner.run(&input, &mut output, anchor, root).unwrap();
        let dd = payload_terminal(&output, result, &list_keys, &map);

        let set = BTreeSet::from([value.clone()]);
        let expected = lookup(&set, &atome(key))[&value].clone();
        let expected_opt = match expected {
            Some(Value::Atom(a)) => Some(a.0),
            _ => None,
        };
        prop_assert_eq!(dd, expected_opt, "DD path != path A (collapsed case)");
    }
}
