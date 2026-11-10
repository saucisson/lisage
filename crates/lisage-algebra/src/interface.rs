//! Interface du cœur (M1-0) : contrats vus par `algebra` du pool de `core`,
//! stockage canonique aveugle (formes et handles). Bornes `Send`/`Sync`
//! figées dès M1-0 (roadmap § 10).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Identifiant de nœud dans le pool, en ordre de création, jamais déplacé.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub u32);

/// Ancre : position absolue de la racine (encodage absolu en M1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Anchor(pub u32);

/// Empreinte du contrôleur (hash des huit champs, proposition § 1.2) :
/// plusieurs contrôleurs coexistent dans le même pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ControllerId(pub u64);

/// Handle = (contrôleur, ancre, forme) : la canonicité migre du nœud vers le
/// handle (spec § 2.4.3, énoncé C1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Handle {
    pub controller: ControllerId,
    pub anchor: Anchor,
    pub node: NodeId,
}

/// Niveau de décomposition : une clé de l'univers balayé, ordonnée.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Level(pub u32);

/// Forme d'un nœud : terminal à payload, ou nœud à arcs explicites (l'arc
/// par défaut δ(ℓ) est stocké comme les autres ; R-default le rend implicite).
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

/// Algèbre de clés : contrat porté par le contrôleur, jamais par le pool.
/// Méthodes d'instance depuis M1-3 : l'algèbre est paramétrée par le schéma
/// du contrôleur (domaines finis des niveaux), une donnée runtime.
pub trait KeyAlgebra: Send + Sync {
    /// Clé d'arc : valeur de décision d'un niveau.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Forme normale d'une clé (sinon le hash-consing violerait C1).
    fn normal_form(&self, key: &Self::Key) -> Self::Key;

    /// Cardinalité |Val(ℓ)|, qui entre dans l'agrégation des plis (A4).
    fn cardinality(&self, level: Level) -> u64;

    /// Valeur par défaut δ(ℓ) du niveau, si le niveau en a une.
    fn default(&self, level: Level) -> Option<Self::Key>;
}

/// Politique de stockage de la mémoire de niveau (M2-3, spec § 3.1) :
/// déclarée par le contrôleur, elle décide quelles entrées de
/// `M : (id(nœud), ℓ) ⇀ V` sont conservées entre deux visites d'un niveau.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PolitiqueMemoire {
    /// Pas de mémoire de niveau : les transductions à effet mémoire sont
    /// rejetées (comportement par défaut, M1).
    #[default]
    Aucune,
    /// Mémoire conservée par `(nœud, niveau)` : chaque visite d'un niveau
    /// peut lire l'état stocké par une visite antérieure du même nœud.
    ParNiveau,
}

/// Contrôleur étendu (proposition § 1.2) : instance des huit champs qui
/// décident des réductions et de l'encodage. Le pool reste aveugle.
pub trait Controller: Send + Sync {
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    /// Payload des terminaux (Bool pour un ensemble, Option(V) pour lookup…).
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;
    /// Algèbre de clés de l'instance.
    type Keys: KeyAlgebra<Key = Self::Key>;

    /// Empreinte posée dans tout handle et toute clé de cache.
    const FINGERPRINT: ControllerId;

    /// Algèbre de clés de cette instance de contrôleur.
    fn keys(&self) -> &Self::Keys;

    /// Payload du diagramme de défaut D_δ d'un niveau : l'arc δ(ℓ) implicite
    /// (R-default) dénote ce terminal. `None` : pas de défaut désigné.
    fn default_payload(&self, level: Level) -> Option<Self::Payload>;

    /// R-collapse (spec § 2.3) : tous les arcs mènent à la même cible ⇒ la
    /// forme se réduit à cette cible. Pure, testée par PBT.
    fn collapse(arcs: &BTreeMap<Self::Key, NodeId>) -> Option<NodeId> {
        crate::reduction::collapse_arcs(arcs)
    }

    /// Politique de stockage de la mémoire de niveau (M2-3, T3) : déclarée
    /// par le contrôleur. `Aucune` par défaut (M1) ; `ParNiveau` autorise
    /// les transductions à effet mémoire.
    fn politique_memoire(&self) -> PolitiqueMemoire {
        PolitiqueMemoire::Aucune
    }
}

/// Pool : port « stocker / récupérer un nœud par clé », implémenté par
/// `lisage-core` (proposition § 8 : `algebra` ne voit que ce port).
pub trait Pool: Send + Sync {
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;

    /// Empreinte du contrôleur propriétaire du pool.
    const CONTROLLER: ControllerId;

    /// Interning hash-consé à clé (contrôleur, ancre, forme réduite) :
    /// R-default puis R-collapse, R-share par la table unique.
    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId;

    /// Forme d'un nœud, ou erreur typée si l'identifiant est inconnu.
    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError>;

    /// Handle canonique d'un nœud pour cette lecture (contrôleur, ancre).
    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle;

    /// Nombre de nœuds internés, terminaux compris.
    fn node_count(&self) -> usize;
}
