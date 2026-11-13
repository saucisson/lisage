//! **DDD-difference** instance-test (backlog § 9 of the roadmap,
//! open question 9 of the spec, thread #83 after the interval
//! and DBM fragments): the *Difference Decision Diagrams* of Møller & Andersen (~2002)
//! — symbolic difference constraints (DBM) for timed automata
//! — as a living instance of the region-key algebra.
//!
//! The key contract (normal form, coverage, intersection) is declared
//! on the controller side (`Dbm`, `DbmKeys`); the pool does not change. The order O
//! is understood over the lattice of regions: a node partitions the box into
//! convex regions, and the absent edge is the continuous "absent" δ(ℓ) = empty
//! region → default payload. The universe sweep (enumeration of the points
//! of the domain) only compiles if the universe is bounded to enumerable
//! regions (spec Q9): diagram construction goes through explicit
//! regions, never through point enumeration — free DDD kingdom
//! assumed on the continuum.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool, PoolError};

use crate::dbm::Dbm;

/// Evaluates a DDD-difference on a point of the domain: at each node, follows
/// the edge whose region contains the point. `Ok(None)` if no edge
/// covers the point: the absent edge is δ(ℓ), the continuous "absent".
pub fn evaluer<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    root: NodeId,
    point: &[u64],
) -> Result<Option<bool>, PoolError> {
    let mut node = root;
    loop {
        match pool.resolve(node)? {
            Form::Terminal(payload) => return Ok(Some(*payload)),
            Form::Node { edges, .. } => {
                match edges
                    .iter()
                    .find(|(region, _)| region.contains(point))
                    .map(|(_, target)| *target)
                {
                    Some(target) => node = target,
                    None => return Ok(None),
                }
            }
        }
    }
}

/// Counts the distinct nodes reachable from `root`, terminals included.
pub fn count_nodes<P: Pool<Key = Dbm, Payload = bool>>(pool: &P, root: NodeId) -> u64 {
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

/// Instance-test: the band `x − y ≤ 1 ∧ y − x ≤ 2` over two clocks,
/// compiled in two levels by explicit regions (no universe sweep
/// at construction):
///
/// - level 0: `x − y ≤ 1` leads to level 1, the complement
///   `y − x ≤ −2` leads to `false` (the two edges cover the box);
/// - level 1: `y − x ≤ 2` leads to `true`; outside this region, the
///   absent edge is δ(ℓ) → `false`.
pub fn compile_band<P: Pool<Key = Dbm, Payload = bool>>(pool: &mut P, anchor: Anchor) -> NodeId {
    let true_node = pool.intern(anchor, Form::Terminal(true));
    let false_node = pool.intern(anchor, Form::Terminal(false));
    let level_1 = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            edges: BTreeMap::from([(Dbm::new(2, &[(1, 0, 2)]), true_node)]),
        },
    );
    pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            edges: BTreeMap::from([
                (Dbm::new(2, &[(0, 1, 1)]), level_1),
                (Dbm::new(2, &[(1, 0, -2)]), false_node),
            ]),
        },
    )
}
