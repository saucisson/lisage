//! Installation of sentential SDDs without *ad hoc* extension (M5-3, C6):
//! completeness test of the framework (proposition § 1.2, contribution n° 6) —
//! the vtree = structured order (parameter, `lisage-plan::vtree`), the tower of
//! controllers = one local anchor per vtree node in the same pool
//! (the handle (controller, anchor, form) makes the C1 canonicity modular),
//! the inter-level projections = set operations on `lang/`.
//!
//! The falsification: no new mechanism in `lisage-core` — this
//! module only assembles schema + anchors + hash-consing + pli.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_algebra::vtree::{ErreurVtree, Vtree};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{count, union};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaError, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Typed error of the SDD installation (causes preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurSdd {
    /// Invalid vtree (duplicated variable).
    Vtree(ErreurVtree),
    /// Vtree with no variable at all.
    SansVariable,
    /// Error compiling a projection against its schema.
    Schema(SchemaError),
    /// Core error (union, pli) during the construction.
    Core(CoreError),
}

impl std::fmt::Display for ErreurSdd {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurSdd::Vtree(erreur) => write!(formatter, "invalid vtree: {erreur}"),
            ErreurSdd::SansVariable => write!(formatter, "the vtree has no variable"),
            ErreurSdd::Schema(erreur) => write!(formatter, "schema error: {erreur}"),
            ErreurSdd::Core(erreur) => write!(formatter, "core error: {erreur}"),
        }
    }
}

impl std::error::Error for ErreurSdd {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErreurSdd::Vtree(erreur) => Some(erreur),
            ErreurSdd::Schema(erreur) => Some(erreur),
            ErreurSdd::Core(erreur) => Some(erreur),
            ErreurSdd::SansVariable => None,
        }
    }
}

impl From<ErreurVtree> for ErreurSdd {
    fn from(erreur: ErreurVtree) -> Self {
        ErreurSdd::Vtree(erreur)
    }
}

impl From<SchemaError> for ErreurSdd {
    fn from(erreur: SchemaError) -> Self {
        ErreurSdd::Schema(erreur)
    }
}

impl From<CoreError> for ErreurSdd {
    fn from(erreur: CoreError) -> Self {
        ErreurSdd::Core(erreur)
    }
}

/// One level of the controller tower: the local DD of a vtree node,
/// built under the node's local anchor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelSdd {
    /// Local anchor of the node (local controller, instance pattern).
    pub anchor: Anchor,
    /// Root of the local DD (set projected on the variables of the subtree).
    pub root: NodeId,
    /// Cardinality of the local DD (pli `(ℕ,+,×)`).
    pub cardinality: u64,
}

/// Pool for an SDD installation: the controller carries the **root** schema
/// (all the vtree variables, domain of the complete set) — the
/// R-default reductions at interning are driven by this schema.
pub fn pool_pour_sdd(vtree: &Vtree, ensemble: &BTreeSet<Value>) -> Arena<ReferenceController> {
    let schema = Schema::Map(vtree.order_plat(), domain_ensemble(ensemble));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Complete installation: one [`LevelSdd`] per vtree node, anchors in
/// pre-order. Returns the root of the root level. The pool must be created by
/// [`pool_pour_sdd`] (root schema = controller schema).
pub fn installer_sdd(
    pool: &mut Arena<ReferenceController>,
    vtree: &Vtree,
    ensemble: &BTreeSet<Value>,
) -> Result<LevelSdd, ErreurSdd> {
    vtree.valider()?;
    let domain = domain_ensemble(ensemble);
    let mut counter_anchors = 0u32;
    installer_level(pool, vtree, ensemble, &domain, &mut counter_anchors)
}

/// Installs one vtree level: projection of the set on the variables
/// of the subtree, compiled under the node's local anchor, then recursion.
fn installer_level(
    pool: &mut Arena<ReferenceController>,
    vtree: &Vtree,
    ensemble: &BTreeSet<Value>,
    domain: &Domain,
    counter_anchors: &mut u32,
) -> Result<LevelSdd, ErreurSdd> {
    let variables = vtree.order_plat();
    if variables.is_empty() {
        return Err(ErreurSdd::SansVariable);
    }
    let anchor = Anchor(*counter_anchors);
    *counter_anchors += 1;
    let projection = projeter(ensemble, &variables);
    let schema = Schema::Map(variables, domain.clone());
    let root = compiler_projection(pool, &schema, &projection, anchor)?;
    let cardinality = count_level(pool, &schema, root, anchor)?;
    if let Vtree::Node { gauche, droite } = vtree {
        installer_level(pool, gauche, &projection, domain, counter_anchors)?;
        installer_level(pool, droite, &projection, domain, counter_anchors)?;
    }
    Ok(LevelSdd {
        anchor,
        root,
        cardinality,
    })
}

/// Projects a set of maps onto a subset of keys (inter-level
/// transduction, `lang/` level: pure set operation).
pub fn projeter(ensemble: &BTreeSet<Value>, keys: &[Atom]) -> BTreeSet<Value> {
    ensemble
        .iter()
        .filter_map(|value| match value {
            Value::Map(map) => {
                let projetee: BTreeMap<Value, Value> = map
                    .iter()
                    .filter(|(key, _)| match key {
                        Value::Atom(atome) => keys.contains(atome),
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

/// Value domain of the complete set: `0..=max` observed over all
/// the maps (shared by all levels, so that the controller schema
/// and the local schemas have the same level cardinalities).
fn domain_ensemble(ensemble: &BTreeSet<Value>) -> Domain {
    let max_value = ensemble
        .iter()
        .filter_map(|value| match value {
            Value::Map(map) => map
                .values()
                .filter_map(|v| match v {
                    Value::Atom(atome) => Some(atome.0),
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
fn compiler_projection(
    pool: &mut Arena<ReferenceController>,
    schema: &Schema,
    projection: &BTreeSet<Value>,
    anchor: Anchor,
) -> Result<NodeId, ErreurSdd> {
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(anchor, Form::Terminal(false));
    for value in projection {
        let membre = compiler(pool, anchor, schema, value)?;
        root = union(pool, &keys, anchor, root, membre)?;
    }
    Ok(root)
}

/// Cardinality of a local DD via the `(ℕ,+,×)` pli.
fn count_level(
    pool: &Arena<ReferenceController>,
    schema: &Schema,
    root: NodeId,
    anchor: Anchor,
) -> Result<u64, ErreurSdd> {
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    Ok(count(pool, &mut output, anchor, schema, root)?)
}
