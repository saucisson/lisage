//! Controller configuration in **classic BDD** (M1-8, statement C6):
//! ordered boolean variables, `0`/`1` arcs always explicit, no default key
//! designated — R-default is inactive, only R-share (hash-consing) and
//! R-collapse reductions apply. This is the reference "witness"
//! configuration for identical reproduction (C6): the node count of a
//! boolean function must be identical across any independent BDD witness
//! (internal mini-BDD, CUDD/Sylvan via FFI — see
//! `docs/spike-ffi-cudd-sylvan.md`).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

/// Cardinality of a BDD level: two arcs, `0` (low) and `1` (high).
pub const CARDINALITE_BDD: u64 = 2;

/// Key algebra of a classic BDD over `nombre_variables` variables: level ℓ
/// is variable ℓ, `Val(ℓ) = {0, 1}`, no δ(ℓ).
#[derive(Clone)]
pub struct BddKeys {
    nombre_variables: u32,
}

impl BddKeys {
    /// Key algebra for `nombre_variables` ordered variables 0..n-1.
    pub fn new(nombre_variables: u32) -> Self {
        BddKeys { nombre_variables }
    }

    /// Number of variables of the BDD.
    pub fn nombre_variables(&self) -> u32 {
        self.nombre_variables
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

/// Classic BDD controller: boolean payload, no default diagram designated
/// per level (R-default inactive), R-collapse only.
pub struct BddController {
    keys: BddKeys,
}

impl BddController {
    /// Controller for `nombre_variables` ordered variables 0..n-1.
    pub fn new(nombre_variables: u32) -> Self {
        BddController {
            keys: BddKeys::new(nombre_variables),
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
pub fn compiler_bdd<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    keys: &BddKeys,
    table: &[bool],
) -> NodeId {
    let nombre_variables = keys.nombre_variables();
    assert_eq!(
        table.len(),
        1usize << nombre_variables,
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
            arcs: BTreeMap::from([(0, bas), (1, haut)]),
        },
    )
}

/// Counts the distinct nodes reachable from `racine`, terminals included.
pub fn compter_noeuds<P: Pool<Key = u64, Payload = bool>>(pool: &P, racine: NodeId) -> u64 {
    let mut visite = BTreeSet::new();
    let mut pile = vec![racine];
    while let Some(noeud) = pile.pop() {
        if !visite.insert(noeud) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(noeud) {
            pile.extend(arcs.values().copied());
        }
    }
    visite.len() as u64
}

/// Evaluates the BDD on an assignment (bit ℓ = variable ℓ).
pub fn evaluer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    affectation: u64,
) -> Result<bool, PoolError> {
    let mut noeud = racine;
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(payload) => return Ok(*payload),
            Form::Node { level, arcs } => {
                let bit = (affectation >> level.0) & 1;
                noeud = *arcs.get(&bit).ok_or(PoolError::UnknownNode(noeud))?;
            }
        }
    }
}
