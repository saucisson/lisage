//! Controller configuration in **classic BDD** (M1-8, statement C6):
//! ordered boolean variables, `0`/`1` arcs always explicit, no default key
//! designated — R-default is inactive, only R-share (hash-consing) and
//! R-collapse reductions apply. This is the reference "witness"
//! configuration for identical reproduction (C6): the node count of a
//! boolean function must be identical across any independent BDD witness
//! (internal mini-BDD, CUDD/Sylvan via FFI — see
//! `docs/spike-ffi-cudd-sylvan.md`).

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use lisage_algebra::interface::{
    Anchor, ArcRun, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

/// Cardinality of a BDD level: two arcs, `0` (low) and `1` (high).
pub const CARDINALITY_BDD: u64 = 2;

/// Key algebra of a classic BDD over `number_variables` variables: level ℓ
/// is variable ℓ, `Val(ℓ) = {0, 1}`, no δ(ℓ).
#[derive(Clone)]
pub struct BddKeys {
    number_variables: u32,
    /// Cached shared enumeration of a level's domain (ALIGN-2): the dense
    /// arc family clones one `Arc` per node, so the table is built once.
    enumeration: Arc<OnceLock<Arc<[u64]>>>,
}

impl BddKeys {
    /// Key algebra for `number_variables` ordered variables 0..n-1.
    pub fn new(number_variables: u32) -> Self {
        BddKeys {
            number_variables,
            enumeration: Arc::new(OnceLock::new()),
        }
    }

    /// Number of variables of the BDD.
    pub fn number_variables(&self) -> u32 {
        self.number_variables
    }
}

impl KeyAlgebra for BddKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITY_BDD
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }

    fn rank(&self, level: Level, key: &Self::Key) -> Option<u64> {
        if *key < self.cardinality(level) {
            Some(*key)
        } else {
            None
        }
    }

    fn unrank(&self, level: Level, rank: u64) -> Option<Self::Key> {
        if rank < self.cardinality(level) {
            Some(rank)
        } else {
            None
        }
    }

    fn enumeration(&self, _level: Level) -> Arc<[Self::Key]> {
        // Cardinality is constant over levels: one cached table for all.
        self.enumeration
            .get_or_init(|| (0..CARDINALITY_BDD).collect::<Vec<_>>().into())
            .clone()
    }

    fn sweep_levels(&self) -> u32 {
        self.number_variables
    }
}

/// Classic BDD controller: boolean payload, no default diagram designated
/// per level (R-default inactive), R-collapse only.
pub struct BddController {
    keys: BddKeys,
}

impl BddController {
    /// Controller for `number_variables` ordered variables 0..n-1.
    pub fn new(number_variables: u32) -> Self {
        BddController {
            keys: BddKeys::new(number_variables),
        }
    }
}

impl Controller for BddController {
    type Key = u64;
    type Payload = bool;
    type Keys = BddKeys;
    type Arcs = ArcRun<Self::Key>;

    const FINGERPRINT: ControllerId = ControllerId(3);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

/// Compiles a truth table into a classic BDD by top-down Shannon expansion:
/// the root is at level 0, level ℓ decides variable ℓ. `table[a]` = value
/// of the function on assignment `a` (bit ℓ of `a` = variable ℓ); the
/// length must be `2^n`.
pub fn compiler_bdd<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    keys: &BddKeys,
    table: &[bool],
) -> NodeId {
    let number_variables = keys.number_variables();
    assert_eq!(
        table.len(),
        1usize << number_variables,
        "truth table of size 2^n (internal invariant)"
    );
    shannon(pool, anchor, Level(0), table, 0, 1)
}

/// Shannon expansion: `table` is indexed by the bits of variables
/// `level.0..n` (bit ℓ of the index = variable ℓ); the sub-table of the
/// current variable is the strided window `(table, offset, stride)` —
/// PERF-2: no half-table copy per level (two `Vec` allocations per
/// interning before), arc 0 reads `offset`, arc 1 reads `offset + stride`.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
    offset: usize,
    stride: usize,
) -> NodeId {
    if stride == table.len() {
        return pool.intern(anchor, Form::Terminal(table[offset]));
    }
    let bas = shannon(pool, anchor, Level(level.0 + 1), table, offset, stride * 2);
    let haut = shannon(
        pool,
        anchor,
        Level(level.0 + 1),
        table,
        offset + stride,
        stride * 2,
    );
    pool.intern(
        anchor,
        Form::Node {
            level,
            arcs: ArcRun::from_pairs([(0, bas), (1, haut)]),
        },
    )
}

/// Counts the distinct nodes reachable from `root`, terminals included.
pub fn count_nodes<P: Pool<Key = u64, Payload = bool>>(pool: &P, root: NodeId) -> u64 {
    let mut visit = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !visit.insert(node) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(node) {
            stack.extend(arcs.values().copied());
        }
    }
    visit.len() as u64
}

/// Evaluates the BDD on an assignment (bit ℓ = variable ℓ).
pub fn evaluate<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    assignment: u64,
) -> Result<bool, PoolError> {
    let mut node = root;
    loop {
        match pool.resolve(node)? {
            Form::Terminal(payload) => return Ok(payload),
            Form::Node { level, arcs } => {
                let bit = (assignment >> level.0) & 1;
                node = *arcs.get(&bit).ok_or(PoolError::UnknownNode(node))?;
            }
        }
    }
}
