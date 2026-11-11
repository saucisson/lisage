//! Triptyque A/B/C sur modèles PNML (M5-2, énoncés C3/C4) : le même
//! espace des marquages atteignables d'un réseau de Petri lu trois fois —
//! - **voie A** = interprète de référence `lisage-lang` : ensemble explicite
//!   (`pnml::espace_atteignable`), la vérité ensembliste ;
//! - **voie B** = DD à ordre fixe : ordre canonique des places, compilé
//!   dans `lisage-core` (union des maps de marquage) ;
//! - **voie C** = DD à ordre runtime : permutation π des clés de places
//!   fournie à l'exécution, adoptée telle quelle si c'est une permutation.
//!
//! Le câblage de π au planner `lisage-plan` (`Order::avec_aval_runtime`)
//! reste pour une PR suivante : la frontière ADR-0002 n'autorise pas
//! `lisage-bench → lisage-plan` aujourd'hui. La falsification C3/C4 tient
//! ici : cardinal identique sur les trois voies, et la voie C accepte un
//! ordre runtime arbitraire (valide) sans changer le cardinal.

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

use crate::pnml::{self, ReseauPetri};

/// Ancre du banc de mesures.
const ANCRE_BANC: Anchor = Anchor(0);

/// Erreur typée du triptyque (causes préservées).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurTriptyque {
    /// Le réseau n'a aucune place : pas de schéma de marquage.
    SansPlace,
    /// π n'est pas une permutation des clés de places (clé manquante, en
    /// double, ou étrangère au réseau).
    OrdreInvalide,
    /// Erreur de compilation d'un marquage contre le schéma.
    Schema(SchemaError),
    /// Erreur du cœur (union, pli) lors de la construction du DD.
    Core(CoreError),
}

impl std::fmt::Display for ErreurTriptyque {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurTriptyque::SansPlace => write!(formatter, "le réseau n'a aucune place"),
            ErreurTriptyque::OrdreInvalide => {
                write!(formatter, "π n'est pas une permutation des clés de places")
            }
            ErreurTriptyque::Schema(erreur) => write!(formatter, "erreur de schéma: {erreur}"),
            ErreurTriptyque::Core(erreur) => write!(formatter, "erreur du cœur: {erreur}"),
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

/// Mesure d'une voie du triptyque.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MesureVoie {
    /// Cardinal de l'espace atteignable (nombre de marquages distincts).
    pub cardinal: u64,
    /// Nœuds vivants du diagramme (0 pour la voie A, ensembliste explicite).
    pub noeuds_vivants: usize,
}

/// Voie A (référence) : cardinal de l'espace atteignable explicite.
pub fn voie_a(reseau: &ReseauPetri) -> MesureVoie {
    MesureVoie {
        cardinal: pnml::espace_atteignable(reseau).len() as u64,
        noeuds_vivants: 0,
    }
}

/// Voie B : DD de l'espace atteignable à **ordre fixe** (ordre canonique
/// des places, clé = index de place).
pub fn voie_b(reseau: &ReseauPetri) -> Result<MesureVoie, ErreurTriptyque> {
    let ordre_canonique: Vec<Atom> = (0..reseau.nombre_places())
        .map(|index| Atom(index as u64))
        .collect();
    voie_dd(reseau, &ordre_canonique)
}

/// Voie C : DD de l'espace atteignable à **ordre runtime** — π des clés de
/// places fourni à l'exécution, adopté tel quel s'il est valide.
pub fn voie_c(reseau: &ReseauPetri, ordre_runtime: &[Atom]) -> Result<MesureVoie, ErreurTriptyque> {
    voie_dd(reseau, ordre_runtime)
}

/// Construit le DD de l'espace atteignable dans l'ordre de clés donné,
/// puis mesure (cardinal par pli, nœuds vivants après construction).
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

/// Schéma `Map(K, V)` d'un marquage de réseau : clés = places dans
/// l'ordre donné (π), domaine des valeurs = `0..=max_jetons` borné par
/// l'espace atteignable. π doit être une permutation des index de places.
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

/// Nombre maximal de jetons observés sur une place dans l'espace
/// atteignable (borne du domaine des valeurs).
fn jetons_max(reseau: &ReseauPetri) -> u64 {
    pnml::espace_atteignable(reseau)
        .iter()
        .filter_map(pnml::marquage_depuis_value)
        .flat_map(|marquage| marquage.into_values())
        .map(|jetons| jetons.0)
        .max()
        .unwrap_or(0)
}

/// Compile l'espace atteignable en un DD : union des maps de marquage
/// compilées une à une contre le schéma.
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
