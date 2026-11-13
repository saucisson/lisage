//! Non-local events (M2-5, spec § 7, statement T3): compilation of a
//! transition of width `w` into a transduction with **level memory**, with
//! a polynomial overhead in `w`.
//!
//! Reference example (spec § 7): the transfer `t : m(a) -= 1; m(b) += 1`
//! over a `Map` — the transition touches the levels of the two keys `a` and
//! `b`, not-adjacent. The semantics is not-local (read `a`, write `b`),
//! but each *segment* is local: the inherited attribute `H` carries the
//! transfer token from `val(a)` to `val(b)` through the intermediate
//! levels, and the level memory `M` records at the emitting node
//! that the decrement has already been produced (idempotence by `(node, level)`).
//!
//! **Per-edge semantics** (spec § 7, `φ_disp(val(a), h, v) = v ≥ 1 ?
//! h_transfer : stop`): the source is decremented as soon as `m(a) ≥ 1`;
//! the destination receives `m(b) + 1` if this value is in the domain,
//! otherwise its edge is kept (the source decrement still applies —
//! case pinned by `test_transfer_destination_out_domain`).
//!
//! **Poly(w) overhead (T3)**: the transduction visits each node once
//! (memo L1) and does O(1) work per node; `H` is an O(1) state per
//! level and `M` stores at most one entry per touched `(node, level)`,
//! i.e. O(w) entries per node. The compilation does not reify any
//! intermediate variable (unlike Zhao/Ciardo): `M` and `H` stitch the
//! segments together.
//!
//! **Scope**: the destination must exist in the diagram (key
//! present, or presence node kept by a later key) — the
//! "absent and collapsed destination" case falls under axis
//! explicitation (T-EXPAND, § lookup), out of this milestone.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::fold::union;
use crate::memory::{EffectMemory, MemoryLevel};
use crate::schema::{ABSENT, Domain, PRESENT, Schema, SchemaKeys};
use crate::transduction::{ContextPhiOut, Transduction};
use lisage_lang::value::Atom;

/// Base fingerprint of the transfer transduction.
const FINGERPRINT_TRANSFERT: u64 = 5001;
/// Number of levels touched by a transfer between two keys: presence +
/// value of the source, presence + value of the destination.
pub const WIDTH_TRANSFERT: u32 = 4;

/// Inherited attribute of the transfer: state of the token between the levels.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum StateTransfer {
    /// Normal descent: no transfer in progress.
    Normal,
    /// Transfer in progress: amount to add to the value of the
    /// destination key (1 unit transferred).
    Transfer(u64),
}

/// Non-local transition `m(a) -= 1; m(b) += 1` compiled into a transduction with
/// level memory (T3). Requires `index_source < index_destination`: the
/// inherited token descends from `val(a)` to `val(b)`.
#[derive(Clone)]
pub struct TransferNotLocal {
    index_source: usize,
    index_destination: usize,
    domain: Domain,
    keys: SchemaKeys,
}

impl TransferNotLocal {
    /// Transfer from key `index_source` to key `index_destination`
    /// (in the key order of the `Map`) of the given schema.
    ///
    /// # Panics
    /// If the schema is not a `Map`, or if `index_source >=
    /// index_destination` (the inherited token does not go up).
    pub fn new(schema: &Schema, index_source: usize, index_destination: usize) -> Self {
        assert!(
            index_source < index_destination,
            "transfer requires source < destination (descending inherited token)"
        );
        let Schema::Map(_, domain) = schema else {
            panic!("TransferNotLocal requires a Map schema");
        };
        TransferNotLocal {
            index_source,
            index_destination,
            domain: domain.clone(),
            keys: SchemaKeys::new(schema.clone()),
        }
    }

    /// Width `w` of the transition: levels touched (T3, poly(w) overhead).
    pub fn width(&self) -> u32 {
        WIDTH_TRANSFERT
    }

    /// Value level of the source key.
    fn level_source(&self) -> Level {
        Schema::level_value(self.index_source)
    }

    /// Value level of the destination key.
    fn level_destination(&self) -> Level {
        Schema::level_value(self.index_destination)
    }

    /// Inserts the remapped edge `key ↦ (n, s)` into `remap`; in case of a
    /// key collision (family: two remapped edges towards the same target), the two
    /// subtrees are **unioned**, never overwritten.
    fn insert_remap<P: Pool<Key = u64, Payload = bool>>(
        pool: &mut P,
        keys: &SchemaKeys,
        anchor: Anchor,
        remap: &mut BTreeMap<u64, (NodeId, ())>,
        key: u64,
        n: NodeId,
        s: (),
    ) {
        match remap.remove(&key) {
            Some((previous, _)) => {
                let fusion = union(pool, keys, anchor, previous, n)
                    .expect("union of sub-edges of the same pool");
                remap.insert(key, (fusion, s));
            }
            None => {
                remap.insert(key, (n, s));
            }
        }
    }

    /// Decrement of the source (level `val(a)`): remap `v → v-1` if
    /// `v ≥ 1` (spec § 7: `φ_disp(val(a), h, v) = v ≥ 1 ? h_transfer : stop`),
    /// and memory effect `M[id(n), val(a)] ← true` if a decrement has been
    /// produced (idempotence by `(node, level)`).
    fn decrement_source<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        memory: &MemoryLevel<bool>,
        node_input: NodeId,
        level: Level,
        edges: BTreeMap<u64, (NodeId, ())>,
    ) -> (BTreeMap<u64, (NodeId, ())>, EffectMemory<bool>) {
        // Already produced by this node at this visit? (M idempotence).
        if memory.read(node_input, level) == Some(&true) {
            return (edges, EffectMemory::new());
        }
        let mut remap = BTreeMap::new();
        let mut decrement = false;
        for (v, (n, s)) in edges {
            if v >= 1 {
                Self::insert_remap(pool, &self.keys, anchor, &mut remap, v - 1, n, s);
                decrement = true;
            } else {
                Self::insert_remap(pool, &self.keys, anchor, &mut remap, v, n, s);
            }
        }
        let effect = if decrement {
            EffectMemory::one(node_input, level, true)
        } else {
            EffectMemory::new()
        };
        (remap, effect)
    }

    /// Placed of the **absent** destination (level `k_b?`): the `ABSENT`
    /// edge becomes `PRESENT` towards a new value node carrying the
    /// transferred amount (spec § 7: `φ_disp(k_b?, h_transfer(w), ·) = h_placed(w+1)`).
    fn placed_presence_destination<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        h: &StateTransfer,
        mut edges: BTreeMap<u64, (NodeId, ())>,
    ) -> BTreeMap<u64, (NodeId, ())> {
        let StateTransfer::Transfer(montant) = h else {
            return edges;
        };
        let montant = *montant;
        if self.domain.contains(Atom(montant))
            && let Some((suite, _)) = edges.remove(&ABSENT)
        {
            let node_value = pool.intern(
                anchor,
                Form::Node {
                    level: self.level_destination(),
                    edges: BTreeMap::from([(montant, suite)]),
                },
            );
            // Collision with an existing PRESENT edge (family): union.
            match edges.remove(&PRESENT) {
                Some((previous, _)) => {
                    let fusion = union(pool, &self.keys, anchor, previous, node_value)
                        .expect("union of sub-edges of the same pool");
                    edges.insert(PRESENT, (fusion, ()));
                }
                None => {
                    edges.insert(PRESENT, (node_value, ()));
                }
            }
        }
        edges
    }

    /// Placed on the value of the destination (level `val(b)`): remap
    /// `u → u + amount` if the target is in the domain; otherwise the edge is
    /// kept as is (**per-edge** semantics: the source decrement
    /// still applies, pinned by `test_transfer_destination_out_domain`).
    fn placed_value_destination<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        h: &StateTransfer,
        edges: BTreeMap<u64, (NodeId, ())>,
    ) -> BTreeMap<u64, (NodeId, ())> {
        let StateTransfer::Transfer(montant) = h else {
            return edges;
        };
        let montant = *montant;
        let mut remap = BTreeMap::new();
        for (u, (n, s)) in edges {
            let target = u + montant;
            if self.domain.contains(Atom(target)) {
                Self::insert_remap(pool, &self.keys, anchor, &mut remap, target, n, s);
            } else {
                Self::insert_remap(pool, &self.keys, anchor, &mut remap, u, n, s);
            }
        }
        remap
    }
}

impl Transduction for TransferNotLocal {
    type H = StateTransfer;
    type S = ();
    type Out = bool;
    type M = bool;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_TRANSFERT ^ (self.index_source as u64) << 32 ^ self.index_destination as u64
    }

    fn h0(&self) -> StateTransfer {
        StateTransfer::Normal
    }

    fn phi_dispatch(&self, level: Level, h: &StateTransfer, edge: u64) -> StateTransfer {
        if level == self.level_source() {
            // φ_disp(val(a), h, v) = v ≥ 1 ? h_transfer(1) : stop :
            // the token is born at the value of a (1 unit to transfer).
            if edge >= 1 {
                StateTransfer::Transfer(1)
            } else {
                StateTransfer::Normal
            }
        } else {
            // The token crosses the intermediate levels unchanged.
            h.clone()
        }
    }

    fn phi_aggregate(
        &self,
        _level: Level,
        _h: &StateTransfer,
        _edges: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<bool>) {
        let ContextPhiOut {
            anchor,
            level,
            h,
            edges,
            memory,
            node_input,
        } = ctx;
        let (edges, effect) = if level == self.level_source() {
            self.decrement_source(pool, anchor, memory, node_input, level, edges)
        } else if level == Schema::level_presence(self.index_destination)
            && matches!(h, StateTransfer::Transfer(_))
        {
            (
                self.placed_presence_destination(pool, anchor, h, edges),
                EffectMemory::new(),
            )
        } else if level == self.level_destination() && matches!(h, StateTransfer::Transfer(_)) {
            (
                self.placed_value_destination(pool, anchor, h, edges),
                EffectMemory::new(),
            )
        } else {
            (edges, EffectMemory::new())
        };
        let edges = edges.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (pool.intern(anchor, Form::Node { level, edges }), effect)
    }

    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &StateTransfer,
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}
