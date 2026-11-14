//! Tests for `lookup(k)` (M1-5, spec § 6): search/extract/stop modes,
//! synthesized DD fragment, T-EXPAND + graft when the key's value level was
//! collapsed.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::lookup::{Lookup, LookupController};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{ABSENT, Domain, PRESENT, ReferenceController, Schema, compiler};
use lisage_core::transduction::Transducteur;
use lisage_lang::value::{Atom, Value};

type PoolOutput = Arena<LookupController>;

fn atome(value: u64) -> Value {
    Value::Atom(Atom(value))
}

fn map_de(paires: &[(u64, u64)]) -> Value {
    Value::Map(
        paires
            .iter()
            .map(|(key, value)| (atome(*key), atome(*value)))
            .collect::<BTreeMap<Value, Value>>(),
    )
}

/// Runs `lookup(key)` on the compiled map and returns (result, fragment).
fn executer_lookup(
    schema: &Schema,
    liste_keys: &[Atom],
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
    let root = compiler(&mut input, anchor, schema, value).unwrap();
    let mut runner =
        Transducteur::new(Lookup::new(key, liste_keys.to_vec(), domain_values.clone()));
    let (resultat, fragment) = runner.run(&input, &mut output, anchor, root).unwrap();
    (output, resultat, fragment)
}

/// Follows the map's path in the output diagram and returns the payload of
/// the reached terminal (None if the followed key is absent from the map:
/// the default arc leads to D_δ = ε(None)).
fn payload_terminal(
    pool: &PoolOutput,
    mut node: NodeId,
    liste_keys: &[Atom],
    map: &BTreeMap<Value, Value>,
) -> Option<u64> {
    loop {
        match pool.resolve(node).unwrap() {
            Form::Terminal(payload) => return *payload,
            Form::Node { level, arcs } => {
                let index = (level.0 / 2) as usize;
                let key = Value::Atom(liste_keys[index]);
                if level.0.is_multiple_of(2) {
                    match map.get(&key) {
                        Some(_) => node = *arcs.get(&PRESENT).expect("present arc"),
                        // Implicit default arc ⇒ D_δ = ε(None); explicit arc
                        // (grafted subtree) ⇒ follow it.
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

/// Two-key map over V = {0,1}: lookup finds the value of both keys.
#[test]
fn test_lookup_keys_presentes() {
    let liste_keys = vec![Atom(0), Atom(1)];
    let domain_values = Domain::new(vec![Atom(0), Atom(1)]);
    let schema = Schema::Map(liste_keys.clone(), domain_values.clone());
    let value = map_de(&[(0, 1), (1, 0)]);
    let Value::Map(ref map) = value else {
        panic!("map")
    };

    let (output, resultat, fragment) =
        executer_lookup(&schema, &liste_keys, &domain_values, &value, Atom(0));
    assert_eq!(
        payload_terminal(&output, resultat, &liste_keys, map),
        Some(1),
        "lookup(a) = 1"
    );
    assert!(
        fragment.contains_key(&1),
        "the fragment carries the value arc 1 to ε(Some(1))"
    );

    let (output, resultat, _) =
        executer_lookup(&schema, &liste_keys, &domain_values, &value, Atom(1));
    assert_eq!(
        payload_terminal(&output, resultat, &liste_keys, map),
        Some(0),
        "lookup(b) = 0"
    );
}

/// Key absent from the map: the terminal is None (D_δ of the output pool).
#[test]
fn test_lookup_key_absente() {
    let liste_keys = vec![Atom(0), Atom(1)];
    let domain_values = Domain::new(vec![Atom(0), Atom(1)]);
    let schema = Schema::Map(liste_keys.clone(), domain_values.clone());
    let value = map_de(&[(0, 1)]);
    let Value::Map(ref map) = value else {
        panic!("map")
    };

    let (output, resultat, fragment) =
        executer_lookup(&schema, &liste_keys, &domain_values, &value, Atom(1));
    assert_eq!(
        payload_terminal(&output, resultat, &liste_keys, map),
        None,
        "lookup(b) = None, absent key"
    );
    assert!(fragment.is_empty(), "no fragment without Stop");
}

/// Value domain of cardinality 1: the val(k) level is collapsed at compile
/// time; φ_out re-explicits it (T-EXPAND) and grafts Some(v).
#[test]
fn test_lookup_niveau_value_collapsed_t_expand() {
    let liste_keys = vec![Atom(0)];
    let domain_values = Domain::new(vec![Atom(0)]);
    let schema = Schema::Map(liste_keys.clone(), domain_values.clone());
    let value = map_de(&[(0, 0)]);
    let Value::Map(ref map) = value else {
        panic!("map")
    };

    let (output, resultat, fragment) =
        executer_lookup(&schema, &liste_keys, &domain_values, &value, Atom(0));
    assert_eq!(
        payload_terminal(&output, resultat, &liste_keys, map),
        Some(0),
        "T-EXPAND: lookup(a) = 0 despite the collapsed value level"
    );
    assert!(
        fragment.is_empty(),
        "no Stop in the collapsed case: the value lives in the DD (T-EXPAND)"
    );
}
