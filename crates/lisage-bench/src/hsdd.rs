//! HSDD spike (parent issue #382): hierarchical decision diagrams of the
//! Paris school installed as a **tower instance pattern** over the blind
//! pool — no mechanism added to `lisage-core` (C6 discipline): only a
//! controller, a key algebra and structural reads around the pool.
//!
//! Model: the philosophers ring, regular case (golden thread, n = 3..=7).
//! The gains mirrored from HSDD on regular models (Hamez, Thierry-Mieg,
//! Kordon, ATPN 2008):
//!
//! - **Storage (C8, O(1) forms + edge)**: the module (the philosopher
//!   automaton, its `(pc, left)` cells) is compiled **once** under one
//!   anchor; the ring is a single **set-valued instantiation arc** (mask of
//!   the n positions, the HSDD-4 in-arc) pointing at the module handle. The
//!   flat prototype of the same reachability set grows with n (pinned in
//!   `philosophers_bench`), the tower stores one edge per n.
//! - **Semantics (structural fold)**: the cardinal |R_n| is computed over
//!   the tower — path traversal of the module, adjacency coupling of
//!   consecutive cells, circular trace — polynomial in n, without expanding
//!   the product: the numbers reproduce the golden thread
//!   (20/56/152/416/1136, fold == enumeration).
//! - **C1**: the module re-interned under the same anchor is a hash-consing
//!   hit; a second pool rebuilds the same forms.
//!
//! Transductions (HSDD-5, issue #387): module-local events fired through
//! the tower as **data on the arc** — one event node per (event, position)
//! above the ring it transforms, chains stack; the `eat` guard crosses the
//! fragment boundary (the right neighbour) and is folded into the pair
//! coupling: image cardinalities by structural fold == enumeration.
//!
//! Saturation (HSDD-6, issue #388): recursive firing of a **set of
//! module-local events** through the whole ring, to a **fixpoint** — a
//! saturation node carries the event set as data on its arc above the
//! tower it saturates, nested chains stack one node per level; the
//! closure is folded structurally over the shared coupling (in-place
//! sweeps of the pair matrices, the ring is never expanded), and the
//! full event set saturates the initial state up to the golden thread
//! itself.
//!
//! Saturation policies (the #388 remainder): the fold runs under three
//! schedulers — the blind ring sweep, the **self-detected** Hamez lift
//! (the active fragments derived from the change structure, no explicit
//! user order) and the **per-fragment** sub-fixpoints (window-local
//! fixpoints, then lift-off through the shared boundary sheets) — all
//! through the core `Observer` port: every round emits the active and
//! modified fragments into the trace, with zero pool growth per round.
//!
//! Per-round diagrams and per-fragment memory versions (the #388 core
//! remainder, HSDD-6): the materialized engine saturates a state
//! interned as a diagram — one sheet node per position, at the
//! fragment level of the position — through the core `saturate`
//! itself: each round is one simultaneous firing of the set, the core
//! `Round` events carry the **real** before/image/after handles and the
//! real pool growth (M2-6 read back from the trace, the state countable
//! from the final diagram), and the runner persists across the rounds
//! so the memo and the memory of the untouched fragments survive (the
//! per-fragment versions of `MemoryLevel`, M2-5).
//!
//! Honest scope (next bricks of the track): external
//! libDDD/its-tools witness (HSDD-9, issue #391).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::error::CoreError;
use lisage_core::memory::EffectMemory;
use lisage_core::observer::{
    NoOpObserver, Observer, SpanId, SpanKind, TraceEvent, contextualize, next_span_id, pop_span,
    push_span,
};
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate as core_saturate};
use lisage_core::schema::{Domain, Schema, SchemaKeys};
use lisage_core::transduction::{PhiOutContext, Transduction as CoreTransduction};
use lisage_lang::value::Atom;

/// Level of the `pc` variable of the module.
pub const LEVEL_PC: u32 = 0;
/// Level of the `left` fork variable of the module.
pub const LEVEL_LEFT: u32 = 1;
/// Level of the instantiation selector of the ring (HSDD in-arc level).
pub const LEVEL_INSTANCE: u32 = 2;
/// Bound of the instantiation positions (mask of one `u64`).
pub const LIMIT_POSITIONS: u64 = 63;
/// `pc` value: think.
pub const THINK: u64 = 0;
/// `pc` value: hungry.
pub const HUNGRY: u64 = 1;
/// `pc` value: eat.
pub const EAT: u64 = 2;
/// Fork state: free (held by nobody).
pub const FREE: u64 = 0;
/// Fork state: held by the local philosopher.
pub const HELD: u64 = 1;
/// Fork state: taken by the previous philosopher.
pub const PREVIOUS: u64 = 2;
/// Cardinality of a `pc` or `left` level: {think, hungry, eat}.
pub const CARDINALITY_LOCAL: u64 = 3;
/// Cardinality of the instantiation level: positions 0..LIMIT_POSITIONS.
pub const CARDINALITY_POSITIONS: u64 = LIMIT_POSITIONS;
/// Expected |R_n| (golden thread, verified by simulation).
pub const RING_COUNTS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];

/// Typed error of the tower reads (causes preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorHsdd {
    /// The read node is not an instantiation node.
    RootNotInstance(NodeId),
    /// The read node is not a module node.
    RootNotModule(NodeId),
    /// A cell of the module is not (pc, left).
    UnexpectedTerminal(NodeId),
    /// The read node is not an event node of a transduction chain.
    RootNotEvent(NodeId),
    /// An event of the chain is unknown, or its position is outside the
    /// ring.
    EventOutOfRange { event: u64, position: u64 },
    /// The read node is not a saturation node of a saturation chain.
    RootNotSaturation(NodeId),
    /// A saturation set is empty or has an unknown event bit.
    EventSetOutOfRange(u64),
    /// The ring size is outside the instantiation bound.
    RingSizeOutOfRange(u64),
    /// A materialized sheet sits at the wrong level: the chain read
    /// expects the sheet of position `i` at `LEVEL_COUPLING_BASE + i`.
    CouplingLevel { found: u32, expected: u32 },
    /// A coupling arc key is outside the pc pair domain `|pcs|^2`.
    CouplingKey(u64),
    /// A materialized sheet carries two distinct arc targets: the
    /// single-target chain contract forbids a fork.
    CouplingFork(NodeId),
    /// Error propagated from the core saturation.
    Saturation(lisage_core::error::CoreError),
    /// Error propagated from the pool port.
    Pool(lisage_algebra::interface::PoolError),
}

impl std::fmt::Display for ErrorHsdd {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorHsdd::RootNotInstance(node) => {
                write!(formatter, "node {node:?} is not an instantiation node")
            }
            ErrorHsdd::RootNotModule(node) => {
                write!(formatter, "node {node:?} is not a module node")
            }
            ErrorHsdd::UnexpectedTerminal(node) => {
                write!(
                    formatter,
                    "node {node:?} is terminal where a cell was expected"
                )
            }
            ErrorHsdd::RootNotEvent(node) => {
                write!(
                    formatter,
                    "node {node:?} is not an event node of a transduction"
                )
            }
            ErrorHsdd::EventOutOfRange { event, position } => {
                write!(
                    formatter,
                    "unknown event {event} or position {position} outside the ring"
                )
            }
            ErrorHsdd::RootNotSaturation(node) => {
                write!(
                    formatter,
                    "node {node:?} is not a saturation node of a chain"
                )
            }
            ErrorHsdd::EventSetOutOfRange(set) => {
                write!(formatter, "unknown or empty event set {set}")
            }
            ErrorHsdd::RingSizeOutOfRange(size) => {
                write!(
                    formatter,
                    "ring size {size} outside the instantiation bound"
                )
            }
            ErrorHsdd::CouplingLevel { found, expected } => {
                write!(
                    formatter,
                    "coupling chain broken at level {found}, sheet of level {expected} expected"
                )
            }
            ErrorHsdd::CouplingKey(key) => {
                write!(
                    formatter,
                    "coupling arc key {key} outside the pc pair domain"
                )
            }
            ErrorHsdd::CouplingFork(node) => {
                write!(
                    formatter,
                    "node {node:?} carries a fork where the single-target chain contract forbids one"
                )
            }
            ErrorHsdd::Saturation(error) => write!(formatter, "core saturation error: {error}"),
            ErrorHsdd::Pool(error) => write!(formatter, "pool error: {error}"),
        }
    }
}

impl std::error::Error for ErrorHsdd {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorHsdd::Saturation(error) => Some(error),
            ErrorHsdd::Pool(error) => Some(error),
            _ => None,
        }
    }
}

impl From<lisage_algebra::interface::PoolError> for ErrorHsdd {
    fn from(error: lisage_algebra::interface::PoolError) -> Self {
        ErrorHsdd::Pool(error)
    }
}

impl From<CoreError> for ErrorHsdd {
    fn from(error: CoreError) -> Self {
        ErrorHsdd::Saturation(error)
    }
}

/// Key algebra of the tower (HSDD-4, the set-valued in-arc as a key):
/// `pc`/`left` levels carry atom keys; the instantiation level carries a
/// **set of positions** (bitmask of the held positions), normal form
/// already canonical. The pool stays blind: it only routes opaque `u64`.
pub struct TowerKeys;

impl KeyAlgebra for TowerKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, level: Level) -> u64 {
        match level.0 {
            LEVEL_PC | LEVEL_LEFT => CARDINALITY_LOCAL,
            LEVEL_EVENT => EVENT_COUNT * (LIMIT_POSITIONS + 1),
            LEVEL_SATURATION => (1 << EVENT_COUNT) - 1,
            level if level >= LEVEL_COUPLING_BASE => CARDINALITY_LOCAL * CARDINALITY_LOCAL,
            _ => CARDINALITY_POSITIONS,
        }
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

/// Controller of the tower instance (instance pattern, M5-3 ladder):
/// the module levels and the instantiation level in one pool. The two
/// reduction toggles are controller decisions feeding the ADR (HSDD-1,
/// issue #383):
/// - `collapse_active = false`: the ring node has one arc by construction
///   (all positions instantiated at once); R-collapse would erase the
///   instantiation record and collapse the ring into the module — the
///   hierarchy edge is semantic, not redundant;
/// - `default_active = false`: no implicit arc δ(ℓ) on the tower.
pub struct TowerController {
    keys: TowerKeys,
}

impl TowerController {
    /// New controller over the tower algebra.
    pub fn new() -> Self {
        TowerController { keys: TowerKeys }
    }
}

impl Default for TowerController {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller for TowerController {
    type Key = u64;
    type Payload = bool;
    type Keys = TowerKeys;
    type Arcs = ArcRun<Self::Key>;
    const FINGERPRINT: ControllerId = ControllerId(0x4853_4444);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
    fn collapse_active(&self) -> bool {
        false
    }
    fn default_active(&self) -> bool {
        false
    }
}

/// One cell of the module: the local state of one philosopher.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Cell {
    /// `pc` of the philosopher: think / hungry / eat.
    pub pc: u64,
    /// State of the left fork: free / held / previous.
    pub left: u64,
}

/// New pool for a tower installation.
pub fn pool_tower() -> Arena<TowerController> {
    Arena::new(TowerController::new(), Box::new(NoOpObserver))
}

/// Builds the **module** once: the philosopher automaton as the set of its
/// legal cells (0,free) (0,previous) (1,free) (1,previous) (2,held) — five
/// cells, five forms + the true terminal, shared by every position and
/// every n. Returns the module root.
pub fn build_module(pool: &mut Arena<TowerController>, anchor: Anchor) -> NodeId {
    let held = pool.intern(anchor, Form::Terminal(true));
    let lefts_free_or_previous = pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_LEFT),
            arcs: ArcRun::from_pairs([(FREE, held), (PREVIOUS, held)]),
        },
    );
    let left_held = pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_LEFT),
            arcs: ArcRun::from_pairs([(HELD, held)]),
        },
    );
    pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_PC),
            arcs: ArcRun::from_pairs([
                (THINK, lefts_free_or_previous),
                (HUNGRY, lefts_free_or_previous),
                (EAT, left_held),
            ]),
        },
    )
}

/// Builds the **ring** for n positions: one instantiation node whose
/// single arc key is the set of the n positions (HSDD in-arc), target the
/// module root. One node per distinct n; the module is never duplicated.
pub fn build_ring(
    pool: &mut Arena<TowerController>,
    anchor: Anchor,
    module: NodeId,
    n: u64,
) -> Result<NodeId, ErrorHsdd> {
    if n == 0 || n > LIMIT_POSITIONS {
        return Err(ErrorHsdd::Pool(
            lisage_algebra::interface::PoolError::UnknownNode(NodeId(n as u32)),
        ));
    }
    let positions = (1u64 << n) - 1;
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_INSTANCE),
            arcs: ArcRun::from_pairs([(positions, module)]),
        },
    ))
}

/// Reads the instance level of a ring root: the set of positions (mask)
/// and the module root it points at.
pub fn read_ring(pool: &Arena<TowerController>, ring: NodeId) -> Result<(u64, NodeId), ErrorHsdd> {
    let Form::Node { level, arcs } = pool.resolve(ring)? else {
        return Err(ErrorHsdd::RootNotInstance(ring));
    };
    if level.0 != LEVEL_INSTANCE {
        return Err(ErrorHsdd::RootNotInstance(ring));
    }
    let Some((mask, module)) = arcs.iter().next() else {
        return Err(ErrorHsdd::RootNotInstance(ring));
    };
    Ok((*mask, *module))
}

/// Reads the cells of the module by path traversal (structural read, no
/// expansion of the product): the module is a set DD over `(pc, left)`,
/// each path is one cell.
pub fn read_cells(pool: &Arena<TowerController>, module: NodeId) -> Result<Vec<Cell>, ErrorHsdd> {
    let Form::Node { level, arcs } = pool.resolve(module)? else {
        return Err(ErrorHsdd::RootNotModule(module));
    };
    if level.0 != LEVEL_PC {
        return Err(ErrorHsdd::RootNotModule(module));
    }
    let mut cells = Vec::new();
    for (pc, left_root) in arcs.iter() {
        let Form::Node {
            level: level_left,
            arcs: arcs_left,
        } = pool.resolve(*left_root)?
        else {
            return Err(ErrorHsdd::UnexpectedTerminal(*left_root));
        };
        if level_left.0 != LEVEL_LEFT {
            return Err(ErrorHsdd::RootNotModule(*left_root));
        }
        for (left, _) in arcs_left.iter() {
            cells.push(Cell {
                pc: *pc,
                left: *left,
            });
        }
    }
    cells.sort();
    cells.dedup();
    Ok(cells)
}

/// Fork rule of the golden thread: the `left` fork of a position, read
/// from the `pc` of the position and of its predecessor.
pub fn fork_state(pc_previous: u64, pc_current: u64) -> u64 {
    if pc_current == EAT {
        HELD
    } else if pc_previous == EAT {
        PREVIOUS
    } else {
        FREE
    }
}

/// Adjacency coupling of consecutive cells over the ring: legal pairs are
/// the pairs whose `left` state is consistent with the `pc` pair (and the
/// physical exclusion of two adjacent eaters).
pub fn coupling(cells: &[Cell]) -> Vec<Vec<u64>> {
    let mut matrix = vec![vec![0u64; cells.len()]; cells.len()];
    for (i, previous) in cells.iter().enumerate() {
        for (j, current) in cells.iter().enumerate() {
            let consistent = fork_state(previous.pc, current.pc) == current.left;
            let mutually_eating = previous.pc == EAT && current.pc == EAT;
            matrix[i][j] = u64::from(consistent && !mutually_eating);
        }
    }
    matrix
}

/// |Ring_n| by circular trace of the coupling matrix (exponentiation by
/// squaring): polynomial in n and in the number of cells — the tower is
/// never expanded into the product.
pub fn count_ring(matrix: &[Vec<u64>], n: u64) -> u64 {
    let size = matrix.len();
    let mut result = identity_matrix(size);
    let mut base: Vec<Vec<u128>> = matrix
        .iter()
        .map(|row| row.iter().map(|&value| value as u128).collect())
        .collect();
    let mut exponent = n;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = multiply_matrix(&result, &base);
        }
        base = multiply_matrix(&base, &base);
        exponent >>= 1;
    }
    (0..size).map(|i| result[i][i]).sum::<u128>() as u64
}

/// Identity matrix of the trace exponentiation.
fn identity_matrix(size: usize) -> Vec<Vec<u128>> {
    (0..size)
        .map(|i| (0..size).map(|j| u128::from(i == j)).collect())
        .collect()
}

/// Product of two square matrices over u128.
fn multiply_matrix(left: &[Vec<u128>], right: &[Vec<u128>]) -> Vec<Vec<u128>> {
    let size = left.len();
    let mut product = vec![vec![0u128; size]; size];
    for (i, row) in left.iter().enumerate() {
        for (j, _) in row.iter().enumerate() {
            let mut sum = 0u128;
            for k in 0..size {
                sum += row[k] * right[k][j];
            }
            product[i][j] = sum;
        }
    }
    product
}

/// Reference words of the golden thread (the enumeration oracle): circular
/// words over {think, hungry, eat} without two adjacent eaters.
pub fn words_reference(n: u64) -> Vec<Vec<u64>> {
    let n = n as usize;
    let mut words = Vec::new();
    for packed in 0u64..3u64.pow(n as u32) {
        let mut word = vec![0u64; n];
        let mut rest = packed;
        for slot in &mut word {
            *slot = rest % 3;
            rest /= 3;
        }
        let circular_ok = (0..n).all(|i| !(word[i] == EAT && word[(i + 1) % n] == EAT));
        if circular_ok {
            words.push(word);
        }
    }
    words
}

/// Reference cardinal |R_n| by enumeration (oracle of the A/B/C triptych).
pub fn count_words_reference(n: u64) -> u64 {
    words_reference(n).len() as u64
}

// ---------------------------------------------------------------------------
// HSDD-5: transductions across the fragment boundary (issue #387)
// ---------------------------------------------------------------------------

/// Level of the transduction nodes: the event selector above the
/// instance level. Pool-opaque like `LEVEL_INSTANCE`.
pub const LEVEL_EVENT: u32 = 3;
/// Module-local event: a thinking philosopher becomes hungry (no guard).
pub const EVENT_HUNGRY: u64 = 0;
/// Module-local event: a hungry philosopher starts eating. The local part
/// of the guard (`left` free) stays inside the cell; the rest **crosses the
/// fragment boundary** (no eater on either side) and is implied by the
/// fold below: firing beside an eater would build an illegal level word
/// (two adjacent eaters), so the coupling itself carries the guard.
pub const EVENT_EAT: u64 = 1;
/// Module-local event: an eating philosopher goes back to thinking.
pub const EVENT_THINK: u64 = 2;
/// Number of module-local events of the tower.
pub const EVENT_COUNT: u64 = 3;

/// Chain read from a transduced tower root: the events in **application
/// order** (innermost first — the root of the chain is the last applied),
/// the instantiation mask of the ring and the shared module root.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Transduction {
    /// Events of the chain: (event, position), first applied first.
    pub events: Vec<(u64, u64)>,
    /// Instantiation mask of the underlying ring (its popcount is n).
    pub mask: u64,
    /// Shared module root (never duplicated by a transduction).
    pub module: NodeId,
}

impl Transduction {
    /// Size n of the underlying ring (popcount of the mask).
    pub fn ring_size(&self) -> u64 {
        self.mask.count_ones() as u64
    }
}

/// Packs (event, position) into the arc key of an event node: the event
/// is **data on the arc** (M4-2 pattern), not a form.
pub fn pack_event(event: u64, position: u64) -> u64 {
    event * (LIMIT_POSITIONS + 1) + position
}

/// Unpacks the arc key of an event node into (event, position).
pub fn unpack_event(key: u64) -> (u64, u64) {
    (key / (LIMIT_POSITIONS + 1), key % (LIMIT_POSITIONS + 1))
}

/// Builds one **transduction node**: the event (event, position) as data
/// on a new arc, target the tower it transforms (ring or event node).
/// One node per (event, position, target), hash-consed: transducing the
/// same event twice is a hit, and the module and the ring are never
/// duplicated. Chains stack one node per event.
pub fn transduce(
    pool: &mut Arena<TowerController>,
    anchor: Anchor,
    target: NodeId,
    event: u64,
    position: u64,
) -> Result<NodeId, ErrorHsdd> {
    if event >= EVENT_COUNT || position >= LIMIT_POSITIONS {
        return Err(ErrorHsdd::EventOutOfRange { event, position });
    }
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_EVENT),
            arcs: ArcRun::from_pairs([(pack_event(event, position), target)]),
        },
    ))
}

/// Reads a transduced tower: walks the chain of event nodes down to the
/// ring (instantiation level), then the mask and the shared module root.
pub fn read_transduction(
    pool: &Arena<TowerController>,
    root: NodeId,
) -> Result<Transduction, ErrorHsdd> {
    let mut events: Vec<(u64, u64)> = Vec::new();
    let mut current = root;
    loop {
        let Form::Node { level, arcs } = pool.resolve(current)? else {
            return Err(ErrorHsdd::RootNotInstance(current));
        };
        match level.0 {
            LEVEL_EVENT => {
                let Some((key, next)) = arcs.iter().next() else {
                    return Err(ErrorHsdd::RootNotEvent(current));
                };
                events.push(unpack_event(*key));
                current = *next;
            }
            LEVEL_INSTANCE => {
                let Some((mask, module)) = arcs.iter().next() else {
                    return Err(ErrorHsdd::RootNotInstance(current));
                };
                // The chain read from the root is last-applied first.
                events.reverse();
                return Ok(Transduction {
                    events,
                    mask: *mask,
                    module: *module,
                });
            }
            _ => return Err(ErrorHsdd::RootNotEvent(current)),
        }
    }
}

/// pc value of a position under `event` (None: the event is not enabled on
/// this pc, the image of the chain dies at this position). The maps are
/// partial injections on the pcs, so the image of a set is counted without
/// duplicates. The guards of `eat` that read the *neighbours* are carried
/// by the coupling (see `count_image`), not by this map.
pub fn fire_pc(event: u64, pc: u64) -> Option<u64> {
    match (event, pc) {
        (EVENT_HUNGRY, THINK) => Some(HUNGRY),
        (EVENT_EAT, HUNGRY) => Some(EAT),
        (EVENT_THINK, EAT) => Some(THINK),
        _ => None,
    }
}

/// Index of a pc value in the golden alphabet (None when outside).
fn pc_index(pcs: &[u64], pc: u64) -> Option<usize> {
    pcs.iter().position(|candidate| *candidate == pc)
}

/// |image of R_n under the event chain| by structural fold over the
/// shared coupling - the **transduction across the fragment boundary**.
/// Every event of the chain is a partial map on the pc of one position
/// (`fire_pc`): the pcs of every level of the chain are functions of the
/// base pcs, a position without event is constant across the levels, and
/// the `left` fields are **derived** pairwise (`fork_state`), so a firing
/// that takes a neighbour fork updates it for free. Each level of the
/// chain must remain a legal ring word (no two adjacent eaters) - which
/// is also what carries the `eat` guards across the fragment boundary:
/// firing beside an eater would build an illegal level word. The
/// circular trace of the heterogeneous pair matrices counts the images in
/// O(n |pcs|^3) - the product is never expanded, the module is never
/// duplicated.
pub fn count_image(cells: &[Cell], n: u64, events: &[(u64, u64)]) -> Result<u64, ErrorHsdd> {
    validate_ring_events(n, events)?;
    let pcs = golden_pcs(cells);
    let size = pcs.len();
    let levels = build_levels(&pcs, n as usize, events);
    let matrices = build_pair_matrices(&pcs, &levels, n as usize);
    Ok(circular_trace(&matrices, size))
}

/// Validates the ring size and the event list (event id < EVENT_COUNT,
/// position < n).
fn validate_ring_events(n: u64, events: &[(u64, u64)]) -> Result<(), ErrorHsdd> {
    if n == 0 || n > LIMIT_POSITIONS {
        return Err(ErrorHsdd::RingSizeOutOfRange(n));
    }
    for &(event, position) in events {
        if event >= EVENT_COUNT || position >= n {
            return Err(ErrorHsdd::EventOutOfRange { event, position });
        }
    }
    Ok(())
}

/// Golden pc alphabet, read from the module cells (sorted, deduplicated).
fn golden_pcs(cells: &[Cell]) -> Vec<u64> {
    let mut pcs: Vec<u64> = cells.iter().map(|cell| cell.pc).collect();
    pcs.sort_unstable();
    pcs.dedup();
    pcs
}

/// pcs of every level of the chain at each position, as functions of the
/// base (level 0) pc index: `levels[position][level][base]`.
fn build_levels(pcs: &[u64], n: usize, events: &[(u64, u64)]) -> Vec<Vec<Vec<Option<usize>>>> {
    let size = pcs.len();
    let mut levels: Vec<Vec<Vec<Option<usize>>>> = Vec::with_capacity(n);
    for position in 0..n {
        let mut columns: Vec<Vec<Option<usize>>> = vec![(0..size).map(Some).collect()];
        for &(event, event_position) in events {
            let column = next_level_column(pcs, &columns, event, event_position, position);
            columns.push(column);
        }
        levels.push(columns);
    }
    levels
}

/// One level column at `position`: applies `event` when it fires there,
/// otherwise copies the previous column.
fn next_level_column(
    pcs: &[u64],
    columns: &[Vec<Option<usize>>],
    event: u64,
    event_position: u64,
    position: usize,
) -> Vec<Option<usize>> {
    let previous = columns.last().expect("depth invariant");
    let event_fires = event_position as usize == position;
    previous
        .iter()
        .map(|&maybe| {
            maybe.and_then(|index| {
                if event_fires {
                    fire_pc(event, pcs[index]).and_then(|fired| pc_index(pcs, fired))
                } else {
                    Some(index)
                }
            })
        })
        .collect()
}

/// Pair matrices of the folded coupling: the pair (position, position + 1)
/// of every level word must be legal (no two adjacent eaters).
fn build_pair_matrices(
    pcs: &[u64],
    levels: &[Vec<Vec<Option<usize>>>],
    n: usize,
) -> Vec<Vec<Vec<u64>>> {
    let size = pcs.len();
    let depth = levels
        .first()
        .map_or(0, |columns| columns.len().saturating_sub(1));
    let mut matrices: Vec<Vec<Vec<u64>>> = Vec::with_capacity(n);
    for position in 0..n {
        let next = (position + 1) % n;
        let mut matrix = vec![vec![0u64; size]; size];
        for (u, row) in matrix.iter_mut().enumerate() {
            for (v, cell) in row.iter_mut().enumerate() {
                *cell = u64::from(pair_is_legal(pcs, levels, position, next, depth, u, v));
            }
        }
        matrices.push(matrix);
    }
    matrices
}

/// Whether the pair (position u, next v) is legal across all level words:
/// both indices resolve at every level and no two adjacent eaters appear.
fn pair_is_legal(
    pcs: &[u64],
    levels: &[Vec<Vec<Option<usize>>>],
    position: usize,
    next: usize,
    depth: usize,
    u: usize,
    v: usize,
) -> bool {
    for column in 0..=depth {
        let (Some(current), Some(neighbour)) =
            (levels[position][column][u], levels[next][column][v])
        else {
            return false;
        };
        if pcs[current] == EAT && pcs[neighbour] == EAT {
            return false;
        }
    }
    true
}

/// Circular trace of the heterogeneous pair matrices (sum of the diagonal
/// of the product), counting the images without expanding the product.
fn circular_trace(matrices: &[Vec<Vec<u64>>], size: usize) -> u64 {
    let mut product: Vec<Vec<u128>> = lift_matrix(&matrices[0]);
    for matrix in &matrices[1..] {
        let lifted = lift_matrix(matrix);
        product = multiply_matrix(&product, &lifted);
    }
    (0..size).map(|i| product[i][i]).sum::<u128>() as u64
}

/// Lifts a u64 matrix to u128 for the trace product.
fn lift_matrix(matrix: &[Vec<u64>]) -> Vec<Vec<u128>> {
    matrix
        .iter()
        .map(|row| row.iter().map(|&value| value as u128).collect())
        .collect()
}

/// Fires `event` at `position` on a legal `pc` word of the golden thread
/// (the oracle side): the boundary guard of `eat` reads both neighbours.
pub fn fire_word(event: u64, position: usize, word: &[u64]) -> Option<Vec<u64>> {
    let n = word.len();
    let pc = word[position];
    let previous = word[(position + n - 1) % n];
    let next = word[(position + 1) % n];
    let fired_cell = match event {
        EVENT_HUNGRY if pc == THINK => Some(HUNGRY),
        EVENT_EAT if pc == HUNGRY && previous != EAT && next != EAT => Some(EAT),
        EVENT_THINK if pc == EAT => Some(THINK),
        _ => None,
    }?;
    let mut image = word.to_vec();
    image[position] = fired_cell;
    Some(image)
}

/// |image of R_n under the event chain| by enumeration (oracle of the
/// A/B/C triptych): the events are partial injective maps on the legal
/// words, so the image of the set is counted without duplicates.
pub fn count_image_reference(n: u64, events: &[(u64, u64)]) -> u64 {
    let mut words = words_reference(n);
    for &(event, position) in events {
        words = words
            .into_iter()
            .filter_map(|word| fire_word(event, position as usize, &word))
            .collect();
    }
    words.len() as u64
}

/// Level of a saturation node: the event set is data on its arc (one node
/// per firing set, whatever the tower it saturates).
pub const LEVEL_SATURATION: u32 = 4;
/// Base of the coupling levels of the materialized state: the sheet of
/// position `i` lives at level `LEVEL_COUPLING_BASE + i` — the fragment
/// of the position (the per-fragment memory version of the core is one
/// entry per sheet level, so an untouched sheet never invalidates the
/// memo entries of its fragment).
pub const LEVEL_COUPLING_BASE: u32 = 5;

/// Chain read from a saturated tower root: the event sets in **application
/// order** (innermost first — the root of the chain is the last applied),
/// the instantiation mask of the ring and the shared module root.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Saturation {
    /// Event sets of the chain: one bit per module-local event, first
    /// applied first.
    pub sets: Vec<u64>,
    /// Instantiation mask of the underlying ring (its popcount is n).
    pub mask: u64,
    /// Shared module root (never duplicated by a saturation).
    pub module: NodeId,
}

impl Saturation {
    /// Size n of the underlying ring (popcount of the mask).
    pub fn ring_size(&self) -> u64 {
        self.mask.count_ones() as u64
    }
}

/// Packs an event set into the arc key of a saturation node: the whole set
/// is **data on the arc** (one bit per module-local event), never a form.
///
/// # Panics
///
/// Panics when an event id is outside the alphabet (`event >= EVENT_COUNT`):
/// the shift would overflow or silently alias in release builds.
pub fn pack_event_set(events: &[u64]) -> u64 {
    let mut set = 0u64;
    for &event in events {
        assert!(event < EVENT_COUNT, "event id {event} outside the alphabet");
        set |= 1 << event;
    }
    set
}

/// Builds one **saturation node**: the event set as data on a new arc,
/// target the tower it saturates (a ring or another saturation node — a
/// nested chain is recursive saturation). One node per (set, target),
/// hash-consed: the same set on the same tower is a hit, the ring and the
/// module are never duplicated.
pub fn saturate(
    pool: &mut Arena<TowerController>,
    anchor: Anchor,
    target: NodeId,
    events: u64,
) -> Result<NodeId, ErrorHsdd> {
    if events == 0 || events >= 1 << EVENT_COUNT {
        return Err(ErrorHsdd::EventSetOutOfRange(events));
    }
    Ok(pool.intern(
        anchor,
        Form::Node {
            level: Level(LEVEL_SATURATION),
            arcs: ArcRun::from_pairs([(events, target)]),
        },
    ))
}

/// Reads a saturated tower: walks the chain of saturation nodes down to
/// the ring (instantiation level), then the mask and the shared module
/// root. Nested chains expose every set, innermost first.
pub fn read_saturation(
    pool: &Arena<TowerController>,
    root: NodeId,
) -> Result<Saturation, ErrorHsdd> {
    let mut sets: Vec<u64> = Vec::new();
    let mut current = root;
    loop {
        let Form::Node { level, arcs } = pool.resolve(current)? else {
            return Err(ErrorHsdd::RootNotSaturation(current));
        };
        match level.0 {
            LEVEL_SATURATION => {
                let Some((events, next)) = arcs.iter().next() else {
                    return Err(ErrorHsdd::RootNotSaturation(current));
                };
                sets.push(*events);
                current = *next;
            }
            LEVEL_INSTANCE => {
                let Some((mask, module)) = arcs.iter().next() else {
                    return Err(ErrorHsdd::RootNotSaturation(current));
                };
                // The chain read from the root is last-applied first.
                sets.reverse();
                return Ok(Saturation {
                    sets,
                    mask: *mask,
                    module: *module,
                });
            }
            _ => return Err(ErrorHsdd::RootNotSaturation(current)),
        }
    }
}

/// Fixpoint measurement of a saturation: closed count and the sweeps that
/// reached it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClosureCount {
    /// |closed language| by structural fold (circular trace).
    pub count: u64,
    /// Sweeps through the ring that reached and confirmed the fixpoint (a
    /// closed seed counts zero rounds).
    pub rounds: u64,
}

/// Fires `event` at `position` over the pair matrices of consecutive
/// cells: the fired cell is the right cell of the pair before it and the
/// left cell of the pair after it. The `eat` guard reads both neighbours:
/// a firing beside an eater would build an illegal (eat, eat) pair, so the
/// clamped pair itself carries the boundary guard, now on both sides and
/// to the fixpoint (the HSDD-2 crossing of the transduction brick).
fn fire_position(
    matrices: &mut [Vec<Vec<u64>>],
    alphabet: &[u64],
    event: u64,
    position: usize,
) -> bool {
    let n = matrices.len();
    let before = (position + n - 1) % n;
    let mut changed = false;
    for (u, &pc) in alphabet.iter().enumerate() {
        let Some(fired) = fire_pc(event, pc) else {
            continue;
        };
        let Some(v) = pc_index(alphabet, fired) else {
            continue;
        };
        changed |= fire_left_guard(matrices, alphabet, before, u, v);
        changed |= fire_right_guard(matrices, alphabet, position, u, v);
    }
    changed
}

/// Left-neighbour leg of the firing: writes the pair (before, position) for
/// every left index `a`, skipping when the left neighbour eats and the
/// fired cell is `eat` (the `eat` boundary guard).
fn fire_left_guard(
    matrices: &mut [Vec<Vec<u64>>],
    alphabet: &[u64],
    before: usize,
    u: usize,
    v: usize,
) -> bool {
    let mut changed = false;
    for (a, &left) in alphabet.iter().enumerate() {
        if matrices[before][a][u] == 0 {
            continue;
        }
        if left == EAT && alphabet[v] == EAT {
            continue;
        }
        if matrices[before][a][v] == 0 {
            matrices[before][a][v] = 1;
            changed = true;
        }
    }
    changed
}

/// Right-neighbour leg of the firing: writes the pair (position, next) for
/// every right index `b`, skipping when the fired cell is `eat` and the
/// right neighbour eats (the `eat` boundary guard).
fn fire_right_guard(
    matrices: &mut [Vec<Vec<u64>>],
    alphabet: &[u64],
    position: usize,
    u: usize,
    v: usize,
) -> bool {
    let mut changed = false;
    for (b, &right) in alphabet.iter().enumerate() {
        if matrices[position][u][b] == 0 {
            continue;
        }
        if alphabet[v] == EAT && right == EAT {
            continue;
        }
        if matrices[position][v][b] == 0 {
            matrices[position][v][b] = 1;
            changed = true;
        }
    }
    changed
}

/// Fires every event of the set at one position (the local step of every
/// scheduling policy below). Returns whether the position's pair sheets
/// changed — the fragment-level change detection the policies feed on.
fn fire_set_at(
    matrices: &mut [Vec<Vec<u64>>],
    alphabet: &[u64],
    set: u64,
    position: usize,
) -> bool {
    let mut changed = false;
    for event in 0..EVENT_COUNT {
        if set & (1 << event) != 0 {
            changed |= fire_position(matrices, alphabet, event, position);
        }
    }
    changed
}

/// One sweep of the event set through the ring (a round of the
/// saturation): every event of the set fires at every position, **in
/// place** — the matrices only grow, position i fires after position i - 1
/// and reads its fresh pairs, so information travels the whole ring in one
/// sweep, exactly the in-place firing of DDD saturation. Returns whether
/// the sweep changed any pair.
fn sweep_event_set(matrices: &mut [Vec<Vec<u64>>], alphabet: &[u64], set: u64) -> bool {
    let mut changed = false;
    for position in 0..matrices.len() {
        changed |= fire_set_at(matrices, alphabet, set, position);
    }
    changed
}

/// |closure of the saturation| by structural fold over the shared
/// coupling — the seed is the **initial state** of the philosophers
/// (every philosopher thinks), the sweeps grow the pair matrices of
/// consecutive cells in place until nothing changes, and the closed count
/// is the circular trace of the product: the ring is never expanded, the
/// module is never duplicated, the count is exact. The sets of the chain
/// apply in order — a nested set saturates the closure of the previous
/// one (recursive saturation), and saturating a closed set with itself
/// does no sweep (idempotence).
pub fn count_closure(cells: &[Cell], n: u64, sets: &[u64]) -> Result<ClosureCount, ErrorHsdd> {
    if n == 0 || n > LIMIT_POSITIONS {
        return Err(ErrorHsdd::RingSizeOutOfRange(n));
    }
    for &set in sets {
        if set == 0 || set >= 1 << EVENT_COUNT {
            return Err(ErrorHsdd::EventSetOutOfRange(set));
        }
    }
    // Golden pc alphabet, read from the module cells.
    let mut pcs: Vec<u64> = cells.iter().map(|cell| cell.pc).collect();
    pcs.sort_unstable();
    pcs.dedup();
    let size = pcs.len();
    let n = n as usize;
    // Seed: the initial state — every philosopher thinks, so only the
    // (think, think) pair is reachable.
    let mut matrices: Vec<Vec<Vec<u64>>> = (0..n)
        .map(|_| {
            (0..size)
                .map(|u| {
                    (0..size)
                        .map(|v| u64::from(pcs[u] == THINK && pcs[v] == THINK))
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut rounds = 0u64;
    for &set in sets {
        let mut sweeps = 0u64;
        loop {
            let changed = sweep_event_set(&mut matrices, &pcs, set);
            if !changed {
                break;
            }
            sweeps += 1;
        }
        // The trailing unproductive sweep confirms the fixpoint: it
        // counts when at least one sweep produced a change.
        if sweeps > 0 {
            sweeps += 1;
        }
        rounds += sweeps;
    }
    // Circular trace of the closed pair matrices (the heterogeneous count
    // of `count_image`, at the fixpoint).
    let count = trace_count(&matrices, size);
    Ok(ClosureCount { count, rounds })
}

/// Closed count of the pair matrices: the circular trace of their
/// product (the heterogeneous count of `count_image`, at the fixpoint)
/// — the product of the matrices is counted, never expanded.
fn trace_count(matrices: &[Vec<Vec<u64>>], size: usize) -> u64 {
    let mut product: Vec<Vec<u128>> = matrices[0]
        .iter()
        .map(|row| row.iter().map(|&value| value as u128).collect())
        .collect();
    for matrix in &matrices[1..] {
        let lifted: Vec<Vec<u128>> = matrix
            .iter()
            .map(|row| row.iter().map(|&value| value as u128).collect())
            .collect();
        product = multiply_matrix(&product, &lifted);
    }
    (0..size).map(|i| product[i][i]).sum::<u128>() as u64
}

/// |closure of the saturation| by BFS enumeration (the enumeration side
/// of the A/B/C triptych): fires every event of every set at every
/// position from the initial state (every philosopher thinks) until the
/// closure is reached — the oracle the fold must reproduce exactly.
pub fn count_closure_reference(n: u64, sets: &[u64]) -> u64 {
    let n = n as usize;
    let mut seen: Vec<Vec<u64>> = vec![vec![THINK; n]];
    let mut frontier = seen.clone();
    while !frontier.is_empty() {
        let mut next: Vec<Vec<u64>> = Vec::new();
        for word in &frontier {
            for &set in sets {
                for event in 0..EVENT_COUNT {
                    if set & (1 << event) == 0 {
                        continue;
                    }
                    for position in 0..n {
                        if let Some(image) = fire_word(event, position, word)
                            && !seen.contains(&image)
                        {
                            seen.push(image.clone());
                            next.push(image);
                        }
                    }
                }
            }
        }
        frontier = next;
    }
    seen.len() as u64
}

// --- Saturation policies: observed, self-detected, per-fragment (#388) ---

/// Recording observer of the tower, the bench side of the M1-0 port: it
/// plugs into the pool (`pool_tower_observed` — the pool emits its own
/// storage events through it) and into the observed saturation engines
/// below. Clones share the same buffer.
#[derive(Clone)]
pub struct TraceCollector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl TraceCollector {
    /// New empty collector.
    pub fn new() -> Self {
        TraceCollector {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Events recorded so far, in emission order.
    pub fn events(&self) -> Vec<TraceEvent> {
        self.events.lock().expect("collector lock").clone()
    }

    /// Round events recorded, as `(number, active fragments, modified
    /// fragments)` — the acceptance-criterion triple of #388.
    pub fn rounds(&self) -> Vec<(u32, Vec<u32>, Vec<u32>)> {
        self.events()
            .iter()
            .filter_map(|event| match event {
                TraceEvent::Round {
                    number,
                    active_levels,
                    modified_levels,
                    ..
                } => Some((*number, active_levels.clone(), modified_levels.clone())),
                _ => None,
            })
            .collect()
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for TraceCollector {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("collector lock")
            .push(event.clone());
    }
}

/// Observed tower: the pool armed with a trace observer instead of the
/// no-op default — the pool emits its storage events (`NodeCreated`,
/// `NodeShared`, reductions) through it, exactly as on the flat pool.
pub fn pool_tower_observed(observer: Box<dyn Observer>) -> Arena<TowerController> {
    Arena::new(TowerController::new(), observer)
}

/// Fold-time read of a saturated tower: the validated ring size, the
/// firing sets of the chain and the sorted golden pc alphabet.
struct TowerRead {
    ring_size: u64,
    sets: Vec<u64>,
    pcs: Vec<u64>,
}

/// Engine read of a saturated tower: validates the chain (the same
/// errors as `count_closure`), reads the shared cells and the golden pc
/// alphabet.
fn fold_tower(pool: &Arena<TowerController>, root: NodeId) -> Result<TowerRead, ErrorHsdd> {
    let saturation = read_saturation(pool, root)?;
    let n = saturation.ring_size();
    if n == 0 || n > LIMIT_POSITIONS {
        return Err(ErrorHsdd::RingSizeOutOfRange(n));
    }
    for &set in &saturation.sets {
        if set == 0 || set >= 1 << EVENT_COUNT {
            return Err(ErrorHsdd::EventSetOutOfRange(set));
        }
    }
    let cells = read_cells(pool, saturation.module)?;
    let mut pcs: Vec<u64> = cells.iter().map(|cell| cell.pc).collect();
    pcs.sort_unstable();
    pcs.dedup();
    Ok(TowerRead {
        ring_size: n,
        sets: saturation.sets.clone(),
        pcs,
    })
}

/// Matrices of the **initial state** seed: every philosopher thinks, so
/// only the (think, think) pair is reachable — the seed of every policy.
fn seed_matrices(n: usize, pcs: &[u64]) -> Vec<Vec<Vec<u64>>> {
    let size = pcs.len();
    (0..n)
        .map(|_| {
            (0..size)
                .map(|u| {
                    (0..size)
                        .map(|v| u64::from(pcs[u] == THINK && pcs[v] == THINK))
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// Structural fingerprint of a saturated chain (n, sets), packed into the
/// OpStart event.
fn chain_fingerprint(n: u64, sets: &[u64]) -> u64 {
    sets.iter().fold(n, |acc, &set| acc.rotate_left(8) ^ set)
}

/// Opens the op span of a saturation and emits `OpStart` inside it. The
/// caller closes with `emit_op_end`.
fn emit_op_start(observer: &dyn Observer, fingerprint: u64) -> SpanId {
    let op = next_span_id();
    push_span(op);
    observer.on_event(&TraceEvent::SpanOpen {
        span: op,
        kind: SpanKind::Op,
        lane: 0,
    });
    observer.on_event(&contextualize(TraceEvent::OpStart {
        fingerprint,
        span: None,
    }));
    op
}

/// Closes the op span of a saturation: `OpEnd` then `SpanClose`.
fn emit_op_end(observer: &dyn Observer, op: SpanId) {
    observer.on_event(&contextualize(TraceEvent::OpEnd { span: None }));
    pop_span();
    observer.on_event(&TraceEvent::SpanClose { span: op });
}

/// Emits one core round event for a tower round (the #388 acceptance
/// visibility), inside its own `Round` span: `number` (1-based),
/// `active_levels` the **fragments** fired in the round (the positions,
/// the n instantiated module copies), `modified_levels` the fragments
/// whose pair sheets changed, `before/image/after` the saturated tower
/// root (the fold is matrix-side: no per-round handle is rebuilt) and
/// `growth` 0 — the sweeps intern nothing, the tower stays one edge per
/// set.
fn emit_round(
    observer: &dyn Observer,
    number: u32,
    active: &BTreeSet<usize>,
    modified: &BTreeSet<usize>,
    root: NodeId,
) {
    let span = next_span_id();
    push_span(span);
    observer.on_event(&TraceEvent::SpanOpen {
        span,
        kind: SpanKind::Round,
        lane: 0,
    });
    observer.on_event(&contextualize(TraceEvent::Round {
        number,
        active_levels: active.iter().map(|&position| position as u32).collect(),
        modified_levels: modified.iter().map(|&position| position as u32).collect(),
        before: root,
        image: root,
        after: root,
        growth: 0,
        span: None,
    }));
    pop_span();
    observer.on_event(&TraceEvent::SpanClose { span });
}

/// Saturates the stored tower under the **ring** policy (the blind
/// sweep of `count_closure`, every position once per sweep), **through
/// the observer**: every performed sweep — the confirming ones included
/// — emits the core round event with the active and modified fragments.
/// Same closure and same round accounting as the fold.
pub fn saturate_observed(
    pool: &Arena<TowerController>,
    root: NodeId,
    observer: &dyn Observer,
) -> Result<ClosureCount, ErrorHsdd> {
    let TowerRead {
        ring_size: n,
        sets,
        pcs,
    } = fold_tower(pool, root)?;
    let op = emit_op_start(observer, chain_fingerprint(n, &sets));
    let mut matrices = seed_matrices(n as usize, &pcs);
    let all: BTreeSet<usize> = (0..n as usize).collect();
    let mut number = 1u32;
    let mut rounds = 0u64;
    for &set in &sets {
        let mut productive = 0u64;
        loop {
            let mut modified = BTreeSet::new();
            for &position in &all {
                if fire_set_at(&mut matrices, &pcs, set, position) {
                    modified.insert(position);
                }
            }
            emit_round(observer, number, &all, &modified, root);
            number += 1;
            if modified.is_empty() {
                break;
            }
            productive += 1;
        }
        // The trailing unproductive sweep confirms the fixpoint: it
        // counts when at least one sweep produced a change.
        if productive > 0 {
            rounds += productive + 1;
        }
    }
    let closed = trace_count(&matrices, pcs.len());
    emit_op_end(observer, op);
    Ok(ClosureCount {
        count: closed,
        rounds,
    })
}

/// Closure and work of the **self-detected** policy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DetectedClosure {
    /// Closed count, rounds = waves (productive + confirming).
    pub closed: ClosureCount,
    /// Position firings performed (the measured work of the policy).
    pub firings: u64,
}

/// Saturates the stored tower under the **self-detected** policy — the
/// Hamez strategy lifted to the tower, the M2-4 AutoDetectee
/// generalized: no explicit order. The seed activates every fragment;
/// a wave fires only the ACTIVE fragments, and the next wave is
/// derived from the structure alone: the fragments whose pair sheets
/// CHANGED re-arm their neighbourhood (the changed fragment and its two
/// neighbours — the only fragments a change can reach through the
/// coupling). A wave that modifies nothing confirms the fixpoint. The
/// trace carries the same round triple as the ring policy; the
/// firings measure the schedule against the blind sweep.
pub fn saturate_detected(
    pool: &Arena<TowerController>,
    root: NodeId,
    observer: &dyn Observer,
) -> Result<DetectedClosure, ErrorHsdd> {
    let TowerRead {
        ring_size: n,
        sets,
        pcs,
    } = fold_tower(pool, root)?;
    let n = n as usize;
    let op = emit_op_start(observer, chain_fingerprint(n as u64, &sets));
    let mut matrices = seed_matrices(n, &pcs);
    let mut number = 1u32;
    let mut rounds = 0u64;
    let mut firings = 0u64;
    for &set in &sets {
        let mut active: BTreeSet<usize> = (0..n).collect();
        let mut productive = 0u64;
        loop {
            let mut modified = BTreeSet::new();
            for &position in &active {
                firings += 1;
                if fire_set_at(&mut matrices, &pcs, set, position) {
                    modified.insert(position);
                }
            }
            emit_round(observer, number, &active, &modified, root);
            number += 1;
            if modified.is_empty() {
                break;
            }
            productive += 1;
            // The structure speaks: only the neighbourhood of a change
            // can fire again.
            active = modified
                .iter()
                .flat_map(|&position| [(position + n - 1) % n, position, (position + 1) % n])
                .collect();
        }
        if productive > 0 {
            rounds += productive + 1;
        }
    }
    let closed = trace_count(&matrices, pcs.len());
    emit_op_end(observer, op);
    Ok(DetectedClosure {
        closed: ClosureCount {
            count: closed,
            rounds,
        },
        firings,
    })
}

/// Closure and work of the **per-fragment** policy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FragmentedClosure {
    /// Closed count, rounds = parent passes (productive + confirming).
    pub closed: ClosureCount,
    /// Position firings performed inside the windows.
    pub firings: u64,
    /// Local fixpoint iterations of the windows (confirming ones included).
    pub local_fixpoints: u64,
}

/// Mutable state threaded through the fragment firing helpers.
struct FragmentFire<'a> {
    matrices: &'a mut [Vec<Vec<u64>>],
    pcs: &'a [u64],
    set: u64,
    n: usize,
    fragment: usize,
    firings: u64,
    local_fixpoints: u64,
}

/// One parent pass over the ring: every window of `fragment` consecutive
/// positions fires to its local fixpoint. Returns the set of modified
/// positions.
fn fire_parent_pass(state: &mut FragmentFire<'_>) -> BTreeSet<usize> {
    let mut modified = BTreeSet::new();
    for start in 0..state.n {
        fire_window_to_fixpoint(state, start, &mut modified);
    }
    modified
}

/// Fires one window (starting at `start`, length `fragment`) to its local
/// fixpoint: the window keeps sweeping until a pass changes nothing.
fn fire_window_to_fixpoint(
    state: &mut FragmentFire<'_>,
    start: usize,
    modified: &mut BTreeSet<usize>,
) {
    loop {
        let mut window_changed = false;
        for offset in 0..state.fragment {
            let position = (start + offset) % state.n;
            state.firings += 1;
            if fire_set_at(state.matrices, state.pcs, state.set, position) {
                modified.insert(position);
                window_changed = true;
            }
        }
        state.local_fixpoints += 1;
        if !window_changed {
            break;
        }
    }
}

/// Saturates the stored tower under the **per-fragment** policy (HSDD-6):
/// the ring splits into the n overlapping windows of `fragment`
/// consecutive positions. A parent pass sweeps the windows; each window
/// fires **to its local fixpoint** (its fragments fire out-of-order inside
/// it, until the window is quiet) before the lift-off to the next — the
/// parent pair sheets carry the change through the window boundaries,
/// the per-fragment memory of the transduction brick (the boundary
/// guard rides the shared coupling). Parent passes repeat until the
/// whole ring is stable; the trace carries one round event per parent
/// pass.
pub fn saturate_fragments(
    pool: &Arena<TowerController>,
    root: NodeId,
    fragment: u64,
    observer: &dyn Observer,
) -> Result<FragmentedClosure, ErrorHsdd> {
    let TowerRead {
        ring_size: n,
        sets,
        pcs,
    } = fold_tower(pool, root)?;
    if fragment == 0 || fragment > n {
        return Err(ErrorHsdd::RingSizeOutOfRange(fragment));
    }
    let n = n as usize;
    let fragment = fragment as usize;
    let op = emit_op_start(observer, chain_fingerprint(n as u64, &sets));
    let mut matrices = seed_matrices(n, &pcs);
    let all: BTreeSet<usize> = (0..n).collect();
    let mut number = 1u32;
    let mut rounds = 0u64;
    let mut firings_total = 0u64;
    let mut local_fixpoints_total = 0u64;
    for &set in &sets {
        let mut state = FragmentFire {
            matrices: &mut matrices,
            pcs: &pcs,
            set,
            n,
            fragment,
            firings: 0,
            local_fixpoints: 0,
        };
        let mut productive = 0u64;
        loop {
            let modified = fire_parent_pass(&mut state);
            emit_round(observer, number, &all, &modified, root);
            number += 1;
            if modified.is_empty() {
                break;
            }
            productive += 1;
        }
        if productive > 0 {
            rounds += productive + 1;
        }
        firings_total += state.firings;
        local_fixpoints_total += state.local_fixpoints;
    }
    let closed = trace_count(&matrices, pcs.len());
    emit_op_end(observer, op);
    Ok(FragmentedClosure {
        closed: ClosureCount {
            count: closed,
            rounds,
        },
        firings: firings_total,
        local_fixpoints: local_fixpoints_total,
    })
}

/// Fingerprint salt of the firing-set step transduction: domain-separates
/// the materialized saturation steps from the other tower fingerprints.
const FINGERPRINT_FIRE_SET: u64 = 0x4855_5354_0000_0000;

/// Fingerprint of a firing-set step: folds the set bits, so two steps of
/// different sets never share a memo entry.
fn fire_set_fingerprint(set: u64) -> u64 {
    (0..EVENT_COUNT).fold(FINGERPRINT_FIRE_SET, |acc, event| {
        if set & (1 << event) != 0 {
            acc.rotate_left(8) ^ (0xF1EA_u64 + event)
        } else {
            acc
        }
    })
}

/// Key algebra of the coupling levels for the `union` of the core
/// `saturate`: an atom domain of the pc pairs `|pcs|^2` — the levels
/// `LEVEL_COUPLING_BASE + i` all share it (no default arc: the sheets
/// carry their pairs explicitly).
fn coupling_keys(size: usize) -> SchemaKeys {
    let domain = Domain::new((0..(size * size) as u64).map(Atom).collect());
    SchemaKeys::new(Schema::Atom(domain))
}

/// Interns a coupling state as a **materialized diagram**: one sheet node
/// per position, the sheet of position `i` at level
/// `LEVEL_COUPLING_BASE + i` (the fragment of the position), arcs keyed
/// by the present pc pairs, all pointing at the sheet of position `i + 1`
/// — the deepest sheet points at the true terminal. Hash-consing shares
/// the equal sheets: two positions with the same pairs are the same node
/// only when they are the same fragment; across rounds, the stable sheets
/// are re-interned as themselves (the handle identity the saturation
/// detects the fixpoint with).
fn intern_coupling_chain(
    pool: &mut Arena<TowerController>,
    anchor: Anchor,
    matrices: &[Vec<Vec<u64>>],
    size: usize,
) -> Result<NodeId, ErrorHsdd> {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let mut target = true_node;
    for (position, matrix) in matrices.iter().enumerate().rev() {
        let mut sheets: Vec<(u64, NodeId)> = Vec::new();
        for (u, row) in matrix.iter().enumerate() {
            for (v, &present) in row.iter().enumerate() {
                if present != 0 {
                    sheets.push((u as u64 * size as u64 + v as u64, target));
                }
            }
        }
        target = pool.intern(
            anchor,
            Form::Node {
                level: Level(LEVEL_COUPLING_BASE + position as u32),
                arcs: ArcRun::from_pairs(sheets),
            },
        );
    }
    Ok(target)
}

/// Reads a materialized coupling state back into its pair matrices: the
/// inverse of `intern_coupling_chain` (the round-trip witness: the
/// diagram IS the coupling state, countable by reading it back).
pub fn read_coupling(
    pool: &Arena<TowerController>,
    state: NodeId,
    n: usize,
    size: usize,
) -> Result<Vec<Vec<Vec<u64>>>, ErrorHsdd> {
    let mut matrices = vec![vec![vec![0u64; size]; size]; n];
    let mut current = state;
    for (position, matrix) in matrices.iter_mut().enumerate() {
        let form = pool.resolve(current).map_err(ErrorHsdd::Pool)?;
        let Form::Node { level, arcs } = form else {
            return Err(ErrorHsdd::UnexpectedTerminal(current));
        };
        if level.0 != LEVEL_COUPLING_BASE + position as u32 {
            return Err(ErrorHsdd::CouplingLevel {
                found: level.0,
                expected: LEVEL_COUPLING_BASE + position as u32,
            });
        }
        let mut next: Option<NodeId> = None;
        for (key, target) in arcs {
            let (u, v) = (key / size as u64, key % size as u64);
            if u >= size as u64 || v >= size as u64 {
                return Err(ErrorHsdd::CouplingKey(key));
            }
            match next {
                None => next = Some(target),
                Some(previous) if previous == target => {}
                Some(_) => return Err(ErrorHsdd::CouplingFork(current)),
            }
            matrix[u as usize][v as usize] = 1;
        }
        current = next.ok_or(ErrorHsdd::UnexpectedTerminal(current))?;
    }
    Ok(matrices)
}

/// One **simultaneous firing step** of an event set over the materialized
/// coupling state: the step transduction the core `saturate` iterates.
/// Each visit restructures one sheet (the fragment of one position): the
/// sheet of position `i` contains the pairs `(pc_i, pc_i+1)`, so it
/// receives the row updates of the firings **at position `i`** (its right
/// pc column) and the column updates of the firings at position `i + 1`
/// (its left pc row) — both computable from the sheet alone plus the set,
/// the eat guard carried by the pair legality (the `(eat, eat)` pair is
/// never written, exactly the two-sided guard of `fire_position`).
///
/// The contract of the restructuring is **single-target sheets** (the
/// chain pattern): a node carrying two distinct arc targets is copied
/// identically (the chain construction only ever interns single-target
/// sheets, and `union` preserves it).
#[derive(Clone)]
struct FireSetStep {
    /// The firing set: one bit per module-local event.
    set: u64,
    /// The golden pc alphabet, read from the module.
    alphabet: Vec<u64>,
    /// Fingerprint of the step.
    fingerprint: u64,
}

impl CoreTransduction for FireSetStep {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        self.fingerprint
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: PhiOutContext<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffectMemory<()>) {
        let alphabet = &self.alphabet;
        let size = alphabet.len();
        let arcs: Vec<(u64, NodeId)> = ctx
            .arcs
            .iter()
            .map(|(key, (node, _))| (*key, *node))
            .collect();
        let target = arcs.first().map(|&(_, node)| node);
        let single_target =
            target.is_some() && arcs.iter().all(|&(_, node)| node == target.unwrap());
        if !single_target {
            // Multi-target node: outside the chain pattern, identity copy.
            return (
                pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: ctx.level,
                        arcs: ArcRun::from_pairs(arcs),
                    },
                ),
                EffectMemory::new(),
            );
        }
        let target = target.unwrap();
        // Input sheet: the pairs present on the visited node.
        let mut matrix = vec![vec![0u64; size]; size];
        for &(key, _) in &arcs {
            matrix[(key / size as u64) as usize][(key % size as u64) as usize] = 1;
        }
        // One simultaneous firing of the set on this sheet: the row
        // updates (firing at the position owning the right pc) and the
        // column updates (firing at the position owning the left pc),
        // the same guarded writes as `fire_position`, left and right.
        for event in 0..EVENT_COUNT {
            if self.set & (1 << event) == 0 {
                continue;
            }
            for (u, &pc) in alphabet.iter().enumerate() {
                let Some(fired) = fire_pc(event, pc) else {
                    continue;
                };
                let Some(v) = pc_index(alphabet, fired) else {
                    continue;
                };
                for a in 0..size {
                    if matrix[a][u] == 0 {
                        continue;
                    }
                    if alphabet[a] == EAT && alphabet[v] == EAT {
                        continue;
                    }
                    matrix[a][v] = 1;
                }
                for b in 0..size {
                    if matrix[u][b] == 0 {
                        continue;
                    }
                    if alphabet[v] == EAT && alphabet[b] == EAT {
                        continue;
                    }
                    matrix[v][b] = 1;
                }
            }
        }
        let mut sheets: Vec<(u64, NodeId)> = Vec::new();
        for (u, row) in matrix.iter().enumerate() {
            for (v, &present) in row.iter().enumerate() {
                if present != 0 {
                    sheets.push((u as u64 * size as u64 + v as u64, target));
                }
            }
        }
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ArcRun::from_pairs(sheets),
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

/// Result of the materialized saturation: the closed count read back
/// **from the final diagram**, the rounds, and the materialized state
/// handles — the per-round diagrams of issue #388.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DiagrammedClosure {
    /// Closed count: the circular trace of the final chain's sheets.
    pub count: u64,
    /// Saturation iterations, confirming ones included, summed over the
    /// sets of the chain.
    pub rounds: u64,
    /// Handle of the final materialized state.
    pub state: NodeId,
    /// Per-set fixpoint handles, one per event set of the chain (the
    /// numbered rounds of the trace restart at each set).
    pub states: Vec<NodeId>,
}

/// Saturates the stored tower with the state **materialized per round**:
/// the coupling seed (every philosopher thinks) is interned as the
/// initial chain of sheet nodes, then every event set of the chain is
/// saturated **through the core `saturate`** — each iteration is one
/// simultaneous firing of the set over the whole ring, the union merges
/// the sheets pairwise (the accumulated pairs), and the fixpoint is
/// detected by handle identity (the image chain equals the current one
/// under hash-consing). Each round emits the core `Round` event with the
/// **real** `before`/`image`/`after` handles, the real pool `growth`,
/// the active fragments (one level per position) and the modified ones.
/// The runner persists across the rounds (core M2-5): the memo entries
/// of the untouched fragments are never invalidated, whatever the round.
pub fn saturate_diagrammed(
    pool: &mut Arena<TowerController>,
    root: NodeId,
    anchor: Anchor,
    observer: Option<Arc<dyn Observer>>,
) -> Result<DiagrammedClosure, ErrorHsdd> {
    let TowerRead {
        ring_size: n,
        sets,
        pcs,
    } = fold_tower(pool, root)?;
    let size = pcs.len();
    let levels: Vec<Level> = (0..n)
        .map(|position| Level(LEVEL_COUPLING_BASE + position as u32))
        .collect();
    let policy = PolicySaturation::Static(levels);
    let keys = coupling_keys(size);
    // Seed: the initial state interned as the first materialized diagram.
    let matrices = seed_matrices(n as usize, &pcs);
    let mut state = intern_coupling_chain(pool, anchor, &matrices, size)?;
    let mut states = Vec::with_capacity(sets.len());
    let mut rounds = 0u64;
    for &set in &sets {
        let step = FireSetStep {
            set,
            alphabet: pcs.clone(),
            fingerprint: fire_set_fingerprint(set),
        };
        let mut params = SaturationParameters {
            transduction: &step,
            policy: &policy,
            pool,
            keys: &keys,
            anchor,
            initial: state,
            number_levels: n as u32,
            observer: observer.clone(),
        };
        let result = core_saturate(&mut params)?;
        rounds += u64::from(result.iterations);
        state = result.node;
        states.push(state);
    }
    // The count is read back FROM the materialized diagram — the chain
    // is the state, the state is countable.
    let matrices = read_coupling(pool, state, n as usize, size)?;
    let count = trace_count(&matrices, size);
    Ok(DiagrammedClosure {
        count,
        rounds,
        state,
        states,
    })
}
