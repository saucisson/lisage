//! ADD/MTBDD (M5-4, statement C7, open question no. 8 of spec § 10):
//! **lifted construction** — not-idempotent weighted merge on terminals,
//! unique table indexed by weights (the handle includes the weight, so two
//! terminals with different weights are distinct handles), ⊕ pointwise
//! like the ADD. Compatibility guard (A5): a weighted fold is exact
//! only on this lifted construction; on a Bool construction
//! (`union` idempotent), only the cardinality is exact — multiplicity is
//! destroyed at write time.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};

use crate::error::CoreError;
use crate::schema::{ABSENT, PAS_LEVEL_MAP, PRESENT, Schema, SchemaError, SchemaKeys};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the ADD controller (distinct from `ReferenceController`).
const FINGERPRINT_ADD: ControllerId = ControllerId(2);

/// Controller of the lifted ADD construction: payload = `u64` weight on
/// terminals, `D_δ = terminal(0)` (zero weight) at presence levels.
pub struct AddController {
    schema: Schema,
    keys: SchemaKeys,
}

impl AddController {
    /// ADD controller for the given schema.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        AddController { schema, keys }
    }

    /// Schema of this instance.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl Controller for AddController {
    type Key = u64;
    type Payload = u64;
    type Keys = SchemaKeys;

    const FINGERPRINT: ControllerId = FINGERPRINT_ADD;

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        match &self.schema {
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(PAS_LEVEL_MAP) {
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Compiles a `Value::Map` into a weighted diagram with the given weight
/// (universe sweep, mirror of `schema::compiled` for `u64` payload).
pub fn compile_weighted<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    anchor: Anchor,
    schema: &Schema,
    value: &Value,
    weight: u64,
) -> Result<NodeId, SchemaError> {
    let Schema::Map(list_keys, domain) = schema else {
        return Err(SchemaError::TypeMismatch {
            expected: "map",
            recu: "other schema",
        });
    };
    let Value::Map(map) = value else {
        return Err(SchemaError::TypeMismatch {
            expected: "map",
            recu: "other value",
        });
    };
    for (key, value) in map {
        let (Value::Atom(atom_key), Value::Atom(atom_value)) = (key, value) else {
            return Err(SchemaError::TypeMismatch {
                expected: "atomic key and value",
                recu: "not-atomic",
            });
        };
        if !list_keys.contains(atom_key) {
            return Err(SchemaError::OutDomain(*atom_key));
        }
        if !domain.contains(*atom_value) {
            return Err(SchemaError::OutDomain(*atom_value));
        }
    }
    Ok(compile_map_weighted(
        pool, anchor, list_keys, map, 0, weight,
    ))
}

/// Recursive compilation of a map into a weighted presence/value chain.
fn compile_map_weighted<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    anchor: Anchor,
    list_keys: &[Atom],
    map: &BTreeMap<Value, Value>,
    index: usize,
    weight: u64,
) -> NodeId {
    if index == list_keys.len() {
        return pool.intern(anchor, Form::Terminal(weight));
    }
    let suite = compile_map_weighted(pool, anchor, list_keys, map, index + 1, weight);
    let key = Value::Atom(list_keys[index]);
    let edges = match map.get(&key) {
        Some(Value::Atom(value)) => {
            let node_value = pool.intern(
                anchor,
                Form::Node {
                    level: Schema::level_value(index),
                    edges: BTreeMap::from([(value.0, suite)]),
                },
            );
            BTreeMap::from([(PRESENT, node_value)])
        }
        _ => BTreeMap::from([(ABSENT, suite)]),
    };
    pool.intern(
        anchor,
        Form::Node {
            level: Schema::level_presence(index),
            edges,
        },
    )
}

/// ADD construction merge: ⊕ pointwise on terminal weights,
/// **not-idempotent** (`merge_add(a, a)` doubles the weights — the unique
/// table is indexed by weights, duplicate collapsing is lost).
pub fn merge_add<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    a: NodeId,
    b: NodeId,
) -> Result<NodeId, CoreError> {
    let (form_a, form_b) = (pool.resolve(a)?.clone(), pool.resolve(b)?.clone());
    match (form_a, form_b) {
        (Form::Terminal(weight_a), Form::Terminal(weight_b)) => {
            Ok(pool.intern(anchor, Form::Terminal(weight_a + weight_b)))
        }
        (Form::Node { level, edges }, Form::Terminal(weight_b)) => {
            if weight_b == 0 {
                Ok(a)
            } else {
                merge_with_other(pool, keys, anchor, level, edges, b)
            }
        }
        (Form::Terminal(weight_a), Form::Node { level, edges }) => {
            if weight_a == 0 {
                Ok(b)
            } else {
                merge_with_other(pool, keys, anchor, level, edges, a)
            }
        }
        (
            Form::Node {
                level: level_a,
                edges: edges_a,
            },
            Form::Node {
                level: level_b,
                edges: edges_b,
            },
        ) => {
            if level_a == level_b {
                let mut edges = edges_a;
                for (edge, target_b) in edges_b {
                    match edges.remove(&edge) {
                        Some(target_a) => {
                            let fusion = merge_add(pool, keys, anchor, target_a, target_b)?;
                            edges.insert(edge, fusion);
                        }
                        None => {
                            edges.insert(edge, target_b);
                        }
                    }
                }
                Ok(pool.intern(
                    anchor,
                    Form::Node {
                        level: level_a,
                        edges,
                    },
                ))
            } else if level_a < level_b {
                merge_with_other(pool, keys, anchor, level_a, edges_a, b)
            } else {
                merge_with_other(pool, keys, anchor, level_b, edges_b, a)
            }
        }
    }
}

/// Merge of a node at level `level` with another diagram `other`
/// (terminal with weight > 0, or deeper node): the path of `other`
/// goes through the default edge `δ(level)` if the level has one, otherwise
/// through all values of the level.
fn merge_with_other<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    level: Level,
    mut edges: BTreeMap<u64, NodeId>,
    other: NodeId,
) -> Result<NodeId, CoreError> {
    match keys.default(level) {
        Some(default) => {
            let target = match edges.remove(&default) {
                Some(target) => merge_add(pool, keys, anchor, target, other)?,
                None => other,
            };
            edges.insert(default, target);
        }
        None => {
            let cardinality = keys.cardinality(level);
            for v in 0..cardinality {
                let target = match edges.remove(&v) {
                    Some(target) => merge_add(pool, keys, anchor, target, other)?,
                    None => other,
                };
                edges.insert(v, target);
            }
        }
    }
    Ok(pool.intern(anchor, Form::Node { level, edges }))
}

/// Weighted fold: sum of terminal weights over all accepting paths
/// (expansion-counting A4 like `Fold`). Exact **only** on a lifted ADD
/// construction — this is the compatibility guard (Q8).
pub fn sum_weight<P: Pool<Key = u64, Payload = u64>>(
    pool: &P,
    schema: &Schema,
    node: NodeId,
) -> Result<u64, CoreError> {
    let keys = SchemaKeys::new(schema.clone());
    let number_levels = schema.number_levels();
    let mut memo = HashMap::new();
    sum(pool, &keys, number_levels, node, &mut memo)
}

/// Recursive weighted sum with memoization on nodes.
fn sum<P: Pool<Key = u64, Payload = u64>>(
    pool: &P,
    keys: &SchemaKeys,
    number_levels: u32,
    node: NodeId,
    memo: &mut HashMap<NodeId, u64>,
) -> Result<u64, CoreError> {
    if let Some(total) = memo.get(&node) {
        return Ok(*total);
    }
    let form = pool.resolve(node)?.clone();
    let total = match form {
        Form::Terminal(weight) => weight,
        Form::Node { level, edges } => {
            let mut total = 0u64;
            for (edge, target) in &edges {
                let level_target = match pool.resolve(*target)?.clone() {
                    Form::Node { level, .. } => Some(level),
                    Form::Terminal(_) => None,
                };
                let multiplicateur =
                    multiplicateur(keys, number_levels, level, *edge, level_target);
                total += multiplicateur * sum(pool, keys, number_levels, *target, memo)?;
            }
            // Implicit default edge: D_δ = terminal(0), zero weight.
            total
        }
    };
    memo.insert(node, total);
    Ok(total)
}

/// (A4) multiplier of edge `edge` from level `level` to a child at level
/// `level_target` (`None` = terminal): product of the cardinalities of the
/// value levels skipped along the path (mirror of `Fold::multiplicateur`).
fn multiplicateur(
    keys: &SchemaKeys,
    number_levels: u32,
    level: Level,
    edge: u64,
    level_target: Option<Level>,
) -> u64 {
    let bound = match level_target {
        Some(level) => level.0,
        None => number_levels,
    };
    let mut n = match keys.default(level) {
        Some(default) if edge == default => level.0 + PAS_LEVEL_MAP,
        _ => level.0 + 1,
    };
    let mut product = 1u64;
    while n < bound {
        if keys.default(Level(n)).is_some() {
            n += PAS_LEVEL_MAP;
        } else {
            product *= keys.cardinality(Level(n));
            n += 1;
        }
    }
    product
}
