//! A/B/C triptych on PNML models (M5-2, statements C3/C4): the same
//! reachable marking space of a Petri net read three times —
//! - **path A** = reference interpreter `lisage-lang`: explicit set
//!   (`pnml::espace_atteignable`), the set-theoretic truth;
//! - **path B** = DD with fixed order: canonical order of the places, compiled
//!   in `lisage-core` (union of the marking maps);
//! - **path C** = DD with runtime order: permutation π of the place keys
//!   supplied at run time, submitted to the `lisage-plan` planner
//!   (`Order::avec_aval_runtime`) — adopted as-is if valid, otherwise
//!   canonical fallback signalled by the returned strategy.
//!
//! The C3/C4 falsification holds here: identical cardinal on the three
//! paths, whatever the downstream strategy adopted by the planner.

use std::collections::BTreeSet;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::schema::{
    Domaine, ReferenceController, Schema, SchemaError, SchemaKeys, compiler,
};
use lisage_lang::value::{Atom, Value};
use lisage_plan::ordre::{CleMap, ClesMap, Order, Phase, StrategieAval};

use crate::pnml::{self, ReseauPetri};

/// Anchor of the measurement bench.
const ANCRE_BANC: Anchor = Anchor(0);

/// Typed error of the triptych (causes preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurTriptyque {
    /// The net has no place: no marking schema.
    SansPlace,
    /// π is not a permutation of the place keys (key missing, duplicated,
    /// or foreign to the net).
    OrdreInvalide,
    /// Error compiling a marking against the schema.
    Schema(SchemaError),
    /// Core error (union, pli) while building the DD.
    Core(CoreError),
}

impl std::fmt::Display for ErreurTriptyque {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurTriptyque::SansPlace => write!(formatter, "the net has no place"),
            ErreurTriptyque::OrdreInvalide => {
                write!(formatter, "π is not a permutation of the place keys")
            }
            ErreurTriptyque::Schema(erreur) => write!(formatter, "schema error: {erreur}"),
            ErreurTriptyque::Core(erreur) => write!(formatter, "core error: {erreur}"),
        }
    }
}

impl std::error::Error for ErreurTriptyque {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErreurTriptyque::Schema(erreur) => Some(erreur),
            ErreurTriptyque::Core(erreur) => Some(erreur),
            _ => None,
        }
    }
}

impl From<SchemaError> for ErreurTriptyque {
    fn from(erreur: SchemaError) -> Self {
        ErreurTriptyque::Schema(erreur)
    }
}

impl From<CoreError> for ErreurTriptyque {
    fn from(erreur: CoreError) -> Self {
        ErreurTriptyque::Core(erreur)
    }
}

/// Measurement of one path of the triptych.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MesureVoie {
    /// Cardinal of the reachable space (number of distinct markings).
    pub cardinal: u64,
    /// Live nodes of the diagram (0 for path A, explicit set).
    pub noeuds_vivants: usize,
}

/// Path A (reference): cardinal of the explicit reachable space.
pub fn voie_a(reseau: &ReseauPetri) -> MesureVoie {
    MesureVoie {
        cardinal: pnml::espace_atteignable(reseau).len() as u64,
        noeuds_vivants: 0,
    }
}

/// Path B: DD of the reachable space with **fixed order** (canonical
/// order of the places, key = place index).
pub fn voie_b(reseau: &ReseauPetri) -> Result<MesureVoie, ErreurTriptyque> {
    let ordre_canonique: Vec<Atom> = (0..reseau.nombre_places())
        .map(|index| Atom(index as u64))
        .collect();
    voie_dd(reseau, &ordre_canonique)
}

/// Result of path C: the measurement of the DD built with the adopted order,
/// and the planner's downstream strategy (runtime order adopted, or canonical
/// fallback).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResultatVoieC {
    /// Measurement of the DD with the order adopted by the planner.
    pub mesure: MesureVoie,
    /// Downstream strategy: `OrdreRuntime` if π was valid, `RepliCanonique`
    /// otherwise (canonical order adopted instead).
    pub strategie: StrategieAval,
}

/// Path C: DD of the reachable space with **runtime order** — π of the
/// place keys submitted to the `lisage-plan` planner (`Order::avec_aval_runtime`
/// on the `ClesMap` descriptor); the adopted order (runtime or canonical) is
/// used to build the schema.
pub fn voie_c(
    reseau: &ReseauPetri,
    ordre_runtime: &[Atom],
) -> Result<ResultatVoieC, ErreurTriptyque> {
    if reseau.nombre_places() == 0 {
        return Err(ErreurTriptyque::SansPlace);
    }
    let cles: Vec<Atom> = (0..reseau.nombre_places())
        .map(|index| Atom(index as u64))
        .collect();
    let descripteur = ClesMap(cles);
    // π runtime: presence then value for each key, in the requested order.
    let pi_runtime: Vec<CleMap> = ordre_runtime
        .iter()
        .flat_map(|cle| {
            [
                CleMap {
                    cle: *cle,
                    phase: Phase::Presence,
                },
                CleMap {
                    cle: *cle,
                    phase: Phase::Valeur,
                },
            ]
        })
        .collect();
    let (ordre, strategie) = Order::avec_aval_runtime(&descripteur, pi_runtime);
    let ordre_adopte: Vec<Atom> = ordre
        .sequence()
        .iter()
        .filter_map(|cle_map| match cle_map.phase {
            Phase::Presence => Some(cle_map.cle),
            Phase::Valeur => None,
        })
        .collect();
    let mesure = voie_dd(reseau, &ordre_adopte)?;
    Ok(ResultatVoieC { mesure, strategie })
}

/// Builds the DD of the reachable space in the given key order,
/// then measures (cardinal via pli, live nodes after construction).
fn voie_dd(reseau: &ReseauPetri, ordre_cles: &[Atom]) -> Result<MesureVoie, ErreurTriptyque> {
    if reseau.nombre_places() == 0 {
        return Err(ErreurTriptyque::SansPlace);
    }
    let schema = schema_petri(reseau, ordre_cles)?;
    let espace = pnml::espace_atteignable(reseau);
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let racine = compiler_espace(&mut pool, &schema, &espace)?;
    let noeuds_vivants = pool.vivants();
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let cardinal = compter(&pool, &mut sortie, ANCRE_BANC, &schema, racine)?;
    Ok(MesureVoie {
        cardinal,
        noeuds_vivants,
    })
}

/// `Map(K, V)` schema of a net marking: keys = places in
/// the given order (π), value domain = `0..=max_jetons` bounded by
/// the reachable space. π must be a permutation of the place indexes.
fn schema_petri(reseau: &ReseauPetri, ordre_cles: &[Atom]) -> Result<Schema, ErreurTriptyque> {
    let mut attendues: Vec<Atom> = (0..reseau.nombre_places())
        .map(|index| Atom(index as u64))
        .collect();
    let mut recues = ordre_cles.to_vec();
    attendues.sort();
    recues.sort();
    if recues != attendues {
        return Err(ErreurTriptyque::OrdreInvalide);
    }
    let max_jetons = jetons_max(reseau);
    let domaine = Domaine::nouveau((0..=max_jetons).map(Atom).collect());
    Ok(Schema::Map(ordre_cles.to_vec(), domaine))
}

/// Maximum number of tokens observed on a place in the
/// reachable space (bound of the value domain).
fn jetons_max(reseau: &ReseauPetri) -> u64 {
    pnml::espace_atteignable(reseau)
        .iter()
        .filter_map(pnml::marquage_depuis_value)
        .flat_map(|marquage| marquage.into_values())
        .map(|jetons| jetons.0)
        .max()
        .unwrap_or(0)
}

/// Compiles the reachable space into a DD: union of the marking maps
/// compiled one by one against the schema.
fn compiler_espace(
    pool: &mut Arena<ReferenceController>,
    schema: &Schema,
    espace: &BTreeSet<Value>,
) -> Result<NodeId, ErreurTriptyque> {
    let keys = SchemaKeys::new(schema.clone());
    let mut racine = pool.intern(ANCRE_BANC, Form::Terminal(false));
    for valeur in espace {
        let membre = compiler(pool, ANCRE_BANC, schema, valeur)?;
        racine = union(pool, &keys, ANCRE_BANC, racine, membre)?;
    }
    Ok(racine)
}
