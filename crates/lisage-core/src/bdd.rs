//! Controller configuration in **classic BDD** (M1-8, statement C6):
//! ordered Boolean variables, `0`/`1` edges always explicit, no designated
//! default key — R-default is inactive, only R-share (hash-consing) and
//! R-collapse reductions apply. This is the reference "witness"
//! configuration for exact reproduction (C6): the node count of a Boolean
//! function must be identical across any independent BDD witness
//! (internal mini-BDD, CUDD/Sylvan via FFI — see
//! `docs/spike-ffi-cudd-sylvan.md`).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

/// Cardinality of a BDD level: two edges, `0` (low) and `1` (high).
pub const CARDINALITE_BDD: u64 = 2;

/// Key algebra of a classic BDD over `number_variables` variables:
/// level ℓ is variable ℓ, `Val(ℓ) = {0, 1}`, no δ(ℓ).
#[derive(Clone)]
pub struct BddKeys {
    number_variables: u32,
}

impl BddKeys {
    /// Key algebra for `number_variables` variables ordered 0..n-1.
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
        CARDINALITE_BDD
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

/// Classic BDD controller: Boolean payload, no default diagram designated
/// per level (R-default inactive), R-collapse only.
pub struct BddController {
    keys: BddKeys,
}

impl BddController {
    /// Controller for `number_variables` variables ordered 0..n-1.
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
pub fn compile_bdd<P: Pool<Key = u64, Payload = bool>>(
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
/// `level.0..n` (bit ℓ of the index = variable ℓ); even positions
/// (bit of the current variable = 0) give edge 0, odd ones edge 1.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
) -> NodeId {
    if table.len() == 1 {
        return pool.intern(anchor, Form::Terminal(table[0]));
    }
    let low_table: Vec<bool> = table.iter().step_by(2).copied().collect();
    let top_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
    let low = shannon(pool, anchor, Level(level.0 + 1), &low_table);
    let top = shannon(pool, anchor, Level(level.0 + 1), &top_table);
    pool.intern(
        anchor,
        Form::Node {
            level,
            edges: BTreeMap::from([(0, low), (1, top)]),
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
        if let Ok(Form::Node { edges, .. }) = pool.resolve(node) {
            stack.extend(edges.values().copied());
        }
    }
    visit.len() as u64
}

/// Evaluates the BDD on an assignment (bit ℓ = variable ℓ).
pub fn evaluer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    affectation: u64,
) -> Result<bool, PoolError> {
    let mut node = root;
    loop {
        match pool.resolve(node)? {
            Form::Terminal(payload) => return Ok(*payload),
            Form::Node { level, edges } => {
                let bit = (affectation >> level.0) & 1;
                node = *edges.get(&bit).ok_or(PoolError::UnknownNode(node))?;
            }
        }
    }
}
