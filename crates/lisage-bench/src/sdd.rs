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
use lisage_core::pli::{compter, union};
use lisage_core::schema::{
    Domaine, ReferenceController, Schema, SchemaError, SchemaKeys, compiler,
};
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
pub struct NiveauSdd {
    /// Local anchor of the node (local controller, instance pattern).
    pub ancre: Anchor,
    /// Root of the local DD (set projected on the variables of the subtree).
    pub racine: NodeId,
    /// Cardinal of the local DD (pli `(ℕ,+,×)`).
    pub cardinal: u64,
}

/// Pool for an SDD installation: the controller carries the **root** schema
/// (all the vtree variables, domain of the complete set) — the
/// R-default reductions at interning are driven by this schema.
pub fn pool_pour_sdd(vtree: &Vtree, ensemble: &BTreeSet<Value>) -> Arena<ReferenceController> {
    let schema = Schema::Map(vtree.ordre_plat(), domaine_ensemble(ensemble));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Complete installation: one [`NiveauSdd`] per vtree node, anchors in
/// pre-order. Returns the root of the root level. The pool must be created by
/// [`pool_pour_sdd`] (root schema = controller schema).
pub fn installer_sdd(
    pool: &mut Arena<ReferenceController>,
    vtree: &Vtree,
    ensemble: &BTreeSet<Value>,
) -> Result<NiveauSdd, ErreurSdd> {
    vtree.valider()?;
    let domaine = domaine_ensemble(ensemble);
    let mut compteur_ancres = 0u32;
    installer_niveau(pool, vtree, ensemble, &domaine, &mut compteur_ancres)
}

/// Installs one vtree level: projection of the set on the variables
/// of the subtree, compiled under the node's local anchor, then recursion.
fn installer_niveau(
    pool: &mut Arena<ReferenceController>,
    vtree: &Vtree,
    ensemble: &BTreeSet<Value>,
    domaine: &Domaine,
    compteur_ancres: &mut u32,
) -> Result<NiveauSdd, ErreurSdd> {
    let variables = vtree.ordre_plat();
    if variables.is_empty() {
        return Err(ErreurSdd::SansVariable);
    }
    let ancre = Anchor(*compteur_ancres);
    *compteur_ancres += 1;
    let projection = projeter(ensemble, &variables);
    let schema = Schema::Map(variables, domaine.clone());
    let racine = compiler_projection(pool, &schema, &projection, ancre)?;
    let cardinal = compter_niveau(pool, &schema, racine, ancre)?;
    if let Vtree::Noeud { gauche, droite } = vtree {
        installer_niveau(pool, gauche, &projection, domaine, compteur_ancres)?;
        installer_niveau(pool, droite, &projection, domaine, compteur_ancres)?;
    }
    Ok(NiveauSdd {
        ancre,
        racine,
        cardinal,
    })
}

/// Projects a set of maps onto a subset of keys (inter-level
/// transduction, `lang/` level: pure set operation).
pub fn projeter(ensemble: &BTreeSet<Value>, cles: &[Atom]) -> BTreeSet<Value> {
    ensemble
        .iter()
        .filter_map(|valeur| match valeur {
            Value::Map(map) => {
                let projetee: BTreeMap<Value, Value> = map
                    .iter()
                    .filter(|(cle, _)| match cle {
                        Value::Atom(atome) => cles.contains(atome),
                        _ => false,
                    })
                    .map(|(cle, valeur)| (cle.clone(), valeur.clone()))
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
fn domaine_ensemble(ensemble: &BTreeSet<Value>) -> Domaine {
    let max_valeur = ensemble
        .iter()
        .filter_map(|valeur| match valeur {
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
    Domaine::nouveau((0..=max_valeur).map(Atom).collect())
}

/// Compiles a projection into a local DD: union of the projected maps under
/// the node's local anchor (hash-consing by handle, modular C1).
fn compiler_projection(
    pool: &mut Arena<ReferenceController>,
    schema: &Schema,
    projection: &BTreeSet<Value>,
    ancre: Anchor,
) -> Result<NodeId, ErreurSdd> {
    let keys = SchemaKeys::new(schema.clone());
    let mut racine = pool.intern(ancre, Form::Terminal(false));
    for valeur in projection {
        let membre = compiler(pool, ancre, schema, valeur)?;
        racine = union(pool, &keys, ancre, racine, membre)?;
    }
    Ok(racine)
}

/// Cardinal of a local DD via the `(ℕ,+,×)` pli.
fn compter_niveau(
    pool: &Arena<ReferenceController>,
    schema: &Schema,
    racine: NodeId,
    ancre: Anchor,
) -> Result<u64, ErreurSdd> {
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    Ok(compter(pool, &mut sortie, ancre, schema, racine)?)
}
