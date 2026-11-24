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
//! Honest scope of the spike (next bricks of the track): transductions
//! crossing fragment boundaries (HSDD-5, issue #387), recursive saturation
//! with firing through the ring (HSDD-6, issue #388), external
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
