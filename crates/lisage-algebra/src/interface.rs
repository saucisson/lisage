//! Interface du cœur (M1-0) : les contrats que le niveau 2 (`algebra`) voit
//! du niveau 3 (`core`). Le pool de bas niveau est un stockage canonique
//! aveugle : il ne connaît ni ordre ni sémantique, seulement des formes et
//! des handles. Les bornes `Send`/`Sync` sont figées dès cette phase
//! d'interface (feuille de route § 10) : l'exécution reste mono-thread, le
//! retrofit coûterait plus cher que la décision.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Identifiant de nœud dans le pool, en ordre de création, jamais déplacé.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub u32);

/// Ancre : position absolue de la racine du diagramme (encodage absolu en M1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Anchor(pub u32);

/// Empreinte du contrôleur : hash des huit champs du contrôleur étendu
/// (proposition § 1.2). Plusieurs contrôleurs coexistent dans le même pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ControllerId(pub u64);

/// Handle : identité canonique d'un diagramme = (contrôleur, ancre, forme).
/// La canonicité migre du nœud vers le handle (spec § 2.4.3, énoncé C1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Handle {
    pub controller: ControllerId,
    pub anchor: Anchor,
    pub node: NodeId,
}

/// Niveau de décomposition : une clé de l'univers balayé par le contrôleur.
/// Les niveaux sont ordonnés : l'ordre physique est le contrat du contrôleur.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Level(pub u32);

/// Forme d'un nœud : terminal à payload, ou nœud de niveau à arcs explicites.
/// L'arc par défaut δ(ℓ) est stocké comme les autres arcs ; quand R-default
/// le supprime, l'arc implicite dénote le diagramme de défaut désigné par le
/// contrôleur (spec § 2.3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Form<Key, Payload> {
    Terminal(Payload),
    Node {
        level: Level,
        arcs: BTreeMap<Key, NodeId>,
    },
}

/// Erreur typée du port pool : identifiants inconnus, cause préservée.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PoolError {
    UnknownNode(NodeId),
    UnknownHandle(Handle),
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::UnknownNode(node) => write!(formatter, "nœud inconnu: {node:?}"),
            PoolError::UnknownHandle(handle) => write!(formatter, "handle inconnu: {handle:?}"),
        }
    }
}

impl std::error::Error for PoolError {}

/// Algèbre de clés : contrat porté par le contrôleur, jamais par le pool
/// (proposition § 1.2) — forme normale, cardinalités, défauts.
pub trait KeyAlgebra: Send + Sync {
    /// Clé d'arc : valeur de décision d'un niveau.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Forme normale d'une clé : deux clés sémantiquement égales ont la même
    /// forme normale, sinon le hash-consing violerait C1.
    fn normal_form(key: &Self::Key) -> Self::Key;

    /// Cardinalité |Val(ℓ)| du domaine d'un niveau ; elle entre dans
    /// l'agrégation des plis (règle A4, spec § 5).
    fn cardinality(level: Level) -> u64;

    /// Valeur par défaut δ(ℓ) du niveau, quand le niveau en a une.
    fn default(level: Level) -> Option<Self::Key>;
}

/// Contrôleur étendu (proposition § 1.2) : instance des huit champs qui
/// décident des réductions et de l'encodage. Le pool reste aveugle.
pub trait Controller: Send + Sync {
    /// Clé d'arc de l'instance.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    /// Payload des terminaux (Bool pour un ensemble, Option(V) pour lookup…).
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;
    /// Algèbre de clés de l'instance.
    type Keys: KeyAlgebra<Key = Self::Key>;

    /// Empreinte du contrôleur, posée dans tout handle et toute clé de cache.
    const FINGERPRINT: ControllerId;

    /// Payload du diagramme de défaut D_δ d'un niveau : l'arc δ(ℓ) implicite
    /// (R-default) dénote ce terminal. `None` : pas de défaut désigné.
    fn default_payload(level: Level) -> Option<Self::Payload>;

    /// R-collapse (spec § 2.3) : si tous les arcs d'un nœud mènent à la même
    /// cible, la forme se réduit à cette cible. Fonction pure, testée par PBT.
    fn collapse<Key>(arcs: &BTreeMap<Key, NodeId>) -> Option<NodeId>
    where
        Key: Ord,
    {
        let mut target: Option<NodeId> = None;
        for child in arcs.values() {
            match target {
                None => target = Some(*child),
                Some(previous) if previous == *child => {}
                Some(_) => return None,
            }
        }
        target
    }
}

/// Pool : port « stocker / récupérer un nœud par clé », implémenté par
/// `lisage-core` (proposition § 8 : `algebra` ne connaît pas `core`, il ne
/// voit que ce port).
pub trait Pool: Send + Sync {
    /// Clé d'arc de l'instance.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    /// Payload des terminaux de l'instance.
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;

    /// Empreinte du contrôleur propriétaire du pool.
    const CONTROLLER: ControllerId;

    /// Interning hash-consé à clé (contrôleur, ancre, forme réduite) :
    /// applique R-default puis R-collapse, puis R-share (table unique).
    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId;

    /// Forme d'un nœud, ou erreur typée si l'identifiant est inconnu.
    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError>;

    /// Handle canonique d'un nœud pour cette lecture (contrôleur, ancre) :
    /// la table des handles se clé sur la forme réduite pour cette lecture.
    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle;

    /// Nombre de nœuds internés, terminaux compris.
    fn node_count(&self) -> usize;
}
