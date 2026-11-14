//! Core interface (M1-0): contracts seen by `algebra` from the `core` pool,
//! blind canonical storage (forms and handles). `Send`/`Sync` bounds
//! frozen from M1-0 (roadmap § 10).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Node identifier in the pool, in creation order, never moved.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeId(pub u32);

/// Anchor: absolute position of the root (absolute encoding in M1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Anchor(pub u32);

/// Controller fingerprint (hash of the eight fields, proposal § 1.2):
/// several controllers coexist in the same pool.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ControllerId(pub u64);

/// Handle = (controller, anchor, form): canonicity migrates from the node to
/// the handle (spec § 2.4.3, claim C1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Handle {
    pub controller: ControllerId,
    pub anchor: Anchor,
    pub node: NodeId,
}

/// Decomposition level: a key of the swept universe, ordered.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Level(pub u32);

/// Form of a node: terminal with payload, or node with explicit arcs (the
/// default arc δ(ℓ) is stored like the others; R-default makes it implicit).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Form<Key, Payload> {
    Terminal(Payload),
    Node {
        level: Level,
        arcs: BTreeMap<Key, NodeId>,
    },
}

/// Typed error of the pool port: unknown identifiers, cause preserved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PoolError {
    UnknownNode(NodeId),
    UnknownHandle(Handle),
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::UnknownNode(node) => write!(formatter, "unknown node: {node:?}"),
            PoolError::UnknownHandle(handle) => write!(formatter, "unknown handle: {handle:?}"),
        }
    }
}

impl std::error::Error for PoolError {}

/// Key algebra: contract carried by the controller, never by the pool.
/// Instance methods from M1-3: the algebra is parameterized by the
/// controller's schema (finite domains of levels), runtime data.
pub trait KeyAlgebra: Send + Sync {
    /// Arc key: decision value of a level.
    type Key: Clone + Ord + Hash + Debug + Send + Sync;

    /// Normal form of a key (otherwise hash-consing would violate C1).
    fn normal_form(&self, key: &Self::Key) -> Self::Key;

    /// Cardinality |Val(ℓ)|, which enters the fold aggregation (A4).
    fn cardinality(&self, level: Level) -> u64;

    /// Default value δ(ℓ) of the level, if the level has one.
    fn default(&self, level: Level) -> Option<Self::Key>;
}

/// Level-memory storage policy (M2-3, spec § 3.1): declared by the
/// controller, it decides which entries of `M : (node id, ℓ) ⇀ V` are kept
/// between two visits to a level.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PolitiqueMemoire {
    /// No level memory: memory-effect transductions are rejected
    /// (default behavior, M1).
    #[default]
    Aucune,
    /// Memory kept per `(node, level)`: each visit to a level can read the
    /// state stored by a previous visit to the same node.
    ParNiveau,
}

/// Extended controller (proposal § 1.2): instance of the eight fields that
/// decide reductions and encoding. The pool stays blind.
pub trait Controller: Send + Sync {
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    /// Payload of terminals (Bool for a set, Option(V) for lookup…).
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;
    /// Key algebra of the instance.
    type Keys: KeyAlgebra<Key = Self::Key>;

    /// Fingerprint placed in every handle and every cache key.
    const FINGERPRINT: ControllerId;

    /// Key algebra of this controller instance.
    fn keys(&self) -> &Self::Keys;

    /// Payload of the default diagram D_δ of a level: the implicit arc δ(ℓ)
    /// (R-default) denotes this terminal. `None`: no designated default.
    fn default_payload(&self, level: Level) -> Option<Self::Payload>;

    /// R-collapse (spec § 2.3): all arcs lead to the same target ⇒ the
    /// form reduces to that target. Pure, tested by PBT.
    fn collapse(arcs: &BTreeMap<Self::Key, NodeId>) -> Option<NodeId> {
        crate::reduction::collapse_arcs(arcs)
    }

    /// R-collapse active? `false` switches the reduction off (didactic
    /// workshop, backlog § 9: each reduction toggled one by one).
    fn collapse_actif(&self) -> bool {
        true
    }

    /// R-default active? `false` switches off the absorption of the arc δ(ℓ)
    /// leading to D_δ (didactic workshop, same).
    fn defaut_actif(&self) -> bool {
        true
    }

    /// Level-memory storage policy (M2-3, T3): declared by the controller.
    /// `Aucune` by default (M1); `ParNiveau` allows memory-effect transductions.
    fn politique_memoire(&self) -> PolitiqueMemoire {
        PolitiqueMemoire::Aucune
    }

    /// Generalized default (D-abs/D-trans, spec § 2.4.1, M4-3, claim A6):
    /// absorption of arcs toward D^default over **any subdomain** (not
    /// only δ(ℓ)) and transparent node (A = ∅ or all targets ≡ D →
    /// D). `false` by default: § 2.3 system (R-default on δ(ℓ) only).
    fn absorption_generalisee(&self) -> bool {
        false
    }

    /// Key transformation group designated by the controller (M4-2,
    /// spec § 2.4.5): normalizes forms modulo G by canonical section
    /// of orbits, the argument living on the incoming arc. `None` by default:
    /// no relative forms (M1, absolute encoding).
    fn groupe_cles(
        &self,
    ) -> Option<&dyn crate::relatif::GroupeCles<Cle = Self::Key, Argument = i64>> {
        None
    }
}

/// Pool: port "store / retrieve a node by key", implemented by
/// `lisage-core` (proposal § 8: `algebra` only sees this port).
pub trait Pool: Send + Sync {
    type Key: Clone + Ord + Hash + Debug + Send + Sync;
    type Payload: Clone + Eq + Hash + Debug + Send + Sync;

    /// Fingerprint of the pool's owning controller.
    const CONTROLLER: ControllerId;

    /// Hash-consed interning by key (controller, anchor, reduced form):
    /// R-default then R-collapse, R-share via the unique table.
    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId;

    /// Form of a node, or typed error if the identifier is unknown.
    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError>;

    /// Canonical handle of a node for this reading (controller, anchor).
    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle;

    /// Number of interned nodes, terminals included.
    fn node_count(&self) -> usize;
}
