//! Saturation benchmark (M2-4) on the reference philosophers controller
//! (golden thread), flat `Map` schema with two levels per key (even
//! presence + odd value, domain {think, hungry, eating}).
//!
//! Four configurations reaching the same fixpoint |R_n|:
//! - `naive`  : `saturate` `Static`(all value levels), BFS rounds;
//! - `hamez`  : `saturate` `AutoDetected`, blind level deactivation;
//! - `ciardo` : manual sweeps i = n-1..0 of `saturate` `Static`([level
//!   i]), until a sterile sweep;
//! - `inplace`: `saturate_in_place`, local depth-first fires (the
//!   in-place kernel of `inplace`; the recursion order replaces the
//!   policy).
//!
//! The step transduction `DiningStep` fires each event exactly once per
//! word: hungry (think -> hungry, unguarded), drop (eating -> think,
//! unguarded), take (hungry -> eating, guarded by both neighbors
//! differing from eating). The left-neighbor guard comes from the
//! inherited channel (`RingState.prev`), the right-neighbor guard from
//! the filtered continuation (`NoEats` / `guarded_continuation`), with
//! shape-aware rebuilds when the continuation collapsed (R-collapse: a
//! skipped value level hides implicit eating words).
//!
//! Checks: count(fixpoint) == explicit enumeration (BFS oracle), and
//! for n <= 7 the golden-thread pins 20/56/152/416/1136.
//!
//! `LIS_SAT_PROBE=1`: probe the `AutoDetected` loop round by round
//! (active set, modified levels, re-traversed diagram) instead of the
//! benchmark.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, ArcRun, Form, Level, MemoryPolicy, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::inplace::saturate_in_place;
use lisage_core::memory::EffectMemory;
use lisage_core::observer::{NoOpObserver, TraceVal, Traceable};
use lisage_core::saturate::{PolicySaturation, RestrictedToLevels, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{PhiOutContext, Transducer, Transduction};
use lisage_lang::value::{Atom, Value};

/// Root anchor.
const ANCHOR: Anchor = Anchor(0);
/// Program-counter values: think (0), hungry (1), eating (2).
const THINK: u64 = 0;
const HUNGRY: u64 = 1;
const EATS: u64 = 2;
/// Sentinel "pc of key 0 unknown" of the inherited channel.
const PROGRAM_COUNTER_ZERO_UNKNOWN: u64 = 3;
/// Arc key of a presence level: absent (0, default), present (1).
const PRESENT: u64 = 1;
/// Fingerprint of the step transduction.
const SEL_STEP: u64 = 0xBACC_ADE0_0000_0000;
/// Fingerprint of the `NoEats` filter (neighbor fragment).
const SEL_NO_EATS: u64 = 0x51A7_E5E5_0000_0000;
/// Golden-thread pins: |R_n| for n = 3..7.
const PINS: [u64; 5] = [20, 56, 152, 416, 1136];
/// Repetitions per (n, configuration) for the median.
const REPETITIONS: usize = 5;
/// Benchmark range: n from 3 to 13.
const N_MAX: u32 = 13;
/// Beyond that, the explicit oracle (BFS over 3^n words) gets too big.
const N_MAX_ORACLE: u32 = 10;

type PoolRef = Arena<ReferenceController>;

// ---------------------------------------------------------------------------
// Inherited channel: pc of key 0 (seam guard) + previously crossed value.
// ---------------------------------------------------------------------------

/// Ring inherited attribute: `program_counter_zero` is the value of key 0 (needed by the
/// seam guard at key n-1), `prev` is the pc value of the key crossed
/// just before (left-neighbor guard).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RingState {
    program_counter_zero: u64,
    prev: u64,
}

impl Traceable for RingState {
    fn trace_val(&self) -> TraceVal {
        TraceVal::Map(vec![
            (
                TraceVal::Integer(self.program_counter_zero as i64),
                TraceVal::Elided,
            ),
            (TraceVal::Integer(self.prev as i64), TraceVal::Elided),
        ])
    }
}

// ---------------------------------------------------------------------------
// Small node builders (shape-aware rebuilds).
// ---------------------------------------------------------------------------

/// Value node "without eating": {think -> tail, hungry -> tail}
/// (two arcs out of three: never collapsed again, it reintroduces the
/// structural pc distinction at the filtered level).
fn value_without_eats<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    value_level: u32,
    tail: NodeId,
) -> NodeId {
    pool.intern(
        anchor,
        Form::Node {
            level: Level(value_level),
            arcs: ArcRun::from_pairs(vec![(THINK, tail), (HUNGRY, tail)]),
        },
    )
}

/// Presence prefix {present -> value_without_eats(tail)} at
/// `presence_level`: the filter of the next value level is reintroduced
/// above a collapsed continuation (a jump or a true terminal).
fn guarded_prefix<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    presence_level: u32,
    tail: NodeId,
) -> NodeId {
    let value = value_without_eats(pool, anchor, presence_level + 1, tail);
    pool.intern(
        anchor,
        Form::Node {
            level: Level(presence_level),
            arcs: ArcRun::from_pairs(vec![(PRESENT, value)]),
        },
    )
}

/// Merges one output branch (idempotent union in case of collision).
fn merge_arc<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    outputs: &mut BTreeMap<u64, NodeId>,
    arc: u64,
    node: NodeId,
) {
    match outputs.get(&arc) {
        Some(&previous) => {
            let joined = union(pool, keys, anchor, previous, node).expect("union");
            outputs.insert(arc, joined);
        }
        None => {
            outputs.insert(arc, node);
        }
    }
}

// ---------------------------------------------------------------------------
// Right-neighbor guard, continuation-aware.
// ---------------------------------------------------------------------------

/// Outcome of filtering one present-branch child.
enum FilteredBranch {
    /// Keep the arc with this (possibly rewritten) child.
    Keep(NodeId),
    /// Drop the arc entirely (only-eating branch: the word dies).
    Drop,
}

/// Filters one value-level child of a present arc: removes the words
/// with pc = eating at the next value level.
fn filter_value_child<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    presence_level: u32,
    child: NodeId,
) -> FilteredBranch {
    match pool.resolve(child).expect("resolve value node") {
        Form::Terminal(false) => FilteredBranch::Keep(child),
        Form::Terminal(true) => {
            let filtered = value_without_eats(pool, anchor, presence_level + 1, child);
            FilteredBranch::Keep(filtered)
        }
        Form::Node {
            level: child_level,
            arcs: value_arcs,
        } if child_level.0 == presence_level + 1 => {
            if value_arcs.iter().any(|(value_key, _)| *value_key == EATS) {
                let kept: Vec<(u64, NodeId)> = value_arcs
                    .iter()
                    .filter(|(value_key, _)| **value_key != EATS)
                    .map(|(value_key, value_target)| (*value_key, *value_target))
                    .collect();
                if kept.is_empty() {
                    return FilteredBranch::Drop;
                }
                let filtered = pool.intern(
                    anchor,
                    Form::Node {
                        level: child_level,
                        arcs: ArcRun::from_pairs(kept),
                    },
                );
                FilteredBranch::Keep(filtered)
            } else {
                FilteredBranch::Keep(child)
            }
        }
        Form::Node {
            level: child_level, ..
        } if child_level.0 > presence_level + 1 => {
            let filtered = value_without_eats(pool, anchor, presence_level + 1, child);
            FilteredBranch::Keep(filtered)
        }
        Form::Node { .. } => FilteredBranch::Keep(child),
    }
}

/// Continuation of the hungry branch under a take at key i
/// (1 <= i <= n-2), filtered of its words with pc_{i+1} = eating.
/// `target` is the raw subdiagram (continuation of the word), expected
/// at the presence level of key i+1.
fn guarded_continuation<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    _keys: &SchemaKeys,
    anchor: Anchor,
    presence_level: u32,
    target: NodeId,
) -> Option<NodeId> {
    match pool.resolve(target).expect("resolve continuation") {
        Form::Terminal(false) => None,
        Form::Terminal(true) => Some(guarded_prefix(pool, anchor, presence_level, target)),
        Form::Node { level: l, arcs } if l.0 == presence_level => {
            let mut branches: Vec<(u64, NodeId)> = Vec::new();
            let mut changed = false;
            for (arc, child) in arcs.iter() {
                let (arc, child) = (*arc, *child);
                if arc != PRESENT {
                    branches.push((arc, child));
                    continue;
                }
                match filter_value_child(pool, anchor, presence_level, child) {
                    FilteredBranch::Keep(filtered) => {
                        changed = changed || filtered != child;
                        branches.push((arc, filtered));
                    }
                    FilteredBranch::Drop => {
                        changed = true;
                    }
                }
            }
            if branches.is_empty() {
                return None;
            }
            if !changed {
                return Some(target);
            }
            Some(pool.intern(
                anchor,
                Form::Node {
                    level: l,
                    arcs: ArcRun::from_pairs(branches),
                },
            ))
        }
        // Jump past the expected presence level: pc_{i+1} is skipped
        // (implicit eating included), rebuild of the filter above it.
        Form::Node { .. } => Some(guarded_prefix(pool, anchor, presence_level, target)),
    }
}

// ---------------------------------------------------------------------------
// NoEats: filter of the diagram at the given value levels.
// ---------------------------------------------------------------------------

/// Filter transduction: drops the words whose target key evaluates to
/// eating, at the listed value levels. Shape-aware: when the filtered
/// value node was collapsed (skipped pc), the filter is reintroduced
/// at the parent presence level
/// ({present -> {think, hungry} -> tail}).
#[derive(Clone)]
struct NoEats {
    /// Filtered value levels (odd).
    levels: BTreeSet<u32>,
}

impl NoEats {
    fn new(n: u32) -> Self {
        // Neighbors of key 0: keys 1 and n-1 (value levels 3 and 2n-1).
        NoEats {
            levels: BTreeSet::from([3u32, 2 * n - 1]),
        }
    }
}

impl Transduction for NoEats {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        self.levels
            .iter()
            .fold(SEL_NO_EATS, |acc, l| acc.rotate_left(7) ^ (*l as u64))
    }

    fn h0(&self) {}

    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}

    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}

    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<Self::M>) {
        let level = ctx.level.0;
        let transduced: Vec<(u64, NodeId)> = ctx.arcs.iter().map(|(v, (n, _))| (*v, *n)).collect();
        // (A2) every child dead: the node is dead.
        if !transduced.is_empty()
            && transduced
                .iter()
                .all(|(_, n)| matches!(pool.resolve(*n), Ok(Form::Terminal(false))))
        {
            return (
                pool.intern(ctx.anchor, Form::Terminal(false)),
                EffectMemory::new(),
            );
        }
        if level % 2 == 0 {
            // Presence level: rebuild when the next value level is
            // filtered and the transduced child was collapsed.
            let value_level = level + 1;
            if self.levels.contains(&value_level)
                && let Some(&child) = transduced
                    .iter()
                    .find(|(v, _)| *v == PRESENT)
                    .map(|(_, n)| n)
            {
                let rebuild = match pool.resolve(child).expect("value node") {
                    Form::Terminal(true) => Some(child),
                    Form::Node {
                        level: child_level, ..
                    } if child_level.0 > value_level => Some(child),
                    _ => None,
                };
                if let Some(tail) = rebuild {
                    let filtered = guarded_prefix(pool, ctx.anchor, level, tail);
                    return (filtered, EffectMemory::new());
                }
            }
        } else if self.levels.contains(&level) {
            // Filtered value level: drop the eating arc.
            let kept: Vec<(u64, NodeId)> = transduced
                .iter()
                .filter(|(v, _)| *v != EATS)
                .map(|(v, n)| (*v, *n))
                .collect();
            if kept.is_empty() {
                return (
                    pool.intern(ctx.anchor, Form::Terminal(false)),
                    EffectMemory::new(),
                );
            }
            return (
                pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: Level(level),
                        arcs: ArcRun::from_pairs(kept),
                    },
                ),
                EffectMemory::new(),
            );
        }
        // Identity (restructure).
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: Level(level),
                    arcs: ArcRun::from_pairs(transduced),
                },
            ),
            EffectMemory::new(),
        )
    }

    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

// ---------------------------------------------------------------------------
// Root fragment of the take at key 0.
// ---------------------------------------------------------------------------

/// Guard of the two neighbors of key 0 (keys 1 and n-1) applied to the
/// raw continuation `child`. Dispatch on the shape of `child`.
fn take_zero_fragment<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    n: u32,
    child: NodeId,
) -> Option<NodeId> {
    match pool.resolve(child).expect("resolve fragment") {
        Form::Terminal(false) => None,
        Form::Terminal(true) => Some(free_chain_filtered(pool, anchor, n)),
        Form::Node { level, .. } if level.0 <= 3 => run_no_eats(pool, anchor, n, child),
        Form::Node { .. } => {
            // Jump: pc_1 is skipped in the continuation; filter key n-1
            // deep first, then reintroduce the key-1 filter.
            let filtered = run_no_eats(pool, anchor, n, child)?;
            let value = value_without_eats(pool, anchor, 3, filtered);
            Some(pool.intern(
                anchor,
                Form::Node {
                    level: Level(2),
                    arcs: ArcRun::from_pairs(vec![(PRESENT, value)]),
                },
            ))
        }
    }
}

/// Runs a fresh `NoEats` over `child` (throwaway runner, no memory).
fn run_no_eats<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    n: u32,
    child: NodeId,
) -> Option<NodeId> {
    let mut runner = Transducer::with_policy(NoEats::new(n), MemoryPolicy::NoMemory);
    let (fragment, _) = runner
        .run_same_pool(pool, anchor, child)
        .expect("NoEats sub-transduction");
    if matches!(pool.resolve(fragment), Ok(Form::Terminal(false))) {
        None
    } else {
        Some(fragment)
    }
}

/// Chain rebuilt for a fully free continuation (a true terminal): keys 1
/// and n-1 filtered to {think, hungry}, the rest free by jump (hence
/// naturally collapsed).
fn free_chain_filtered<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    n: u32,
) -> NodeId {
    let terminal = pool.intern(anchor, Form::Terminal(true));
    // Key n-1: presence 2(n-1), value 2n-1.
    let bottom = guarded_prefix(pool, anchor, 2 * (n - 1), terminal);
    // Key 1: presence 2, value 3, tail = the key n-1 branch.
    guarded_prefix(pool, anchor, 2, bottom)
}

// ---------------------------------------------------------------------------
// Step transduction: one fire of every event per word.
// ---------------------------------------------------------------------------

/// Single-event dining-philosophers step: `τ(R) = ∪_j ev_j(R)`. Events
/// fire at the value levels (odd): hungry, drop, take.
#[derive(Clone)]
struct DiningStep {
    /// Number of philosophers (keys).
    n: u32,
    /// Schema key algebra (for the output-branch unions).
    keys: Arc<SchemaKeys>,
}

impl Transduction for DiningStep {
    type H = RingState;
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        SEL_STEP ^ (self.n as u64)
    }

    fn h0(&self) -> RingState {
        RingState {
            program_counter_zero: PROGRAM_COUNTER_ZERO_UNKNOWN,
            prev: PROGRAM_COUNTER_ZERO_UNKNOWN,
        }
    }

    /// (T-NODE) dispatch: leaving an odd (value) level, the arc IS the
    /// pc value of the crossed key; `prev` records it, `program_counter_zero` is pinned
    /// at level 1.
    fn phi_disp(&self, level: Level, h: &RingState, arc: u64) -> RingState {
        if level.0 % 2 == 1 {
            RingState {
                program_counter_zero: if level.0 == 1 {
                    arc
                } else {
                    h.program_counter_zero
                },
                prev: arc,
            }
        } else {
            h.clone()
        }
    }

    fn phi_aggr(&self, _level: Level, _h: &RingState, _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}

    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<Self::M>) {
        let level = ctx.level.0;
        if level % 2 == 0 {
            self.phi_out_presence(pool, ctx, level)
        } else {
            self.phi_out_value(pool, ctx, level)
        }
    }

    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &RingState,
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

impl DiningStep {
    /// Presence level: identity (including A2), never fires.
    fn phi_out_presence<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<
            '_,
            RingState,
            <DiningStep as Transduction>::S,
            <DiningStep as Transduction>::M,
        >,
        level: u32,
    ) -> (NodeId, EffectMemory<<DiningStep as Transduction>::M>) {
        let transduced: BTreeMap<u64, NodeId> =
            ctx.arcs.iter().map(|(v, (n, _))| (*v, *n)).collect();
        if !transduced.is_empty()
            && transduced
                .values()
                .all(|n| matches!(pool.resolve(*n), Ok(Form::Terminal(false))))
        {
            return (
                pool.intern(ctx.anchor, Form::Terminal(false)),
                EffectMemory::new(),
            );
        }
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: Level(level),
                    arcs: ArcRun::from_pairs(transduced),
                },
            ),
            EffectMemory::new(),
        )
    }

    /// Value level (key i): fire the events.
    fn phi_out_value<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<
            '_,
            RingState,
            <DiningStep as Transduction>::S,
            <DiningStep as Transduction>::M,
        >,
        level: u32,
    ) -> (NodeId, EffectMemory<<DiningStep as Transduction>::M>) {
        let i = (level - 1) / 2;
        let raw_arcs: Vec<(u64, NodeId)> = match pool.resolve(ctx.node_input).expect("input node") {
            Form::Node { arcs, .. } => arcs.iter().map(|(v, c)| (*v, *c)).collect(),
            Form::Terminal(_) => Vec::new(),
        };
        let mut outputs: BTreeMap<u64, NodeId> =
            ctx.arcs.iter().map(|(v, (n, _))| (*v, *n)).collect();
        for (value, child) in raw_arcs {
            match value {
                THINK => {
                    // Hungry: think -> hungry, unguarded.
                    merge_arc(pool, &self.keys, ctx.anchor, &mut outputs, HUNGRY, child);
                }
                EATS => {
                    // Drop: eating -> think, unguarded.
                    merge_arc(pool, &self.keys, ctx.anchor, &mut outputs, THINK, child);
                }
                HUNGRY => {
                    self.fire_take(pool, ctx.anchor, i, ctx.h, child, &mut outputs);
                }
                _ => {}
            }
        }
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: Level(level),
                    arcs: ArcRun::from_pairs(outputs),
                },
            ),
            EffectMemory::new(),
        )
    }

    /// Take event: hungry -> eating, guarded by both neighbors.
    fn fire_take<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        i: u32,
        state: &RingState,
        child: NodeId,
        outputs: &mut BTreeMap<u64, NodeId>,
    ) {
        if i == 0 {
            if let Some(fragment) = take_zero_fragment(pool, anchor, self.n, child) {
                merge_arc(pool, &self.keys, anchor, outputs, EATS, fragment);
            }
        } else if state.prev != EATS {
            if i + 1 == self.n {
                // Seam: left neighbor through `prev`,
                // right neighbor is program_counter of key 0.
                if state.program_counter_zero != EATS
                    && state.program_counter_zero != PROGRAM_COUNTER_ZERO_UNKNOWN
                {
                    merge_arc(pool, &self.keys, anchor, outputs, EATS, child);
                }
            } else if let Some(kept) =
                guarded_continuation(pool, &self.keys, anchor, 2 * (i + 1), child)
            {
                merge_arc(pool, &self.keys, anchor, outputs, EATS, kept);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

/// Flat pc schema + pool + initial word (everybody thinks).
fn setup(n: u32) -> (Schema, SchemaKeys, PoolRef, NodeId) {
    let schema = Schema::Map(
        (0..n as u64).map(Atom).collect(),
        Domain::new(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let word: BTreeMap<Value, Value> = (0..n)
        .map(|i| (Value::Atom(Atom(i as u64)), Value::Atom(Atom(THINK))))
        .collect();
    let initial =
        compiler(&mut pool, ANCHOR, &schema, &Value::Map(word)).expect("compile the initial word");
    (schema, keys, pool, initial)
}

/// Cardinality of a fixpoint (counting fold, A4/A5).
fn cardinality(pool: &PoolRef, schema: &Schema, node: NodeId) -> u64 {
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    count(pool, &mut output, ANCHOR, schema, node).expect("count fold")
}

/// Naive saturation: `Static`(all value levels); the final sterile
/// round is excluded from the round count.
fn saturate_naive(n: u32, keys: &SchemaKeys, pool: &mut PoolRef, initial: NodeId) -> (NodeId, u32) {
    let active: Vec<Level> = (0..n).map(|i| Level(2 * i + 1)).collect();
    let policy = PolicySaturation::Static(active);
    let step = DiningStep {
        n,
        keys: Arc::new(keys.clone()),
    };
    saturate_with(keys, &policy, &step, pool, initial, 2 * n)
}

/// Hamez saturation: `AutoDetected`.
fn saturate_hamez(n: u32, keys: &SchemaKeys, pool: &mut PoolRef, initial: NodeId) -> (NodeId, u32) {
    let policy = PolicySaturation::AutoDetected;
    let step = DiningStep {
        n,
        keys: Arc::new(keys.clone()),
    };
    saturate_with(keys, &policy, &step, pool, initial, 2 * n)
}

fn saturate_with(
    keys: &SchemaKeys,
    policy: &PolicySaturation,
    step: &DiningStep,
    pool: &mut PoolRef,
    initial: NodeId,
    levels: u32,
) -> (NodeId, u32) {
    let mut params = SaturationParameters {
        transduction: step,
        policy,
        pool,
        keys,
        anchor: ANCHOR,
        initial,
        number_levels: levels,
        observer: None,
    };
    let result = saturate(&mut params).expect("saturation");
    (result.node, result.iterations)
}

/// Ciardo saturation: manual sweeps i = n-1..0 of `saturate`
/// `Static`([value level i]), until a sterile sweep.
fn saturate_ciardo(
    n: u32,
    keys: &SchemaKeys,
    pool: &mut PoolRef,
    initial: NodeId,
) -> (NodeId, u32, u32) {
    let mut current = initial;
    let mut sweeps = 0u32;
    let mut rounds = 0u32;
    loop {
        let mut stable = true;
        for i in (0..n).rev() {
            let policy = PolicySaturation::Static(vec![Level(2 * i + 1)]);
            let step = DiningStep {
                n,
                keys: Arc::new(keys.clone()),
            };
            let (root, iterations) = saturate_with(keys, &policy, &step, pool, current, 2 * n);
            rounds += iterations.saturating_sub(1);
            current = root;
            if iterations > 1 {
                stable = false;
            }
        }
        sweeps += 1;
        if stable {
            return (current, rounds, sweeps);
        }
    }
}

/// In-place saturation: local depth-first fires (`inplace` kernel; the
/// policy is ignored, documented).
fn saturate_inplace(
    n: u32,
    keys: &SchemaKeys,
    pool: &mut PoolRef,
    initial: NodeId,
) -> (NodeId, u32) {
    let step = DiningStep {
        n,
        keys: Arc::new(keys.clone()),
    };
    let policy = PolicySaturation::AutoDetected; // ignored (documented).
    let mut params = SaturationParameters {
        transduction: &step,
        policy: &policy,
        pool,
        keys,
        anchor: ANCHOR,
        initial,
        number_levels: 2 * n,
        observer: None,
    };
    let result = saturate_in_place(&mut params).expect("in-place saturation");
    (result.node, result.iterations)
}

/// Explicit oracle: BFS of the words reachable from all-think.
fn reachable_words(n: u32) -> u64 {
    let m = n as usize;
    let mut visited: BTreeSet<Vec<u64>> = BTreeSet::new();
    let mut worklist: Vec<Vec<u64>> = vec![vec![THINK; m]];
    while let Some(word) = worklist.pop() {
        if !visited.insert(word.clone()) {
            continue;
        }
        for i in 0..m {
            if word[i] == THINK {
                let mut s = word.clone();
                s[i] = HUNGRY;
                worklist.push(s);
            }
            if word[i] == EATS {
                let mut s = word.clone();
                s[i] = THINK;
                worklist.push(s);
            }
            if word[i] == HUNGRY {
                let left = word[(i + m - 1) % m];
                let right = word[(i + 1) % m];
                if left != EATS && right != EATS {
                    let mut s = word.clone();
                    s[i] = EATS;
                    worklist.push(s);
                }
            }
        }
    }
    visited.len() as u64
}

/// One result row.
struct Row {
    n: u32,
    config: &'static str,
    cardinal: u64,
    median_ms: f64,
    rounds: u32,
    sweeps: u32,
    live: usize,
    creations: u64,
}

// ---------------------------------------------------------------------------
// Probe: why AutoDetected degenerates. Replicates the saturate loop in
// Hamez mode, tracking round by round the active set, the really
// modified levels and the re-traversed diagram (the runner — hence its
// memo — is recreated at each round, exactly as in saturate).
// ---------------------------------------------------------------------------

fn probe_hamez(n: u32) {
    let (_schema, keys, mut pool, initial) = setup(n);
    let step = DiningStep {
        n,
        keys: Arc::new(keys.clone()),
    };
    let mut active: Vec<Level> = (0..2 * n).map(Level).collect();
    let mut current = initial;
    let mut number = 0u32;
    let mut traversed_total = 0usize;
    println!(
        "=== AutoDetected probe, n = {n} ({n} keys, {} levels) ===",
        2 * n
    );
    loop {
        number += 1;
        let wrapped = RestrictedToLevels::new(step.clone(), active.clone());
        let mut runner = Transducer::with_policy(wrapped, MemoryPolicy::PerLevel);
        let (image, _) = runner
            .run_same_pool(&mut pool, ANCHOR, current)
            .expect("probe pass");
        let modified = runner.transduction().modified_levels();
        let traversed = runner.size_memo();
        traversed_total += traversed;
        let next = union(&mut pool, &keys, ANCHOR, current, image).expect("probe union");
        let list: Vec<u32> = modified.iter().map(|l| l.0).collect();
        println!(
            "round {number:2}: active {}/{}, modified {:2} {:?}, re-traversed {traversed:5} nodes, pool {:5} nodes",
            active.len(),
            2 * n,
            modified.len(),
            list,
            pool.live()
        );
        if next == current {
            println!(
                "fixpoint: {number} rounds, {traversed_total} nodes re-traversed in total for a pool of {} nodes created",
                pool.live()
            );
            break;
        }
        active.retain(|l| modified.contains(l));
        if active.is_empty() {
            println!("active set emptied at round {number}");
            break;
        }
        current = next;
    }
}

fn main() {
    if std::env::var("LIS_SAT_PROBE").is_ok() {
        for n in [5u32, 8, 10] {
            probe_hamez(n);
            println!();
        }
        return;
    }
    let mut rows: Vec<Row> = Vec::new();
    println!("lisage saturation benchmark: philosophers, flat Map schema, 2n levels");
    println!(
        "configs: naive (Static odd), hamez (AutoDetected), ciardo (single-level Static sweeps), inplace (in-place kernel)"
    );
    for n in 3..=N_MAX {
        let oracle = if n <= N_MAX_ORACLE {
            Some(reachable_words(n))
        } else {
            None
        };
        if n <= 7 {
            let pin = PINS[(n - 3) as usize];
            if let Some(o) = oracle {
                assert_eq!(o, pin, "n = {n}: oracle {o} contradicts pin {pin}");
            }
        }
        let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
        for config in ["naive", "hamez", "ciardo", "inplace"] {
            let mut durations: Vec<f64> = Vec::with_capacity(REPETITIONS);
            let mut last: Option<(u32, u32, usize, u64, u64)> = None;
            for _ in 0..REPETITIONS {
                let (schema, keys, mut pool, initial) = setup(n);
                let t0 = Instant::now();
                let (root, a, b) = match config {
                    "naive" => {
                        let (r, iterations) = saturate_naive(n, &keys, &mut pool, initial);
                        (r, iterations, 0)
                    }
                    "hamez" => {
                        let (r, iterations) = saturate_hamez(n, &keys, &mut pool, initial);
                        (r, iterations, 0)
                    }
                    "ciardo" => {
                        let (r, rounds, sweeps) = saturate_ciardo(n, &keys, &mut pool, initial);
                        (r, rounds, sweeps)
                    }
                    _ => {
                        let (r, fires) = saturate_inplace(n, &keys, &mut pool, initial);
                        (r, fires, 0)
                    }
                };
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                let live = pool.live();
                let creations = pool.metrics().creations;
                let cardinal = cardinality(&pool, &schema, root);
                if let Some(o) = oracle {
                    assert_eq!(
                        cardinal, o,
                        "n = {n}, config {config}: |R| = {cardinal}, expected {o}"
                    );
                }
                if let Some(&c) = counts.get(config) {
                    assert_eq!(cardinal, c, "n = {n}, config {config}: unstable count");
                }
                counts.insert(config, cardinal);
                durations.push(ms);
                last = Some((a, b, live, creations, cardinal));
            }
            let (a, b, live, creations, cardinal) = last.expect("at least one repetition");
            durations.sort_by(|x, y| x.total_cmp(y));
            let median_ms = durations[durations.len() / 2];
            let (rounds, sweeps) = match config {
                "ciardo" => (a, b),
                _ => (a, 0),
            };
            println!(
                "n = {n:2} | {config:8} | |R| = {cardinal:13} | {median_ms:10.2} ms | fires = {rounds:4} | sweeps = {sweeps:2} | live = {live:9} | creations = {creations:10}"
            );
            rows.push(Row {
                n,
                config,
                cardinal,
                median_ms,
                rounds,
                sweeps,
                live,
                creations,
            });
        }
        // Cross-configuration consistency.
        let mut values: Vec<u64> = counts.values().copied().collect();
        values.dedup();
        assert_eq!(
            values.len(),
            1,
            "n = {n}: configurations diverge: {counts:?}"
        );
        if let Some(&pin) = PINS.get((n - 3) as usize).filter(|_| n <= 7) {
            assert_eq!(values[0], pin, "n = {n}: golden-thread pin violated");
        }
    }
    for r in &rows {
        println!(
            "RESULT {{\"n\":{},\"config\":\"{}\",\"cardinal\":{},\"median_ms\":{:.3},\"rounds\":{},\"sweeps\":{},\"live\":{},\"creations\":{}}}",
            r.n, r.config, r.cardinal, r.median_ms, r.rounds, r.sweeps, r.live, r.creations
        );
    }
    println!(
        "benchmark done: every assertion (oracle, pins, cross-configuration consistency) passes."
    );
}
