//! ADD/MTBDD (M5-4, statement C7, open question n° 8 of spec § 10):
//! **lifted construction** — non-idempotent merge with weights on the
//! terminals, unique table indexed by the weights (the handle includes the
//! weight, so two terminals of different weights are distinct handles), ⊕
//! pointwise like ADD. Compatibility guard (A5): a weighted fold is exact
//! only on this lifted construction; on a Bool construction (idempotent
//! `union`), only the cardinality is exact — multiplicity is destroyed at
//! write time.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};

use crate::error::CoreError;
use crate::schema::{ABSENT, PAS_LEVEL_MAP, PRESENT, Schema, SchemaError, SchemaKeys};
use lisage_lang::value::{Atom, Value};

/// Fingerprint of the ADD controller (distinct from `ReferenceController`).
const FINGERPRINT_ADD: ControllerId = ControllerId(2);

/// Controller of the lifted ADD construction: payload = weight `u64` on the
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
    type Arcs = ArcRun<Self::Key>;

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

/// Compiles a `Value::Map` into a weighted diagram of the given weight
/// (universe sweep, mirror of `schema::compiler` for payload `u64`).
pub fn compiler_pondere<P: Pool<Key = u64, Payload = u64>>(
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
        let (Value::Atom(atome_key), Value::Atom(atome_value)) = (key, value) else {
            return Err(SchemaError::TypeMismatch {
                expected: "atomic key and value",
                recu: "non-atomic",
            });
        };
        if !list_keys.contains(atome_key) {
            return Err(SchemaError::HorsDomain(*atome_key));
        }
        if !domain.contains(*atome_value) {
            return Err(SchemaError::HorsDomain(*atome_value));
        }
    }
    Ok(compiler_map_pondere(
        pool, anchor, list_keys, map, 0, weight,
    ))
}

/// Recursive compilation of a map into a weighted presence/value chain.
fn compiler_map_pondere<P: Pool<Key = u64, Payload = u64>>(
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
    let suite = compiler_map_pondere(pool, anchor, list_keys, map, index + 1, weight);
    let key = Value::Atom(list_keys[index]);
    let arcs = match map.get(&key) {
        Some(Value::Atom(value)) => {
            let node_value = pool.intern(
                anchor,
                Form::Node {
                    level: Schema::level_value(index),
                    arcs: ArcRun::from_pairs([(value.0, suite)]),
                },
            );
            ArcRun::from_pairs([(PRESENT, node_value)])
        }
        _ => ArcRun::from_pairs([(ABSENT, suite)]),
    };
    pool.intern(
        anchor,
        Form::Node {
            level: Schema::level_presence(index),
            arcs,
        },
    )
}

/// ADD construction merge: ⊕ pointwise on the terminal weights,
/// **non-idempotent** (`merge_add(a, a)` doubles the weights — the unique
/// table is indexed by the weights, duplicate collapse is lost).
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
        (Form::Node { level, arcs }, Form::Terminal(weight_b)) => {
            if weight_b == 0 {
                Ok(a)
            } else {
                merge_with_other(pool, keys, anchor, level, arcs, b)
            }
        }
        (Form::Terminal(weight_a), Form::Node { level, arcs }) => {
            if weight_a == 0 {
                Ok(b)
            } else {
                merge_with_other(pool, keys, anchor, level, arcs, a)
            }
        }
        (
            Form::Node {
                level: level_a,
                arcs: arcs_a,
            },
            Form::Node {
                level: level_b,
                arcs: arcs_b,
            },
        ) => {
            if level_a == level_b {
                let mut acc: BTreeMap<u64, NodeId> = arcs_a.into_iter().collect();
                for (arc, target_b) in arcs_b {
                    match acc.remove(&arc) {
                        Some(target_a) => {
                            let merge = merge_add(pool, keys, anchor, target_a, target_b)?;
                            acc.insert(arc, merge);
                        }
                        None => {
                            acc.insert(arc, target_b);
                        }
                    }
                }
                Ok(pool.intern(
                    anchor,
                    Form::Node {
                        level: level_a,
                        arcs: acc.into(),
                    },
                ))
            } else if level_a < level_b {
                merge_with_other(pool, keys, anchor, level_a, arcs_a, b)
            } else {
                merge_with_other(pool, keys, anchor, level_b, arcs_b, a)
            }
        }
    }
}

/// Merge of a node of level `level` with another diagram `other` (terminal
/// of weight > 0, or a deeper node): the path of `other` goes through the
/// default arc `δ(level)` if the level has one, otherwise through all the
/// values of the level.
fn merge_with_other<P: Pool<Key = u64, Payload = u64>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    level: Level,
    arcs: ArcRun<u64>,
    other: NodeId,
) -> Result<NodeId, CoreError> {
    let mut acc: BTreeMap<u64, NodeId> = arcs.into_iter().collect();
    match keys.default(level) {
        Some(defaut) => {
            let target = match acc.remove(&defaut) {
                Some(target) => merge_add(pool, keys, anchor, target, other)?,
                None => other,
            };
            acc.insert(defaut, target);
        }
        None => {
            let cardinality = keys.cardinality(level);
            for v in 0..cardinality {
                let target = match acc.remove(&v) {
                    Some(target) => merge_add(pool, keys, anchor, target, other)?,
                    None => other,
                };
                acc.insert(v, target);
            }
        }
    }
    Ok(pool.intern(
        anchor,
        Form::Node {
            level,
            arcs: acc.into(),
        },
    ))
}

/// Weighted fold: sum of the terminal weights over all accepting paths
/// (A4 counting expansion like `Pli`). Exact **only** on a lifted ADD
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

/// Recursive weighted sum with memo on nodes.
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
        Form::Node { level, arcs } => {
            let mut total = 0u64;
            for (arc, target) in &arcs {
                let level_target = match pool.resolve(*target)?.clone() {
                    Form::Node { level, .. } => Some(level),
                    Form::Terminal(_) => None,
                };
                let multiplicateur = multiplicateur(keys, number_levels, level, *arc, level_target);
                total += multiplicateur * sum(pool, keys, number_levels, *target, memo)?;
            }
            // Implicit default arc: D_δ = terminal(0), of zero weight.
            total
        }
    };
    memo.insert(node, total);
    Ok(total)
}

/// (A4) multiplier of arc `arc` from level `level` to a child of level
/// `level_target` (`None` = terminal): product of the cardinalities of the
/// value levels skipped on the path (mirror of `Pli::multiplicateur`).
fn multiplicateur(
    keys: &SchemaKeys,
    number_levels: u32,
    level: Level,
    arc: u64,
    level_target: Option<Level>,
) -> u64 {
    let bound = match level_target {
        Some(level) => level.0,
        None => number_levels,
    };
    let mut n = match keys.default(level) {
        Some(defaut) if arc == defaut => level.0 + PAS_LEVEL_MAP,
        _ => level.0 + 1,
    };
    let mut produit = 1u64;
    while n < bound {
        if keys.default(Level(n)).is_some() {
            n += PAS_LEVEL_MAP;
        } else {
            produit *= keys.cardinality(Level(n));
            n += 1;
        }
    }
    produit
}
