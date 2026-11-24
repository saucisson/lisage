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
//! Honest scope (next bricks of the track): external
//! libDDD/its-tools witness (HSDD-9, issue #391).

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

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
            ErrorHsdd::Pool(error) => write!(formatter, "pool error: {error}"),
        }
    }
}

impl std::error::Error for ErrorHsdd {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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
    if n == 0 || n > LIMIT_POSITIONS {
        return Err(ErrorHsdd::RingSizeOutOfRange(n));
    }
    for &(event, position) in events {
        if event >= EVENT_COUNT || position >= n {
            return Err(ErrorHsdd::EventOutOfRange { event, position });
        }
    }
    // Golden pc alphabet, read from the module cells.
    let mut pcs: Vec<u64> = cells.iter().map(|cell| cell.pc).collect();
    pcs.sort_unstable();
    pcs.dedup();
    let depth = events.len();
    let size = pcs.len();
    let n = n as usize;
    // pcs of every level of the chain at each position, as functions of
    // the base (level 0) pc index: `levels[position][level][base]`.
    let mut levels: Vec<Vec<Vec<Option<usize>>>> = Vec::with_capacity(n);
    for position in 0..n {
        let mut columns: Vec<Vec<Option<usize>>> = vec![(0..size).map(Some).collect()];
        for &(event, event_position) in events {
            let previous = columns.last().expect("depth invariant");
            let event_fires = event_position as usize == position;
            let column: Vec<Option<usize>> = previous
                .iter()
                .map(|&maybe| {
                    maybe.and_then(|index| {
                        if event_fires {
                            fire_pc(event, pcs[index]).and_then(|fired| pc_index(&pcs, fired))
                        } else {
                            Some(index)
                        }
                    })
                })
                .collect();
            columns.push(column);
        }
        levels.push(columns);
    }
    // Pair matrices of the folded coupling: the pair (position, position +
    // 1) of every level word must be legal (no two adjacent eaters).
    let mut matrices: Vec<Vec<Vec<u64>>> = Vec::with_capacity(n);
    for position in 0..n {
        let next = (position + 1) % n;
        let mut matrix = vec![vec![0u64; size]; size];
        for (u, _) in pcs.iter().enumerate() {
            for (v, _) in pcs.iter().enumerate() {
                let mut legal = true;
                for column in 0..=depth {
                    let (Some(current), Some(neighbour)) =
                        (levels[position][column][u], levels[next][column][v])
                    else {
                        legal = false;
                        break;
                    };
                    if pcs[current] == EAT && pcs[neighbour] == EAT {
                        legal = false;
                        break;
                    }
                }
                matrix[u][v] = u64::from(legal);
            }
        }
        matrices.push(matrix);
    }
    // Circular trace of the heterogeneous pair matrices.
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
    Ok((0..size).map(|i| product[i][i]).sum::<u128>() as u64)
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
pub fn pack_event_set(events: &[u64]) -> u64 {
    let mut set = 0u64;
    for &event in events {
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
        for (a, &left) in alphabet.iter().enumerate() {
            if matrices[before][a][u] == 0 {
                continue;
            }
            // The left neighbour eats: the `eat` guard skips the firing.
            if left == EAT && alphabet[v] == EAT {
                continue;
            }
            if matrices[before][a][v] == 0 {
                matrices[before][a][v] = 1;
                changed = true;
            }
        }
        for (b, &right) in alphabet.iter().enumerate() {
            if matrices[position][u][b] == 0 {
                continue;
            }
            // The right neighbour eats: the `eat` guard skips the firing.
            if alphabet[v] == EAT && right == EAT {
                continue;
            }
            if matrices[position][v][b] == 0 {
                matrices[position][v][b] = 1;
                changed = true;
            }
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
    let n = matrices.len();
    let mut changed = false;
    for position in 0..n {
        for event in 0..EVENT_COUNT {
            if set & (1 << event) != 0 {
                changed |= fire_position(matrices, alphabet, event, position);
            }
        }
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
    let count = (0..size).map(|i| product[i][i]).sum::<u128>() as u64;
    Ok(ClosureCount { count, rounds })
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
