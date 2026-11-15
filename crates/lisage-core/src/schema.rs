//! Reference schemas (M1-3, spec § 1 and § 2.1): bounded `Atom(D)`,
//! `Set(T)`, `Map(K, V)`, **universe sweep** compilation (one presence level
//! per key, δ = absent, no marker).
//!
//! Levels: `Atom(D)` = one level `Val = D`; `Set(D)` = one presence level
//! per element (`Val = {absent, present}`, δ = absent); `Map(K,V)`
//! = per key `k_i` a presence level `2i` (δ = absent) then a value level
//! `2i+1` (`Val = V`, δ = ⊥). Compilation: only the chosen arc is interned,
//! the other leads to the carrier default D_δ = terminal(false) on presences
//! (R-collapse skips absents, ZDD style).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_lang::value::{Atom, Value};

/// Presence arc key: absent (default δ) or present.
pub const ABSENT: u64 = 0;
/// Presence arc key: present.
pub const PRESENT: u64 = 1;
/// Cardinality of a presence level: {absent, present}.
pub const CARDINALITY_PRESENCE: u64 = 2;
/// Stride between the levels of a map: presence of key `i` at level `2i`,
/// value at level `2i+1`.
pub const PAS_LEVEL_MAP: u32 = 2;

/// Bounded atom domain: finite enumeration, with optional default.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Domain {
    /// Values of the domain, in canonical order.
    pub values: Vec<Atom>,
    /// Default δ of the domain, if declared.
    pub defaut: Option<Atom>,
}

impl Domain {
    /// Domain without default.
    pub fn new(values: Vec<Atom>) -> Self {
        Domain {
            values,
            defaut: None,
        }
    }

    /// Does the domain contain this atom?
    pub fn contains(&self, atome: Atom) -> bool {
        self.values.contains(&atome)
    }

    /// Number of values in the domain.
    pub fn size(&self) -> usize {
        self.values.len()
    }
}

/// Reference schema (bounded type algebra Σ, spec § 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Schema {
    /// `Atom(D)`: bounded domain of atoms.
    Atom(Domain),
    /// `Set(Atom(D))`: set of atoms from a bounded domain.
    Set(Domain),
    /// `Map(K, V)`: ordered keys to a bounded value domain.
    Map(Vec<Atom>, Domain),
    /// `Seq(D, N)`: sequence of at most `N` elements of a bounded domain `D`
    /// (M2-1, `list` constructor). The end marker is a required key.
    Seq(Domain, u32),
    /// `Tag(K, V)`: typed union — discriminant level (keys `K`) then value
    /// level `V` (M2-2). A single useful arc per node (C2).
    Tag(Vec<Atom>, Domain),
}

impl Schema {
    /// Number of levels of the universe sweep.
    pub fn number_levels(&self) -> u32 {
        match self {
            Schema::Atom(_) => 1,
            Schema::Set(domain) => domain.size() as u32,
            Schema::Map(list_keys, _) => PAS_LEVEL_MAP * list_keys.len() as u32,
            Schema::Seq(_, longueur_max) => longueur_max + 1,
            Schema::Tag(list, _) => PAS_LEVEL_MAP * list.len() as u32,
        }
    }

    /// Presence level of the i-th key of a map.
    pub fn level_presence(index: usize) -> Level {
        Level(PAS_LEVEL_MAP * index as u32)
    }

    /// Value level of the i-th key of a map.
    pub fn level_value(index: usize) -> Level {
        Level(PAS_LEVEL_MAP * index as u32 + 1)
    }
}

/// Typed error of schema compilation/decompilation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SchemaError {
    /// The value does not match the schema (different Σ shape).
    TypeMismatch {
        /// Shape expected by the schema.
        expected: &'static str,
        /// Shape received.
        recu: &'static str,
    },
    /// Atom outside the domain declared by the schema.
    HorsDomain(Atom),
    /// Diagram node incompatible with the schema (decompilation).
    NodeInexpected {
        /// Shape expected at this position.
        expected: &'static str,
        /// Node encountered.
        node: NodeId,
    },
    /// Error propagated from the pool port.
    Pool(PoolError),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::TypeMismatch { expected, recu } => {
                write!(formatter, "type mismatch: expected {expected}, got {recu}")
            }
            SchemaError::HorsDomain(atome) => write!(formatter, "atom outside domain: {atome:?}"),
            SchemaError::NodeInexpected { expected, node } => {
                write!(
                    formatter,
                    "unexpected node: expected {expected}, node {node:?}"
                )
            }
            SchemaError::Pool(error) => write!(formatter, "pool error: {error}"),
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SchemaError::Pool(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PoolError> for SchemaError {
    fn from(error: PoolError) -> Self {
        SchemaError::Pool(error)
    }
}

/// Key algebra of a schema: `Val(ℓ)` and δ(ℓ) per level of the sweep.
#[derive(Clone)]
pub struct SchemaKeys {
    schema: Schema,
}

impl SchemaKeys {
    /// Key algebra for the given schema.
    pub fn new(schema: Schema) -> Self {
        SchemaKeys { schema }
    }
}

impl KeyAlgebra for SchemaKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, level: Level) -> u64 {
        match &self.schema {
            Schema::Atom(domain) => domain.size() as u64,
            Schema::Set(_) => CARDINALITY_PRESENCE,
            Schema::Map(_, domain_values) => {
                if level.0.is_multiple_of(PAS_LEVEL_MAP) {
                    CARDINALITY_PRESENCE
                } else {
                    domain_values.size() as u64
                }
            }
            Schema::Seq(domain, longueur_max) => {
                if level.0 < *longueur_max {
                    domain.size() as u64 + 1 // +1 for the end marker
                } else {
                    CARDINALITY_PRESENCE
                }
            }
            Schema::Tag(list, domain_values) => {
                if level.0.is_multiple_of(PAS_LEVEL_MAP) {
                    list.len() as u64
                } else {
                    domain_values.size() as u64
                }
            }
        }
    }

    fn default(&self, level: Level) -> Option<Self::Key> {
        match &self.schema {
            Schema::Atom(domain) => domain.defaut.map(|atome| atome.0),
            Schema::Set(_) => Some(ABSENT),
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(PAS_LEVEL_MAP) {
                    Some(ABSENT)
                } else {
                    None
                }
            }
            Schema::Seq(_, _) => None, // end marker = required key, no default
            Schema::Tag(_, _) => None, // discriminant = required key
        }
    }
}

/// Controller of the reference instance: carries the schema, designates D_δ =
/// terminal(false) (the empty set) for presence levels and atom domains
/// with default; δ = ⊥ (no default) for value levels (spec § 2.1).
pub struct ReferenceController {
    schema: Schema,
    keys: SchemaKeys,
}

impl ReferenceController {
    /// Controller for the given schema.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        ReferenceController { schema, keys }
    }

    /// Schema of this instance.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl Controller for ReferenceController {
    type Key = u64;
    type Payload = bool;
    type Keys = SchemaKeys;

    const FINGERPRINT: ControllerId = ControllerId(1);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        match &self.schema {
            Schema::Atom(domain) => domain.defaut.map(|_| false),
            Schema::Set(_) => Some(false),
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(PAS_LEVEL_MAP) {
                    Some(false)
                } else {
                    None
                }
            }
            Schema::Seq(_, _) => None,
            Schema::Tag(_, _) => None,
        }
    }
}

/// Compiles a value into a diagram (universe sweep); the acceptance terminal
/// has payload `true` (set-theoretic reading).
pub fn compiler<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    schema: &Schema,
    value: &Value,
) -> Result<NodeId, SchemaError> {
    match (schema, value) {
        (Schema::Atom(domain), Value::Atom(atome)) => {
            if !domain.contains(*atome) {
                return Err(SchemaError::HorsDomain(*atome));
            }
            let terminal = pool.intern(anchor, Form::Terminal(true));
            Ok(pool.intern(
                anchor,
                Form::Node {
                    level: Level(0),
                    arcs: BTreeMap::from([(atome.0, terminal)]),
                },
            ))
        }
        (Schema::Set(domain), Value::Set(elements)) => {
            for element in elements {
                match element {
                    Value::Atom(atome) if domain.contains(*atome) => {}
                    Value::Atom(atome) => return Err(SchemaError::HorsDomain(*atome)),
                    other => {
                        return Err(SchemaError::TypeMismatch {
                            expected: "set of atoms",
                            recu: form_value(other),
                        });
                    }
                }
            }
            compiler_set(pool, anchor, domain, elements, 0)
        }
        (Schema::Map(list_keys, domain_values), Value::Map(map)) => {
            valider_map(list_keys, domain_values, map)?;
            compiler_map(pool, anchor, list_keys, map, 0)
        }
        (Schema::Seq(domain, longueur_max), Value::Seq(elements)) => {
            valider_seq(domain, *longueur_max, elements)?;
            compiler_seq(pool, anchor, domain, *longueur_max, elements)
        }
        (Schema::Tag(list, domain_values), Value::Tag(discriminant, payload)) => {
            valider_tag(list, domain_values, *discriminant, payload)?;
            compiler_tag(pool, anchor, list, domain_values, *discriminant, payload)
        }
        (schema, value) => Err(SchemaError::TypeMismatch {
            expected: form_schema(schema),
            recu: form_value(value),
        }),
    }
}

/// Validates the keys and values of a map against the schema (typed errors,
/// the faulty atom is the one reported).
fn valider_map(
    list_keys: &[Atom],
    domain_values: &Domain,
    map: &BTreeMap<Value, Value>,
) -> Result<(), SchemaError> {
    for (key, value) in map {
        let Value::Atom(atome_key) = key else {
            return Err(SchemaError::TypeMismatch {
                expected: "atomic key",
                recu: form_value(key),
            });
        };
        if !list_keys.contains(atome_key) {
            return Err(SchemaError::HorsDomain(*atome_key));
        }
        let Value::Atom(atome_value) = value else {
            return Err(SchemaError::TypeMismatch {
                expected: "atomic value",
                recu: form_value(value),
            });
        };
        if !domain_values.contains(*atome_value) {
            return Err(SchemaError::HorsDomain(*atome_value));
        }
    }
    Ok(())
}

fn form_schema(schema: &Schema) -> &'static str {
    match schema {
        Schema::Atom(_) => "atom",
        Schema::Set(_) => "set",
        Schema::Map(_, _) => "map",
        Schema::Seq(_, _) => "sequence",
        Schema::Tag(_, _) => "typed union",
    }
}

fn form_value(value: &Value) -> &'static str {
    match value {
        Value::Atom(_) => "atom",
        Value::Set(_) => "set",
        Value::Map(_) => "map",
        Value::Seq(_) => "sequence",
        Value::Tag(_, _) => "typed union",
    }
}

fn compiler_set<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    domain: &Domain,
    elements: &BTreeSet<Value>,
    index: usize,
) -> Result<NodeId, SchemaError> {
    if index == domain.size() {
        return Ok(pool.intern(anchor, Form::Terminal(true)));
    }
    let suite = compiler_set(pool, anchor, domain, elements, index + 1)?;
    let present = elements.contains(&Value::Atom(domain.values[index]));
    let arc = if present { PRESENT } else { ABSENT };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(index as u32),
            arcs: BTreeMap::from([(arc, suite)]),
        },
    ))
}

fn compiler_map<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    list_keys: &[Atom],
    map: &BTreeMap<Value, Value>,
    index: usize,
) -> Result<NodeId, SchemaError> {
    if index == list_keys.len() {
        return Ok(pool.intern(anchor, Form::Terminal(true)));
    }
    let suite = compiler_map(pool, anchor, list_keys, map, index + 1)?;
    let key = Value::Atom(list_keys[index]);
    let arcs = match map.get(&key) {
        Some(Value::Atom(value)) => {
            let node_value = pool.intern(
                anchor,
                Form::Node {
                    level: Schema::level_value(index),
                    arcs: BTreeMap::from([(value.0, suite)]),
                },
            );
            BTreeMap::from([(PRESENT, node_value)])
        }
        Some(other) => {
            return Err(SchemaError::TypeMismatch {
                expected: "atomic value",
                recu: form_value(other),
            });
        }
        None => BTreeMap::from([(ABSENT, suite)]),
    };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Schema::level_presence(index),
            arcs,
        },
    ))
}

/// Validates a sequence against the Seq schema (elements in the domain,
/// bounded length).
fn valider_seq(domain: &Domain, longueur_max: u32, elements: &[Value]) -> Result<(), SchemaError> {
    if elements.len() > longueur_max as usize {
        return Err(SchemaError::TypeMismatch {
            expected: "bounded-length sequence",
            recu: "sequence too long",
        });
    }
    for element in elements {
        match element {
            Value::Atom(atome) if domain.contains(*atome) => {}
            Value::Atom(atome) => return Err(SchemaError::HorsDomain(*atome)),
            other => {
                return Err(SchemaError::TypeMismatch {
                    expected: "atomic element",
                    recu: form_value(other),
                });
            }
        }
    }
    Ok(())
}

/// Arc key of the end marker in a sequence (value = domain size).
fn key_fin(domain: &Domain) -> u64 {
    domain.size() as u64
}

/// Compiles a sequence by **positions** (universe sweep, M2-1):
/// each position `i` has a level of cardinality `|D|+1` (domain
/// values + end marker). The end marker leads to terminal true.
fn compiler_seq<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    domain: &Domain,
    longueur_max: u32,
    elements: &[Value],
) -> Result<NodeId, SchemaError> {
    compiler_seq_position(pool, anchor, domain, longueur_max, elements, 0)
}

fn compiler_seq_position<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    domain: &Domain,
    longueur_max: u32,
    elements: &[Value],
    position: u32,
) -> Result<NodeId, SchemaError> {
    if position >= longueur_max {
        // End-marker level: required key, present = end of sequence.
        let terminal = pool.intern(anchor, Form::Terminal(true));
        return Ok(pool.intern(
            anchor,
            Form::Node {
                level: Level(longueur_max),
                arcs: BTreeMap::from([(PRESENT, terminal)]),
            },
        ));
    }
    let suite = compiler_seq_position(pool, anchor, domain, longueur_max, elements, position + 1)?;
    let arc = if (position as usize) < elements.len() {
        match &elements[position as usize] {
            Value::Atom(atome) => atome.0,
            _ => unreachable!("validated by valider_seq"),
        }
    } else {
        key_fin(domain) // end marker
    };
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(position),
            arcs: BTreeMap::from([(arc, suite)]),
        },
    ))
}

/// Validates a tag against the schema (discriminant in the list, payload in the domain).
fn valider_tag(
    list: &[Atom],
    domain_values: &Domain,
    discriminant: Atom,
    payload: &Value,
) -> Result<(), SchemaError> {
    if !list.contains(&discriminant) {
        return Err(SchemaError::HorsDomain(discriminant));
    }
    match payload {
        Value::Atom(atome) if domain_values.contains(*atome) => Ok(()),
        Value::Atom(atome) => Err(SchemaError::HorsDomain(*atome)),
        other => Err(SchemaError::TypeMismatch {
            expected: "atomic payload",
            recu: form_value(other),
        }),
    }
}

/// Compiles a tag: discriminant level (one arc = one variant) then value
/// level. A single useful arc per node (C2).
fn compiler_tag<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    list: &[Atom],
    domain_values: &Domain,
    discriminant: Atom,
    payload: &Value,
) -> Result<NodeId, SchemaError> {
    let terminal = pool.intern(anchor, Form::Terminal(true));
    let Value::Atom(atome_value) = payload else {
        return Err(SchemaError::TypeMismatch {
            expected: "atomic payload",
            recu: form_value(payload),
        });
    };
    let node_value = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(atome_value.0, terminal)]),
        },
    );
    let index = list.iter().position(|a| *a == discriminant).unwrap();
    let _ = domain_values;
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(index as u64, node_value)]),
        },
    ))
}

/// Decompiles a diagram into a value (inverse of [`compiler`]): useful for
/// round-trip tests and diagnostics.
pub fn decompiler<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    schema: &Schema,
    node: NodeId,
) -> Result<Value, SchemaError> {
    match schema {
        Schema::Atom(domain) => {
            // Single-element domain: R-collapse reduces the atomic node
            // to the acceptance terminal.
            if matches!(pool.resolve(node), Ok(Form::Terminal(true))) {
                return match domain.values.first() {
                    Some(atome) if domain.size() == 1 => Ok(Value::Atom(*atome)),
                    _ => Err(SchemaError::NodeInexpected {
                        expected: "atomic-level node",
                        node,
                    }),
                };
            }
            let Form::Node { level, arcs } = pool.resolve(node)? else {
                return Err(SchemaError::NodeInexpected {
                    expected: "atomic-level node",
                    node,
                });
            };
            if *level != Level(0) {
                return Err(SchemaError::NodeInexpected {
                    expected: "level 0",
                    node,
                });
            }
            for (key, cible) in arcs {
                if matches!(pool.resolve(*cible), Ok(Form::Terminal(true))) {
                    let atome = Atom(*key);
                    if domain.contains(atome) {
                        return Ok(Value::Atom(atome));
                    }
                    return Err(SchemaError::HorsDomain(atome));
                }
            }
            Err(SchemaError::NodeInexpected {
                expected: "arc to terminal true",
                node,
            })
        }
        Schema::Set(domain) => {
            let mut elements = BTreeSet::new();
            decompiler_set(pool, domain, node, &mut elements)?;
            Ok(Value::Set(elements))
        }
        Schema::Map(list_keys, _) => {
            let mut map = BTreeMap::new();
            decompiler_map(pool, list_keys, node, &mut map)?;
            Ok(Value::Map(map))
        }
        Schema::Seq(domain, longueur_max) => {
            let mut elements = Vec::new();
            decompiler_seq(pool, domain, *longueur_max, node, &mut elements)?;
            Ok(Value::Seq(elements))
        }
        Schema::Tag(list, domain_values) => decompiler_tag(pool, list, domain_values, node),
    }
}

fn decompiler_tag<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    list: &[Atom],
    domain_values: &Domain,
    node: NodeId,
) -> Result<Value, SchemaError> {
    let Form::Node { level, arcs } = pool.resolve(node)? else {
        return Err(SchemaError::NodeInexpected {
            expected: "discriminant node",
            node,
        });
    };
    if *level != Level(0) {
        return Err(SchemaError::NodeInexpected {
            expected: "discriminant level (0)",
            node,
        });
    }
    let Some((&index, &node_value)) = arcs.iter().next() else {
        return Err(SchemaError::NodeInexpected {
            expected: "discriminant arc",
            node,
        });
    };
    let discriminant = *list
        .get(index as usize)
        .ok_or(SchemaError::NodeInexpected {
            expected: "valid discriminant index",
            node,
        })?;
    let Form::Node {
        level: level_value,
        arcs: arcs_value,
    } = pool.resolve(node_value)?
    else {
        return Err(SchemaError::NodeInexpected {
            expected: "value node",
            node: node_value,
        });
    };
    if *level_value != Level(1) {
        return Err(SchemaError::NodeInexpected {
            expected: "value level (1)",
            node: node_value,
        });
    }
    let Some((&value, _)) = arcs_value.iter().next() else {
        return Err(SchemaError::NodeInexpected {
            expected: "value arc",
            node: node_value,
        });
    };
    let atome = Atom(value);
    if !domain_values.contains(atome) {
        return Err(SchemaError::HorsDomain(atome));
    }
    Ok(Value::Tag(discriminant, Box::new(Value::Atom(atome))))
}

fn decompiler_seq<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    domain: &Domain,
    longueur_max: u32,
    mut node: NodeId,
    elements: &mut Vec<Value>,
) -> Result<(), SchemaError> {
    loop {
        match pool.resolve(node)? {
            Form::Terminal(true) => return Ok(()),
            Form::Terminal(false) => {
                return Err(SchemaError::NodeInexpected {
                    expected: "terminal true or position node",
                    node,
                });
            }
            Form::Node { level, arcs } => {
                if level.0 > longueur_max {
                    return Err(SchemaError::NodeInexpected {
                        expected: "position level or end marker",
                        node,
                    });
                }
                if level.0 == longueur_max {
                    // End-marker level.
                    if arcs.contains_key(&PRESENT) {
                        return Ok(());
                    }
                    return Err(SchemaError::NodeInexpected {
                        expected: "end marker present",
                        node,
                    });
                }
                let Some((&arc, &suite)) = arcs.iter().next() else {
                    return Err(SchemaError::NodeInexpected {
                        expected: "position arc",
                        node,
                    });
                };
                if arc == key_fin(domain) {
                    // Early end marker: the sequence stops here.
                    node = suite;
                    continue;
                }
                let atome = Atom(arc);
                if !domain.contains(atome) {
                    return Err(SchemaError::HorsDomain(atome));
                }
                elements.push(Value::Atom(atome));
                node = suite;
            }
        }
    }
}

fn decompiler_set<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    domain: &Domain,
    mut node: NodeId,
    elements: &mut BTreeSet<Value>,
) -> Result<(), SchemaError> {
    loop {
        match pool.resolve(node)? {
            Form::Terminal(true) => return Ok(()),
            Form::Terminal(false) => {
                return Err(SchemaError::NodeInexpected {
                    expected: "terminal true (empty set) or presence node",
                    node,
                });
            }
            Form::Node { level, arcs } => {
                let index = level.0 as usize;
                if index >= domain.size() {
                    return Err(SchemaError::NodeInexpected {
                        expected: "domain presence level",
                        node,
                    });
                }
                if let Some(&suite) = arcs.get(&PRESENT) {
                    elements.insert(Value::Atom(domain.values[index]));
                    node = suite;
                } else if let Some(&suite) = arcs.get(&ABSENT) {
                    node = suite;
                } else {
                    return Err(SchemaError::NodeInexpected {
                        expected: "presence arc",
                        node,
                    });
                }
            }
        }
    }
}

fn decompiler_map<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    list_keys: &[Atom],
    mut node: NodeId,
    map: &mut BTreeMap<Value, Value>,
) -> Result<(), SchemaError> {
    loop {
        match pool.resolve(node)? {
            Form::Terminal(true) => return Ok(()),
            Form::Terminal(false) => {
                return Err(SchemaError::NodeInexpected {
                    expected: "terminal true (empty map) or presence node",
                    node,
                });
            }
            Form::Node { level, arcs } => {
                if !level.0.is_multiple_of(PAS_LEVEL_MAP) {
                    return Err(SchemaError::NodeInexpected {
                        expected: "presence level",
                        node,
                    });
                }
                let index = (level.0 / PAS_LEVEL_MAP) as usize;
                if index >= list_keys.len() {
                    return Err(SchemaError::NodeInexpected {
                        expected: "presence level of a key",
                        node,
                    });
                }
                if let Some(&node_value) = arcs.get(&PRESENT) {
                    let Form::Node {
                        level: level_value,
                        arcs: arcs_value,
                    } = pool.resolve(node_value)?
                    else {
                        return Err(SchemaError::NodeInexpected {
                            expected: "value-level node",
                            node: node_value,
                        });
                    };
                    if *level_value != Schema::level_value(index) {
                        return Err(SchemaError::NodeInexpected {
                            expected: "value level of the key",
                            node: node_value,
                        });
                    }
                    let Some((&value, &suite)) = arcs_value.iter().next() else {
                        return Err(SchemaError::NodeInexpected {
                            expected: "value arc",
                            node: node_value,
                        });
                    };
                    map.insert(Value::Atom(list_keys[index]), Value::Atom(Atom(value)));
                    node = suite;
                } else if let Some(&suite) = arcs.get(&ABSENT) {
                    node = suite;
                } else {
                    return Err(SchemaError::NodeInexpected {
                        expected: "presence arc",
                        node,
                    });
                }
            }
        }
    }
}
