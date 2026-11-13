//! `lookup(k)` tests (M1-5, spec § 6): seek/extract/stop modes,
//! synthesized DD fragment, T-EXPAND + grafting when the value level of
//! the key has been collapsed.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{ABSENT, Domain, PRESENT, ReferenceController, Schema, compiled};
use lisage_core::transduction::Transducer;
use lisage_lang::value::{Atom, Value};

type PoolOutput = Arena<LookupController>;

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

/// Runs `lookup(key)` on the compiled map and returns (result, fragment).
fn execute_lookup(
    schema: &Schema,
    list_keys: &[Atom],
    domain_values: &Domain,
    value: &Value,
    key: Atom,
) -> (PoolOutput, NodeId, BTreeMap<u64, NodeId>) {
    let mut input = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut output = Arena::new(
        LookupController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let anchor = Anchor(0);
    let root = compiled(&mut input, anchor, schema, value).unwrap();
    let mut runner = Transducer::new(Lookup::new(key, list_keys.to_vec(), domain_values.clone()));
    let (result, fragment) = runner.run(&input, &mut output, anchor, root).unwrap();
    (output, result, fragment)
}

/// Follows the path of the map in the output diagram and returns the
/// payload of the reached terminal (None if the followed key is absent from the
/// map: the default edge leads to D_δ = ε(None)).
fn payload_terminal(
    pool: &PoolOutput,
    mut node: NodeId,
    list_keys: &[Atom],
    map: &BTreeMap<Value, Value>,
) -> Option<u64> {
    loop {
        match pool.resolve(node).unwrap() {
            Form::Terminal(payload) => return *payload,
            Form::Node { level, edges } => {
                let index = (level.0 / 2) as usize;
                let key = Value::Atom(list_keys[index]);
                if level.0.is_multiple_of(2) {
                    match map.get(&key) {
                        Some(_) => node = *edges.get(&PRESENT).expect("present edge"),
                        // Implicit default edge ⇒ D_δ = ε(None); explicit
                        // edge (grafted subtree) ⇒ we follow it.
                        None => match edges.get(&ABSENT) {
                            Some(&suite) => node = suite,
                            None => return None,
                        },
                    }
                } else {
                    let Some(Value::Atom(value)) = map.get(&key) else {
                        panic!("present key with atomic value")
                    };
                    node = *edges.get(&value.0).expect("value edge");
                }
            }
        }
    }
}

/// Map with two keys over V = {0,1}: lookup retrieves the value of both keys.
#[test]
fn test_lookup_keys_present() {
    let list_keys = vec![Atom(0), Atom(1)];
    let domain_values = Domain::new(vec![Atom(0), Atom(1)]);
    let schema = Schema::Map(list_keys.clone(), domain_values.clone());
    let value = map_of(&[(0, 1), (1, 0)]);
    let Value::Map(ref map) = value else {
        panic!("map")
    };

    let (output, result, fragment) =
        execute_lookup(&schema, &list_keys, &domain_values, &value, Atom(0));
    assert_eq!(
        payload_terminal(&output, result, &list_keys, map),
        Some(1),
        "lookup(a) = 1"
    );
    assert!(
        fragment.contains_key(&1),
        "the fragment carries the value edge 1 to ε(Some(1))"
    );

    let (output, result, _) = execute_lookup(&schema, &list_keys, &domain_values, &value, Atom(1));
    assert_eq!(
        payload_terminal(&output, result, &list_keys, map),
        Some(0),
        "lookup(b) = 0"
    );
}

/// Key absent from the map: the terminal is None (D_δ of the output pool).
#[test]
fn test_lookup_key_absent() {
    let list_keys = vec![Atom(0), Atom(1)];
    let domain_values = Domain::new(vec![Atom(0), Atom(1)]);
    let schema = Schema::Map(list_keys.clone(), domain_values.clone());
    let value = map_of(&[(0, 1)]);
    let Value::Map(ref map) = value else {
        panic!("map")
    };

    let (output, result, fragment) =
        execute_lookup(&schema, &list_keys, &domain_values, &value, Atom(1));
    assert_eq!(
        payload_terminal(&output, result, &list_keys, map),
        None,
        "lookup(b) = None, absent key"
    );
    assert!(fragment.is_empty(), "no fragment without Stop");
}

/// Value domain of cardinality 1: the level val(k) is collapsed at
/// compilation; φ_out re-explicits it (T-EXPAND) and grafts Some(v).
#[test]
fn test_lookup_level_value_collapsed_t_expand() {
    let list_keys = vec![Atom(0)];
    let domain_values = Domain::new(vec![Atom(0)]);
    let schema = Schema::Map(list_keys.clone(), domain_values.clone());
    let value = map_of(&[(0, 0)]);
    let Value::Map(ref map) = value else {
        panic!("map")
    };

    let (output, result, fragment) =
        execute_lookup(&schema, &list_keys, &domain_values, &value, Atom(0));
    assert_eq!(
        payload_terminal(&output, result, &list_keys, map),
        Some(0),
        "T-EXPAND: lookup(a) = 0 despite the collapsed value level"
    );
    assert!(
        fragment.is_empty(),
        "no Stop in the collapsed case: the value lives in the DD (T-EXPAND)"
    );
}
