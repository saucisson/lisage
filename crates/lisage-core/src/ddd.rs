//! **DDD-difference** instance-test (backlog § 9 of the roadmap, open
//! question 9 of the spec, thread #83 after the intervals and DBM
//! fragments): *Difference Decision Diagrams* by Møller & Andersen (~2002)
//! — symbolic difference constraints (DBM) for timed automata — as a living
//! instance of the region key algebra.
//!
//! The key contract (normal form, coverage, intersection) is declared on
//! the controller side (`Dbm`, `DbmKeys`); the pool does not change. The
//! order O is understood on the region lattice: a node partitions the box
//! into convex regions, and the absent arc means the continuous "absent"
//! δ(ℓ) = empty region → default payload. Universe sweep (enumeration of
//! domain points) only compiles if the universe is bounded to enumerable
//! regions (spec Q9): diagram construction goes through explicit regions,
//! never through point enumeration — free DDD domain assumed on the
//! continuum.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool, PoolError};

use crate::dbm::Dbm;

/// Evaluates a DDD-difference on a domain point: at each node, follows the
/// arc whose region contains the point. `Ok(None)` if no arc covers the
/// point: the absent arc means δ(ℓ), the continuous "absent".
pub fn evaluate<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    root: NodeId,
    point: &[u64],
) -> Result<Option<bool>, PoolError> {
    let mut node = root;
    loop {
        match pool.resolve(node)? {
            Form::Terminal(payload) => return Ok(Some(*payload)),
            Form::Node { arcs, .. } => {
                match arcs
                    .iter()
                    .find(|(region, _)| region.contains(point))
                    .map(|(_, cible)| *cible)
                {
                    Some(cible) => node = cible,
                    None => return Ok(None),
                }
            }
        }
    }
}

/// Counts the distinct nodes reachable from `root`, terminals included.
pub fn count_nodes<P: Pool<Key = Dbm, Payload = bool>>(pool: &P, root: NodeId) -> u64 {
    let mut visite = BTreeSet::new();
    let mut pile = vec![root];
    while let Some(node) = pile.pop() {
        if !visite.insert(node) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(node) {
            pile.extend(arcs.values().copied());
        }
    }
    visite.len() as u64
}

/// Instance-test: the band `x − y ≤ 1 ∧ y − x ≤ 2` over two clocks,
/// compiled into two levels by explicit regions (no universe sweep at
/// construction):
///
/// - level 0: `x − y ≤ 1` leads to level 1, the complement `y − x ≤ −2`
///   leads to `false` (the two arcs cover the box);
/// - level 1: `y − x ≤ 2` leads to `true`; outside this region, the absent
///   arc means δ(ℓ) → `false`.
pub fn compiler_bande<P: Pool<Key = Dbm, Payload = bool>>(pool: &mut P, anchor: Anchor) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let faux = pool.intern(anchor, Form::Terminal(false));
    let level_1 = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(Dbm::new(2, &[(1, 0, 2)]), vrai)]),
        },
    );
    pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([
                (Dbm::new(2, &[(0, 1, 1)]), level_1),
                (Dbm::new(2, &[(1, 0, -2)]), faux),
            ]),
        },
    )
}
