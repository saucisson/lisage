//! Installation of sentential SDDs without *ad hoc* extension (M5-3, C6):
//! completeness test of the framework (proposition § 1.2, contribution
//! no. 6) — the vtree = structured order (parameter, `lisage-plan::vtree`),
//! the controller tower = one local anchor per vtree node in the same pool
//! (the handle (controller, anchor, form) makes canonicity C1 modular),
//! the inter-level projections = set operations on `lang/`.
//!
//! The falsification: no new mechanism in `lisage-core` — this module only
//! assembles schema + anchors + hash-consing + fold.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_algebra::vtree::{ErrorVtree, Vtree};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaError, SchemaKeys, compiled};
use lisage_lang::value::{Atom, Value};

/// Typed error of the SDD installation (causes preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorSdd {
    /// Invalid vtree (duplicate variable).
    Vtree(ErrorVtree),
    /// Vtree with no variable.
    WithoutVariable,
    /// Error compiling a projection against its schema.
    Schema(SchemaError),
    /// Core error (union, fold) during construction.
    Core(CoreError),
}

impl std::fmt::Display for ErrorSdd {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSdd::Vtree(error) => write!(formatter, "invalid vtree: {error}"),
            ErrorSdd::WithoutVariable => write!(formatter, "the vtree has no variable"),
            ErrorSdd::Schema(error) => write!(formatter, "schema error: {error}"),
            ErrorSdd::Core(error) => write!(formatter, "core error: {error}"),
        }
    }
}

impl std::error::Error for ErrorSdd {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorSdd::Vtree(error) => Some(error),
            ErrorSdd::Schema(error) => Some(error),
            ErrorSdd::Core(error) => Some(error),
            ErrorSdd::WithoutVariable => None,
        }
    }
}

impl From<ErrorVtree> for ErrorSdd {
    fn from(error: ErrorVtree) -> Self {
        ErrorSdd::Vtree(error)
    }
}

impl From<SchemaError> for ErrorSdd {
    fn from(error: SchemaError) -> Self {
        ErrorSdd::Schema(error)
    }
}

impl From<CoreError> for ErrorSdd {
    fn from(error: CoreError) -> Self {
        ErrorSdd::Core(error)
    }
}

/// One level of the controller tower: the local DD of a vtree node,
/// built under the node's local anchor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelSdd {
    /// Local anchor of the node (local controller, instance pattern).
    pub anchor: Anchor,
    /// Root of the local DD (set projected onto the variables of the subtree).
    pub root: NodeId,
    /// Cardinality of the local DD (fold `(ℕ,+,×)`).
    pub cardinal: u64,
}

/// Pool for an SDD installation: the controller carries the **root** schema
/// (all the variables of the vtree, domain of the full set) — the R-default
/// reductions at interning are driven by this schema.
pub fn pool_for_sdd(vtree: &Vtree, set: &BTreeSet<Value>) -> Arena<ReferenceController> {
    let schema = Schema::Map(vtree.order_flat(), domain_set(set));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Complete installation: one [`LevelSdd`] per vtree node, anchors in
/// pre-order. Returns the root of the root level. The pool must be created
/// by [`pool_for_sdd`] (root schema = controller schema).
pub fn install_sdd(
    pool: &mut Arena<ReferenceController>,
    vtree: &Vtree,
    set: &BTreeSet<Value>,
) -> Result<LevelSdd, ErrorSdd> {
    vtree.validate()?;
    let domain = domain_set(set);
    let mut counter_anchors = 0u32;
    install_level(pool, vtree, set, &domain, &mut counter_anchors)
}

/// Installs one level of the vtree: projection of the set onto the
/// variables of the subtree, compiled under the node's local anchor, then
/// recursion.
fn install_level(
    pool: &mut Arena<ReferenceController>,
    vtree: &Vtree,
    set: &BTreeSet<Value>,
    domain: &Domain,
    counter_anchors: &mut u32,
) -> Result<LevelSdd, ErrorSdd> {
    let variables = vtree.order_flat();
    if variables.is_empty() {
        return Err(ErrorSdd::WithoutVariable);
    }
    let anchor = Anchor(*counter_anchors);
    *counter_anchors += 1;
    let projection = projeter(set, &variables);
    let schema = Schema::Map(variables, domain.clone());
    let root = compile_projection(pool, &schema, &projection, anchor)?;
    let cardinal = count_level(pool, &schema, root, anchor)?;
    if let Vtree::Node { left, right } = vtree {
        install_level(pool, left, &projection, domain, counter_anchors)?;
        install_level(pool, right, &projection, domain, counter_anchors)?;
    }
    Ok(LevelSdd {
        anchor,
        root,
        cardinal,
    })
}

/// Projects a set of maps onto a subset of keys (inter-level transduction,
/// `lang/` level: pure set operation).
pub fn projeter(set: &BTreeSet<Value>, keys: &[Atom]) -> BTreeSet<Value> {
    set.iter()
        .filter_map(|value| match value {
            Value::Map(map) => {
                let projetee: BTreeMap<Value, Value> = map
                    .iter()
                    .filter(|(key, _)| match key {
                        Value::Atom(atom) => keys.contains(atom),
                        _ => false,
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();
                Some(Value::Map(projetee))
            }
            _ => None,
        })
        .collect()
}

/// Domain of the values of the full set: `0..=max` observed over all maps
/// (shared by all levels, so that the controller schema and the local
/// schemas have the same level cardinalities).
fn domain_set(set: &BTreeSet<Value>) -> Domain {
    let max_value = set
        .iter()
        .filter_map(|value| match value {
            Value::Map(map) => map
                .values()
                .filter_map(|v| match v {
                    Value::Atom(atom) => Some(atom.0),
                    _ => None,
                })
                .max(),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    Domain::new((0..=max_value).map(Atom).collect())
}

/// Compiles a projection into a local DD: union of the projected maps under
/// the node's local anchor (hash-consing by handle, modular C1).
fn compile_projection(
    pool: &mut Arena<ReferenceController>,
    schema: &Schema,
    projection: &BTreeSet<Value>,
    anchor: Anchor,
) -> Result<NodeId, ErrorSdd> {
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(anchor, Form::Terminal(false));
    for value in projection {
        let membre = compiled(pool, anchor, schema, value)?;
        root = union(pool, &keys, anchor, root, membre)?;
    }
    Ok(root)
}

/// Cardinality of a local DD by fold `(ℕ,+,×)`.
fn count_level(
    pool: &Arena<ReferenceController>,
    schema: &Schema,
    root: NodeId,
    anchor: Anchor,
) -> Result<u64, ErrorSdd> {
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    Ok(count(pool, &mut output, anchor, schema, root)?)
}
