//! P1-1 (issue #139): parallel execution of independent transductions
//! (INV10, ADR-0105): n independent operations read the same input pool
//! read-shared and each writes its own output pool. The sequential path is
//! the one-worker case; the parallel path is the same operations on a rayon
//! fork-join. PAR-1 (equivalence on the same inputs) is pinned by the PBT in
//! `lisage-bench/tests/par_equivalence.rs`.

use lisage_algebra::interface::{Anchor, NodeId};
use rayon::prelude::*;

use crate::arena::Arena;
use crate::error::CoreError;
use crate::observer::NoOpObserver;
use crate::schema::{ReferenceController, Schema};
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
