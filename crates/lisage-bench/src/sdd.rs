//! Installation des SDD sententiels sans extension *ad hoc* (M5-3, C6) :
//! test de complétude du cadre (proposition § 1.2, contribution n° 6) —
//! le vtree = ordre structuré (paramètre, `lisage-plan::vtree`), la tour de
//! contrôleurs = une ancre locale par nœud du vtree dans le même pool
//! (le handle (contrôleur, ancre, forme) rend la canonicité C1 modulaire),
//! les projections inter-niveaux = opérations ensemblistes sur `lang/`.
//!
//! La falsification : aucun mécanisme nouveau dans `lisage-core` — ce
//! module ne fait qu'assembler schéma + ancres + hash-consing + pli.

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

/// Erreur typée de l'installation SDD (causes préservées).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurSdd {
    /// Vtree invalide (variable en double).
    Vtree(ErreurVtree),
    /// Vtree sans aucune variable.
    SansVariable,
    /// Erreur de compilation d'une projection contre son schéma.
    Schema(SchemaError),
    /// Erreur du cœur (union, pli) lors de la construction.
    Core(CoreError),
}

impl std::fmt::Display for ErreurSdd {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurSdd::Vtree(erreur) => write!(formatter, "vtree invalide: {erreur}"),
            ErreurSdd::SansVariable => write!(formatter, "le vtree n'a aucune variable"),
            ErreurSdd::Schema(erreur) => write!(formatter, "erreur de schéma: {erreur}"),
            ErreurSdd::Core(erreur) => write!(formatter, "erreur du cœur: {erreur}"),
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

/// Un niveau de la tour de contrôleurs : le DD local d'un nœud du vtree,
/// construit sous l'ancre locale du nœud.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NiveauSdd {
    /// Ancre locale du nœud (contrôleur local, motif d'instance).
    pub ancre: Anchor,
    /// Racine du DD local (ensemble projeté sur les variables du sous-arbre).
    pub racine: NodeId,
    /// Cardinal du DD local (pli `(ℕ,+,×)`).
    pub cardinal: u64,
}

/// Pool pour une installation SDD : le contrôleur porte le schéma **racine**
/// (toutes les variables du vtree, domaine de l'ensemble complet) — les
/// réductions R-default à l'internement sont pilotées par ce schéma.
pub fn pool_pour_sdd(vtree: &Vtree, ensemble: &BTreeSet<Value>) -> Arena<ReferenceController> {
    let schema = Schema::Map(vtree.ordre_plat(), domaine_ensemble(ensemble));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Installation complète : un [`NiveauSdd`] par nœud du vtree, ancres en
/// pré-ordre. Renvoie la racine du niveau racine. Le pool doit être créé par
/// [`pool_pour_sdd`] (schéma racine = schéma du contrôleur).
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

/// Installe un niveau du vtree : projection de l'ensemble sur les variables
/// du sous-arbre, compilée sous l'ancre locale du nœud, puis récursion.
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

/// Projette un ensemble de maps sur un sous-ensemble de clés (transduction
/// inter-niveaux, niveau `lang/` : opération ensembliste pure).
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

/// Domaine des valeurs de l'ensemble complet : `0..=max` observé sur toutes
/// les maps (partagé par tous les niveaux, pour que le schéma du contrôleur
/// et les schémas locaux aient les mêmes cardinalités de niveaux).
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

/// Compile une projection en DD local : union des maps projetées sous
/// l'ancre locale du nœud (hash-consing par handle, C1 modulaire).
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

/// Cardinal d'un DD local par pli `(ℕ,+,×)`.
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
