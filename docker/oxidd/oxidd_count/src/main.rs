//! Local differential witness for OxiDD (M7-2, pattern ADR-0102).
//!
//! Reads on stdin: `n` (number of variables) then `2^n` values `0`/`1`
//! (truth table, same input format as `cudd_count`). Writes on stdout the
//! OxiDD BDD node count (one integer, then a newline).

use oxidd::bdd::{BDDFunction, new_manager};
use oxidd::util::AllocResult;
use oxidd::{BooleanFunction, BooleanOperator, Function, Manager, ManagerRef};

/// Capacity of the manager's unique table (witness sizing, not a protocol
/// value): sized for the real M7-4 charges (ERK-PT-000001 needs 9 721 nodes).
const INNER_NODE_CAPACITY: usize = 1 << 20;
/// Capacity of the manager's apply cache (witness sizing, not a protocol value).
const APPLY_CACHE_CAPACITY: usize = 1 << 20;
/// Worker threads: counting is single-threaded, one worker is enough.
const THREADS: u32 = 1;

fn main() -> AllocResult<()> {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).expect("failed to read stdin");
    let mut tokens = input.split_whitespace();
    let n: u32 = tokens
        .next()
        .expect("missing variable count")
        .parse()
        .expect("invalid variable count");
    let table: Vec<bool> = tokens.map(|token| token == "1").collect();
    assert_eq!(table.len(), 1usize << n, "truth table size mismatch");

    let manager_ref = new_manager(INNER_NODE_CAPACITY, APPLY_CACHE_CAPACITY, THREADS);
    // add_vars needs exclusive access; the DD operations (var/ite) and
    // node_count take the shared lock, so they must run outside the
    // exclusive closure (otherwise they deadlock on the manager lock).
    manager_ref.with_manager_exclusive(|manager| {
        manager.add_vars(n);
    });
    let function = manager_ref.with_manager_shared(|manager| {
        let vars: Vec<BDDFunction> = (0..n)
            .map(|var| BDDFunction::var(manager, var))
            .collect::<AllocResult<Vec<_>>>()?;
        // Bottom-up Shannon expansion: layer starts as the 2^n constants,
        // then each level combines pairs with ITE(var, high, low).
        let mut layer: Vec<BDDFunction> = table
            .iter()
            .map(|&value| {
                if value {
                    BDDFunction::t(manager)
                } else {
                    BDDFunction::f(manager)
                }
            })
            .collect();
        // bit l of the table index = variable l, variable 0 at the root
        // (same convention as lisage's compiler_bdd): combine entries that
        // differ in bit `level`, i.e. at stride 2^level.
        for level in (0..n).rev() {
            let stride = 1usize << level;
            let mut next = Vec::with_capacity(stride);
            for j in 0..stride {
                next.push(vars[level as usize].ite(&layer[j + stride], &layer[j])?);
            }
            layer = next;
        }
        Ok(layer[0].clone())
    })?;
    // node_count() counts reachable nodes including terminals — the same
    // convention as lisage's count_nodes (verified locally, see PR).
    println!("{}", function.node_count());
    Ok(())
}
