//! Not-local events (M2-5, spec § 7, statement T3): compiling a transition of
//! width `w` into a transduction with **level memory**, with
//! a polynomial overhead in `w`.
//!
//! Reference example (spec § 7): the transfer `t : m(a) -= 1; m(b) += 1`
//! on a `Map` — the transition touches the levels of the two keys `a` and
//! `b`, not-adjacent. The semantics is not-local (read `a`, write `b`),
//! but each *segment* is local: the inherited attribute `H` carries the
//! transfer token from `val(a)` to `val(b)` through the
//! intermediate levels, and the level memory `M` records at the emitting node
//! that the decrement has already been produced (idempotence by `(node, level)`).
//!
//! **Per-arc semantics** (spec § 7, `φ_disp(val(a), h, v) = v ≥ 1 ?
//! h_transfer : stop`): the source is decremented as soon as `m(a) ≥ 1`;
//! the destination receives `m(b) + 1` if this value is in the domain,
//! otherwise its arc is kept (the source decrement still applies —
//! case pinned by `test_transfer_destination_outside_domain`).
//!
//! **Poly(w) overhead (T3)**: the transduction visits each node once
//! (memo L1) and does O(1) work per node; `H` is an O(1) state per
//! level and `M` stores at most one entry per touched `(node, level)`,
//! i.e. O(w) entries per node. The compilation does not reify any
//! intermediate variable (unlike Zhao/Ciardo): `M` and `H` stitch the
//! segments.
//!
//! **Scope**: the destination must exist in the diagram (key
//! present, or presence node kept by a later key) — the
//! "absent and collapsed destination" case falls under axis
//! explicitation (T-EXPAND, § lookup), outside this milestone.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, NodeId, Pool};

use crate::fold::union;
use crate::memory::EffectMemory;
use crate::observer::{TraceVal, Traceable};
use crate::schema::{ABSENT, Domain, PRESENT, Schema, SchemaKeys};
use crate::transduction::{MemoryProbe, PhiOutContext, Transduction};
use lisage_lang::value::Atom;

/// Base fingerprint of the transfer transduction.
const FINGERPRINT_TRANSFER: u64 = 5001;
/// Number of levels touched by a transfer between two keys: presence +
/// value of the source, presence + value of the destination.
pub const WIDTH_TRANSFER: u32 = 4;

/// Inherited attribute of the transfer: state of the token between levels.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum StateTransfer {
    /// Normal descent: no transfer in progress.
    Normal,
    /// Transfer in progress: amount to add to the value of the
    /// destination key (1 unit transferred).
    Transfer(u64),
}

impl Traceable for StateTransfer {
    fn trace_val(&self) -> TraceVal {
        match self {
            StateTransfer::Normal => TraceVal::text("Normal"),
            StateTransfer::Transfer(amount) => TraceVal::map(vec![(
                TraceVal::text("Transfert"),
                TraceVal::integer(*amount as i64),
            )]),
        }
    }
}

/// Not-local transition `m(a) -= 1; m(b) += 1` compiled into a level-memory
/// transduction (T3). Requires `index_source < index_destination`: the
/// inherited token descends from `val(a)` to `val(b)`.
#[derive(Clone)]
pub struct TransferNonLocal {
    index_source: usize,
    index_destination: usize,
    domain: Domain,
    keys: SchemaKeys,
}

impl TransferNonLocal {
    /// Transfer from key `index_source` to key `index_destination`
    /// (in the key order of the `Map`) of the given schema.
    ///
    /// # Panics
    /// If the schema is not a `Map`, or if `index_source >=
    /// index_destination` (the inherited token does not go upward).
    pub fn new(schema: &Schema, index_source: usize, index_destination: usize) -> Self {
        assert!(
            index_source < index_destination,
            "transfer requires source < destination (downward inherited token)"
        );
        let Schema::Map(_, domain) = schema else {
            panic!("TransfertNonLocal requires a Map schema");
        };
        TransferNonLocal {
            index_source,
            index_destination,
            domain: domain.clone(),
            keys: SchemaKeys::new(schema.clone()),
        }
    }

    /// Width `w` of the transition: touched levels (T3, poly(w) overhead).
    pub fn width(&self) -> u32 {
        WIDTH_TRANSFER
    }

    /// Value level of the source key.
    fn level_source(&self) -> Level {
        Schema::level_value(self.index_source)
    }

    /// Value level of the destination key.
    fn level_destination(&self) -> Level {
        Schema::level_value(self.index_destination)
    }

    /// Inserts the remapped arc `key ↦ (n, s)` into `remap`; on key collision
    /// (family: two arcs remapped to the same target), the two
    /// subtrees are **unioned**, never overwritten.
    fn inserer_remap<P: Pool<Key = u64, Payload = bool>>(
        pool: &mut P,
        keys: &SchemaKeys,
        anchor: Anchor,
        remap: &mut BTreeMap<u64, (NodeId, ())>,
        key: u64,
        n: NodeId,
        s: (),
    ) {
        match remap.remove(&key) {
            Some((old, _)) => {
                let merge =
                    union(pool, keys, anchor, old, n).expect("union of sub-arcs of the same pool");
                remap.insert(key, (merge, s));
            }
            None => {
                remap.insert(key, (n, s));
            }
        }
    }

    /// Decrement of the source (level `val(a)`): remap `v → v-1` if
    /// `v ≥ 1` (spec § 7: `φ_disp(val(a), h, v) = v ≥ 1 ? h_transfer : stop`),
    /// and memory effect `M[id(n), val(a)] ← true` if a decrement was
    /// produced (idempotence by `(node, level)`).
    fn decrement_source<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        memory: &MemoryProbe<'_, bool>,
        node_input: NodeId,
        level: Level,
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> (BTreeMap<u64, (NodeId, ())>, EffectMemory<bool>) {
        // Already produced by this node at this visit? (idempotence M).
        if memory.read(node_input, level) == Some(&true) {
            return (arcs, EffectMemory::new());
        }
        let mut remap = BTreeMap::new();
        let mut decrement = false;
        for (v, (n, s)) in arcs {
            if v >= 1 {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, v - 1, n, s);
                decrement = true;
            } else {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, v, n, s);
            }
        }
        let effect = if decrement {
            EffectMemory::one(node_input, level, true)
        } else {
            EffectMemory::new()
        };
        (remap, effect)
    }

    /// Placement of the **absent** destination (level `k_b?`): the `ABSENT`
    /// arc becomes `PRESENT` towards a new value node carrying the
    /// transferred amount (spec § 7: `φ_disp(k_b?, h_transfer(w), ·) = h_pose(w+1)`).
    fn pose_presence_destination<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        h: &StateTransfer,
        mut arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> BTreeMap<u64, (NodeId, ())> {
        let StateTransfer::Transfer(amount) = h else {
            return arcs;
        };
        let amount = *amount;
        if self.domain.contains(Atom(amount))
            && let Some((suite, _)) = arcs.remove(&ABSENT)
        {
            let node_value = pool.intern(
                anchor,
                Form::Node {
                    level: self.level_destination(),
                    arcs: ArcRun::from_pairs([(amount, suite)]),
                },
            );
            // Collision with an existing PRESENT arc (family): union.
            match arcs.remove(&PRESENT) {
                Some((old, _)) => {
                    let merge = union(pool, &self.keys, anchor, old, node_value)
                        .expect("union of sub-arcs of the same pool");
                    arcs.insert(PRESENT, (merge, ()));
                }
                None => {
                    arcs.insert(PRESENT, (node_value, ()));
                }
            }
        }
        arcs
    }

    /// Placement on the value of the destination (level `val(b)`): remap
    /// `u → u + amount` if the target is in the domain; otherwise the arc is
    /// kept as is (**per-arc** semantics: the source decrement
    /// still applies, pinned by `test_transfer_destination_outside_domain`).
    fn pose_value_destination<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        h: &StateTransfer,
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> BTreeMap<u64, (NodeId, ())> {
        let StateTransfer::Transfer(amount) = h else {
            return arcs;
        };
        let amount = *amount;
        let mut remap = BTreeMap::new();
        for (u, (n, s)) in arcs {
            let target = u + amount;
            if self.domain.contains(Atom(target)) {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, target, n, s);
            } else {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, u, n, s);
            }
        }
        remap
    }
}

impl Transduction for TransferNonLocal {
    type H = StateTransfer;
    type S = ();
    type Out = bool;
    type M = bool;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_TRANSFER ^ (self.index_source as u64) << 32 ^ self.index_destination as u64
    }

    fn h0(&self) -> StateTransfer {
        StateTransfer::Normal
    }

    fn phi_disp(&self, level: Level, h: &StateTransfer, arc: u64) -> StateTransfer {
        if level == self.level_source() {
            // φ_disp(val(a), h, v) = v ≥ 1 ? h_transfer(1) : stop:
            // the token is born at the value of a (1 unit to transfer).
            if arc >= 1 {
                StateTransfer::Transfer(1)
            } else {
                StateTransfer::Normal
            }
        } else {
            // The token crosses the intermediate levels unchanged.
            h.clone()
        }
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &StateTransfer,
        _arcs: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<bool>) {
        let PhiOutContext {
            anchor,
            level,
            h,
            arcs,
            memory,
            node_input,
        } = ctx;
        let (arcs, effect) = if level == self.level_source() {
            self.decrement_source(pool, anchor, &memory, node_input, level, arcs)
        } else if level == Schema::level_presence(self.index_destination)
            && matches!(h, StateTransfer::Transfer(_))
        {
            (
                self.pose_presence_destination(pool, anchor, h, arcs),
                EffectMemory::new(),
            )
        } else if level == self.level_destination() && matches!(h, StateTransfer::Transfer(_)) {
            (
                self.pose_value_destination(pool, anchor, h, arcs),
                EffectMemory::new(),
            )
        } else {
            (arcs, EffectMemory::new())
        };
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (pool.intern(anchor, Form::Node { level, arcs }), effect)
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
