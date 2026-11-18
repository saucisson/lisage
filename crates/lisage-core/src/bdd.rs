//! Controller configuration in **classic BDD** (M1-8, statement C6):
//! ordered boolean variables, `0`/`1` arcs always explicit, no default key
//! designated — R-default is inactive, only R-share (hash-consing) and
//! R-collapse reductions apply. This is the reference "witness"
//! configuration for identical reproduction (C6): the node count of a
//! boolean function must be identical across any independent BDD witness
//! (internal mini-BDD, CUDD/Sylvan via FFI — see
//! `docs/spike-ffi-cudd-sylvan.md`).

use std::collections::BTreeSet;

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
}

impl BddKeys {
    /// Key algebra for `number_variables` ordered variables 0..n-1.
    pub fn new(number_variables: u32) -> Self {
        BddKeys { number_variables }
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

    fn unrank(&self, level: Level, rang: u64) -> Option<Self::Key> {
        if rang < self.cardinality(level) {
            Some(rang)
        } else {
            None
        }
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
    shannon(pool, anchor, Level(0), table)
}

/// Shannon expansion: `table` is indexed by the bits of variables
/// `level.0..n` (bit ℓ of the index = variable ℓ); even positions (bit of
/// the current variable = 0) give arc 0, odd ones arc 1.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
) -> NodeId {
    if table.len() == 1 {
        return pool.intern(anchor, Form::Terminal(table[0]));
    }
    let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
    let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
    let bas = shannon(pool, anchor, Level(level.0 + 1), &bas_table);
    let haut = shannon(pool, anchor, Level(level.0 + 1), &haut_table);
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
    let mut visite = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !visite.insert(node) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(node) {
            stack.extend(arcs.values().copied());
        }
    }
    visite.len() as u64
}

/// Evaluates the BDD on an assignment (bit ℓ = variable ℓ).
pub fn evaluate<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    affectation: u64,
) -> Result<bool, PoolError> {
    let mut node = root;
    loop {
        match pool.resolve(node)? {
            Form::Terminal(payload) => return Ok(*payload),
            Form::Node { level, arcs } => {
                let bit = (affectation >> level.0) & 1;
                node = *arcs.get(&bit).ok_or(PoolError::UnknownNode(node))?;
            }
        }
    }
}
