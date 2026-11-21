//! P1-1 (issue #139): parallel execution of independent transductions
//! (INV10, ADR-0105): n independent operations read the same input pool
//! read-shared and each writes its own output pool. The sequential path is
//! the one-worker case; the parallel path is the same operations on a rayon
//! fork-join. PAR-1 (equivalence on the same inputs) is pinned by the PBT in
//! `lisage-bench/tests/par_equivalence.rs`.

use lisage_algebra::interface::{Anchor, Form, NodeId};
use rayon::prelude::*;

use crate::arena::Arena;
use crate::error::CoreError;
use crate::observer::NoOpObserver;
use crate::schema::{ReferenceController, Schema, SchemaKeys};
use crate::transduction::{Transducteur, Transduction};

type PoolRef = Arena<ReferenceController>;

/// One independent operation: a transduction to run on a root of the shared
/// input pool. Each operation gets its own output pool.
pub struct IndependentOp<T: Transduction> {
    /// The transduction to run.
    pub transduction: T,
    /// Root of the operation in the input pool.
    pub root: NodeId,
}

/// Runs one operation: fresh output pool, transduction run. Returns the
/// output pool (the result node is pool-local), the result node and the
/// synthesized attribute.
fn run_one<T: Transduction<Out = bool> + Clone>(
    input: &PoolRef,
    schema: &Schema,
    anchor: Anchor,
    op: &IndependentOp<T>,
) -> Result<(PoolRef, NodeId, T::S), CoreError> {
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut runner = Transducteur::new(op.transduction.clone());
    let (node, attr) = runner.run(input, &mut output, anchor, op.root)?;
    Ok((output, node, attr))
}

/// Sequential reference: the same operations, one worker (INV10). Results
/// in the input order.
pub fn run_independent_seq<T: Transduction<Out = bool> + Clone>(
    input: &PoolRef,
    schema: &Schema,
    anchor: Anchor,
    ops: &[IndependentOp<T>],
) -> Result<Vec<(PoolRef, NodeId, T::S)>, CoreError> {
    ops.iter()
        .map(|op| run_one(input, schema, anchor, op))
        .collect()
}

/// Parallel execution (rayon fork-join) of independent transductions, all
/// reading the shared input pool read-only, each in its own output pool
/// (INV10). Results in the input order.
pub fn run_independent_par<T: Transduction<Out = bool> + Clone>(
    input: &PoolRef,
    schema: &Schema,
    anchor: Anchor,
    ops: &[IndependentOp<T>],
) -> Result<Vec<(PoolRef, NodeId, T::S)>, CoreError> {
    ops.par_iter()
        .map(|op| run_one(input, schema, anchor, op))
        .collect()
}

// ---------------------------------------------------------------------------
// P1-2 (issue #140): merge of the output pools — parallel pairwise union,
// including cross-pool sharing.
// ---------------------------------------------------------------------------

use std::collections::HashMap as Map;

/// Copies a diagram from `source` into `destination` (cross-pool sharing:
/// identical subforms are interned once in the destination's unique table).
/// Returns the copied root in the destination pool.
pub fn copy_into(
    source: &PoolRef,
    destination: &mut PoolRef,
    anchor: Anchor,
    node: NodeId,
) -> Result<NodeId, CoreError> {
    let mut memo: Map<NodeId, NodeId> = Map::new();
    copy_node(source, destination, anchor, node, &mut memo)
}

fn copy_node(
    source: &PoolRef,
    destination: &mut PoolRef,
    anchor: Anchor,
    node: NodeId,
    memo: &mut Map<NodeId, NodeId>,
) -> Result<NodeId, CoreError> {
    use lisage_algebra::interface::Pool as _;
    if let Some(&copied) = memo.get(&node) {
        return Ok(copied);
    }
    let copied = match source.resolve(node)? {
        Form::Terminal(payload) => destination.intern(anchor, Form::Terminal(payload)),
        Form::Node { level, arcs } => {
            let mut new_arcs = Vec::with_capacity(arcs.len());
            for (key, target) in arcs.iter() {
                let copied_target = copy_node(source, destination, anchor, *target, memo)?;
                new_arcs.push((*key, copied_target));
            }
            destination.intern(
                anchor,
                Form::Node {
                    level,
                    arcs: lisage_algebra::interface::ArcRun::from_pairs(new_arcs),
                },
            )
        }
    };
    memo.insert(node, copied);
    Ok(copied)
}

/// Sequential merge: copies each pool's root into one fresh merge pool and
/// unions them (the one-worker reference).
pub fn merge_pools_seq(
    schema: &Schema,
    anchor: Anchor,
    pools: &[PoolRef],
    roots: &[NodeId],
) -> Result<(PoolRef, NodeId), CoreError> {
    use crate::pli::union;
    use lisage_algebra::interface::Pool as _;
    assert_eq!(pools.len(), roots.len(), "one root per pool");
    let keys = SchemaKeys::new(schema.clone());
    let mut merged = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut root = merged.intern(anchor, Form::Terminal(false));
    for (pool, &node) in pools.iter().zip(roots.iter()) {
        let copied = copy_into(pool, &mut merged, anchor, node)?;
        root = union(&mut merged, &keys, anchor, root, copied)?;
    }
    Ok((merged, root))
}

/// Parallel merge: pairwise tree reduction of the output pools (merge at
/// barriers) — each pair is merged in its own rayon task into a fresh pool,
/// until one pool remains. Cross-pool sharing happens inside every merge.
pub fn merge_pools_par(
    schema: &Schema,
    anchor: Anchor,
    pools: Vec<PoolRef>,
    roots: Vec<NodeId>,
) -> Result<(PoolRef, NodeId), CoreError> {
    use crate::pli::union;
    use lisage_algebra::interface::Pool as _;
    assert_eq!(pools.len(), roots.len());
    if pools.is_empty() {
        let mut merged = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let root = merged.intern(anchor, Form::Terminal(false));
        return Ok((merged, root));
    }
    let keys = SchemaKeys::new(schema.clone());
    let mut current: Vec<(PoolRef, NodeId)> = pools.into_iter().zip(roots).collect();
    while current.len() > 1 {
        let taken = std::mem::take(&mut current);
        let mut pairs: Vec<Vec<(PoolRef, NodeId)>> = Vec::new();
        let mut pair: Vec<(PoolRef, NodeId)> = Vec::with_capacity(2);
        for item in taken {
            pair.push(item);
            if pair.len() == 2 {
                pairs.push(std::mem::replace(&mut pair, Vec::with_capacity(2)));
            }
        }
        if !pair.is_empty() {
            pairs.push(pair);
        }
        current = pairs
            .into_par_iter()
            .map(|pair| {
                let mut merged = Arena::new(
                    ReferenceController::new(schema.clone()),
                    Box::new(NoOpObserver),
                );
                let mut root = merged.intern(anchor, Form::Terminal(false));
                for (pool, node) in &pair {
                    let copied = copy_into(pool, &mut merged, anchor, *node)?;
                    root = union(&mut merged, &keys, anchor, root, copied)?;
                }
                Ok((merged, root))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
    }
    Ok(current.pop().expect("one pool remains"))
}
