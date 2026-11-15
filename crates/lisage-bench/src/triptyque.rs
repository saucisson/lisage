//! A/B/C triptych on PNML models (M5-2, statements C3/C4): the same
//! reachable marking space of a Petri net read three times —
//! - **path A** = reference interpreter `lisage-lang`: explicit set
//!   (`pnml::space_atteignable`), the set-theoretic truth;
//! - **path B** = DD with fixed order: canonical order of the places, compiled
//!   in `lisage-core` (union of the marking maps);
//! - **path C** = DD with runtime order: permutation π of the place keys
//!   supplied at run time, submitted to the `lisage-plan` planner
//!   (`Order::with_aval_runtime`) — adopted as-is if valid, otherwise
//!   canonical fallback signalled by the returned strategy.
//!
//! The C3/C4 falsification holds here: identical cardinality on the three
//! paths, whatever the downstream strategy adopted by the planner.

use std::collections::BTreeSet;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{count, union};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaError, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};
use lisage_plan::order::{KeyMap, KeysMap, Order, Phase, StrategieAval};

use crate::pnml::{self, NetworkPetri};

/// Anchor of the measurement bench.
const ANCHOR_BANC: Anchor = Anchor(0);

/// Typed error of the triptych (causes preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorTriptyque {
    /// The net has no place: no marking schema.
    WithoutPlace,
    /// π is not a permutation of the place keys (key missing, duplicated,
    /// or foreign to the net).
    OrderInvalide,
    /// Error compiling a marking against the schema.
    Schema(SchemaError),
    /// Core error (union, pli) while building the DD.
    Core(CoreError),
}

impl std::fmt::Display for ErrorTriptyque {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorTriptyque::WithoutPlace => write!(formatter, "the net has no place"),
            ErrorTriptyque::OrderInvalide => {
                write!(formatter, "π is not a permutation of the place keys")
            }
            ErrorTriptyque::Schema(error) => write!(formatter, "schema error: {error}"),
            ErrorTriptyque::Core(error) => write!(formatter, "core error: {error}"),
        }
    }
}

impl std::error::Error for ErrorTriptyque {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorTriptyque::Schema(error) => Some(error),
            ErrorTriptyque::Core(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SchemaError> for ErrorTriptyque {
    fn from(error: SchemaError) -> Self {
        ErrorTriptyque::Schema(error)
    }
}

impl From<CoreError> for ErrorTriptyque {
    fn from(error: CoreError) -> Self {
        ErrorTriptyque::Core(error)
    }
}

/// Measurement of one path of the triptych.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MeasureVoie {
    /// Cardinality of the reachable space (number of distinct markings).
    pub cardinality: u64,
    /// Live nodes of the diagram (0 for path A, explicit set).
    pub nodes_live: usize,
}

/// Path A (reference): cardinality of the explicit reachable space.
pub fn voie_a(network: &NetworkPetri) -> MeasureVoie {
    MeasureVoie {
        cardinality: pnml::space_atteignable(network).len() as u64,
        nodes_live: 0,
    }
}

/// Path B: DD of the reachable space with **fixed order** (canonical
/// order of the places, key = place index).
pub fn voie_b(network: &NetworkPetri) -> Result<MeasureVoie, ErrorTriptyque> {
    let order_canonique: Vec<Atom> = (0..network.number_places())
        .map(|index| Atom(index as u64))
        .collect();
    voie_dd(network, &order_canonique)
}

/// Result of path C: the measurement of the DD built with the adopted order,
/// and the planner's downstream strategy (runtime order adopted, or canonical
/// fallback).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResultVoieC {
    /// Measurement of the DD with the order adopted by the planner.
    pub measure: MeasureVoie,
    /// Downstream strategy: `OrderRuntime` if π was valid, `RepliCanonique`
    /// otherwise (canonical order adopted instead).
    pub strategie: StrategieAval,
}

/// Path C: DD of the reachable space with **runtime order** — π of the
/// place keys submitted to the `lisage-plan` planner (`Order::with_aval_runtime`
/// on the `KeysMap` descriptor); the adopted order (runtime or canonical) is
/// used to build the schema.
pub fn voie_c(
    network: &NetworkPetri,
    order_runtime: &[Atom],
) -> Result<ResultVoieC, ErrorTriptyque> {
    if network.number_places() == 0 {
        return Err(ErrorTriptyque::WithoutPlace);
    }
    let keys: Vec<Atom> = (0..network.number_places())
        .map(|index| Atom(index as u64))
        .collect();
    let descripteur = KeysMap(keys);
    // π runtime: presence then value for each key, in the requested order.
    let pi_runtime: Vec<KeyMap> = order_runtime
        .iter()
        .flat_map(|key| {
            [
                KeyMap {
                    key: *key,
                    phase: Phase::Presence,
                },
                KeyMap {
                    key: *key,
                    phase: Phase::Value,
                },
            ]
        })
        .collect();
    let (order, strategie) = Order::with_aval_runtime(&descripteur, pi_runtime);
    let order_adopte: Vec<Atom> = order
        .sequence()
        .iter()
        .filter_map(|key_map| match key_map.phase {
            Phase::Presence => Some(key_map.key),
            Phase::Value => None,
        })
        .collect();
    let measure = voie_dd(network, &order_adopte)?;
    Ok(ResultVoieC { measure, strategie })
}

/// Builds the DD of the reachable space in the given key order,
/// then measures (cardinality via pli, live nodes after construction).
fn voie_dd(network: &NetworkPetri, order_keys: &[Atom]) -> Result<MeasureVoie, ErrorTriptyque> {
    if network.number_places() == 0 {
        return Err(ErrorTriptyque::WithoutPlace);
    }
    let schema = schema_petri(network, order_keys)?;
    let space = pnml::space_atteignable(network);
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let root = compiler_space(&mut pool, &schema, &space)?;
    let nodes_live = pool.live();
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let cardinality = count(&pool, &mut output, ANCHOR_BANC, &schema, root)?;
    Ok(MeasureVoie {
        cardinality,
        nodes_live,
    })
}

/// `Map(K, V)` schema of a net marking: keys = places in
/// the given order (π), value domain = `0..=max_jetons` bounded by
/// the reachable space. π must be a permutation of the place indexes.
fn schema_petri(network: &NetworkPetri, order_keys: &[Atom]) -> Result<Schema, ErrorTriptyque> {
    let mut expected_f: Vec<Atom> = (0..network.number_places())
        .map(|index| Atom(index as u64))
        .collect();
    let mut recues = order_keys.to_vec();
    expected_f.sort();
    recues.sort();
    if recues != expected_f {
        return Err(ErrorTriptyque::OrderInvalide);
    }
    let max_jetons = jetons_max(network);
    let domain = Domain::new((0..=max_jetons).map(Atom).collect());
    Ok(Schema::Map(order_keys.to_vec(), domain))
}

/// Maximum number of tokens observed on a place in the
/// reachable space (bound of the value domain).
fn jetons_max(network: &NetworkPetri) -> u64 {
    pnml::space_atteignable(network)
        .iter()
        .filter_map(pnml::marquage_depuis_value)
        .flat_map(|marquage| marquage.into_values())
        .map(|jetons| jetons.0)
        .max()
        .unwrap_or(0)
}

/// Compiles the reachable space into a DD: union of the marking maps
/// compiled one by one against the schema.
fn compiler_space(
    pool: &mut Arena<ReferenceController>,
    schema: &Schema,
    space: &BTreeSet<Value>,
) -> Result<NodeId, ErrorTriptyque> {
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(ANCHOR_BANC, Form::Terminal(false));
    for value in space {
        let membre = compiler(pool, ANCHOR_BANC, schema, value)?;
        root = union(pool, &keys, ANCHOR_BANC, root, membre)?;
    }
    Ok(root)
}
