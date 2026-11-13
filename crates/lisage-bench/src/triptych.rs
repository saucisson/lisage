//! Triptych A/B/C on PNML models (M5-2, statements C3/C4): the same
//! space of reachable markings of a Petri net read three times —
//! - **lane A** = reference interpreter `lisage-lang`: explicit set
//!   (`pnml::space_reachable`), the set-theoretic truth;
//! - **lane B** = fixed-order DD: canonical order of the places, compiled
//!   in `lisage-core` (union of the marking maps);
//! - **lane C** = runtime-order DD: permutation π of the place keys
//!   supplied at runtime, submitted to the `lisage-plan` planner
//!   (`Order::with_downstream_runtime`) — adopted as-is if valid, otherwise
//!   canonical fallback reported by the returned strategy.
//!
//! The C3/C4 falsification holds here: identical cardinality on all three
//! lanes, whatever downstream strategy the planner adopts.

use std::collections::BTreeSet;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaError, SchemaKeys, compiled};
use lisage_lang::value::{Atom, Value};
use lisage_plan::order::{KeyMap, KeysMap, Order, Phase, StrategieDownstream};

use crate::pnml::{self, NetworkPetri};

/// Anchor of the measurement bench.
const ANCRE_BANC: Anchor = Anchor(0);

/// Typed error of the triptych (causes preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorTriptych {
    /// The network has no place: no marking schema.
    WithoutPlace,
    /// π is not a permutation of the place keys (missing key, duplicate, or
    /// foreign to the network).
    OrderInvalid,
    /// Error compiling a marking against the schema.
    Schema(SchemaError),
    /// Core error (union, fold) while building the DD.
    Core(CoreError),
}

impl std::fmt::Display for ErrorTriptych {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorTriptych::WithoutPlace => write!(formatter, "the network has no place"),
            ErrorTriptych::OrderInvalid => {
                write!(formatter, "π is not a permutation of the place keys")
            }
            ErrorTriptych::Schema(error) => write!(formatter, "schema error: {error}"),
            ErrorTriptych::Core(error) => write!(formatter, "core error: {error}"),
        }
    }
}

impl std::error::Error for ErrorTriptych {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorTriptych::Schema(error) => Some(error),
            ErrorTriptych::Core(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SchemaError> for ErrorTriptych {
    fn from(error: SchemaError) -> Self {
        ErrorTriptych::Schema(error)
    }
}

impl From<CoreError> for ErrorTriptych {
    fn from(error: CoreError) -> Self {
        ErrorTriptych::Core(error)
    }
}

/// Measurement of one lane of the triptych.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MeasureLane {
    /// Cardinality of the reachable space (number of distinct markings).
    pub cardinal: u64,
    /// Live nodes of the diagram (0 for lane A, explicit set).
    pub nodes_live: usize,
}

/// Lane A (reference): cardinality of the explicit reachable space.
pub fn lane_a(network: &NetworkPetri) -> MeasureLane {
    MeasureLane {
        cardinal: pnml::space_reachable(network).len() as u64,
        nodes_live: 0,
    }
}

/// Lane B: DD of the reachable space at **fixed order** (canonical order
/// of the places, key = place index).
pub fn lane_b(network: &NetworkPetri) -> Result<MeasureLane, ErrorTriptych> {
    let order_canonical: Vec<Atom> = (0..network.number_places())
        .map(|index| Atom(index as u64))
        .collect();
    lane_dd(network, &order_canonical)
}

/// Result of lane C: the measurement of the DD built at the adopted order,
/// and the planner's downstream strategy (runtime order adopted, or
/// canonical fallback).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResultLaneC {
    /// Measurement of the DD at the order adopted by the planner.
    pub measure: MeasureLane,
    /// Downstream strategy: `OrderRuntime` if π was valid, `FoldCanonical`
    /// otherwise (canonical order adopted instead).
    pub strategie: StrategieDownstream,
}

/// Lane C: DD of the reachable space at **runtime order** — π of the place
/// keys submitted to the `lisage-plan` planner (`Order::with_downstream_runtime`
/// on the `KeysMap` descriptor); the adopted order (runtime or canonical) is
/// used to build the schema.
pub fn lane_c(
    network: &NetworkPetri,
    order_runtime: &[Atom],
) -> Result<ResultLaneC, ErrorTriptych> {
    if network.number_places() == 0 {
        return Err(ErrorTriptych::WithoutPlace);
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
    let (order, strategie) = Order::with_downstream_runtime(&descripteur, pi_runtime);
    let order_adopted: Vec<Atom> = order
        .sequence()
        .iter()
        .filter_map(|key_map| match key_map.phase {
            Phase::Presence => Some(key_map.key),
            Phase::Value => None,
        })
        .collect();
    let measure = lane_dd(network, &order_adopted)?;
    Ok(ResultLaneC { measure, strategie })
}

/// Builds the DD of the reachable space in the given key order, then
/// measures (cardinality by fold, live nodes after construction).
fn lane_dd(network: &NetworkPetri, order_keys: &[Atom]) -> Result<MeasureLane, ErrorTriptych> {
    if network.number_places() == 0 {
        return Err(ErrorTriptych::WithoutPlace);
    }
    let schema = schema_petri(network, order_keys)?;
    let space = pnml::space_reachable(network);
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let root = compile_space(&mut pool, &schema, &space)?;
    let nodes_live = pool.live();
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let cardinal = count(&pool, &mut output, ANCRE_BANC, &schema, root)?;
    Ok(MeasureLane {
        cardinal,
        nodes_live,
    })
}

/// `Map(K, V)` schema of a network marking: keys = places in the given
/// order (π), value domain = `0..=max_tokens` bounded by the reachable
/// space. π must be a permutation of the place indices.
fn schema_petri(network: &NetworkPetri, order_keys: &[Atom]) -> Result<Schema, ErrorTriptych> {
    let mut attendues: Vec<Atom> = (0..network.number_places())
        .map(|index| Atom(index as u64))
        .collect();
    let mut recues = order_keys.to_vec();
    attendues.sort();
    recues.sort();
    if recues != attendues {
        return Err(ErrorTriptych::OrderInvalid);
    }
    let max_tokens = tokens_max(network);
    let domain = Domain::new((0..=max_tokens).map(Atom).collect());
    Ok(Schema::Map(order_keys.to_vec(), domain))
}

/// Maximum number of tokens observed on a place in the reachable space
/// (bound of the value domain).
fn tokens_max(network: &NetworkPetri) -> u64 {
    pnml::space_reachable(network)
        .iter()
        .filter_map(pnml::marking_since_value)
        .flat_map(|marking| marking.into_values())
        .map(|tokens| tokens.0)
        .max()
        .unwrap_or(0)
}

/// Compiles the reachable space into a DD: union of the marking maps
/// compiled one by one against the schema.
fn compile_space(
    pool: &mut Arena<ReferenceController>,
    schema: &Schema,
    space: &BTreeSet<Value>,
) -> Result<NodeId, ErrorTriptych> {
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(ANCRE_BANC, Form::Terminal(false));
    for value in space {
        let membre = compiled(pool, ANCRE_BANC, schema, value)?;
        root = union(pool, &keys, ANCRE_BANC, root, membre)?;
    }
    Ok(root)
}
