//! Local differential witness for OxiDD (M7-2, pattern ADR-0102; full
//! measurement line since M7-5, issue #115).
//!
//! Reads on stdin: `n` (number of variables) then `2^n` values `0`/`1`
//! (truth table, same input format as `cudd_count`). Optional argv[1]:
//! the charge label serialized in the JSON line. Writes on stdout one
//! JSON line in the confrontation protocol (same key order as lisage's
//! `serialiser_ligne`): `outil`, `variante`, `charge`, `parametres`,
//! `temps_ms`, `rss_max_ko`, `noeuds`, `verifie`.
//!
//! Measured block (protocol): input reading + diagram construction +
//! verification counting. Library initialization (manager, variables) is
//! outside the block. Peak RSS is the whole-process `VmHWM` (one fresh
//! container per measurement, orchestrator convention).

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
/// Maximum number of variables pre-declared at init (the `pnml` charges use
/// 2p ≤ 24): declaring the variables upfront keeps library initialization
/// outside the measured block.
const MAX_VARIABLES: u32 = 24;

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
fn rss_max_ko() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("failed to read /proc/self/status");
    for line in status.lines() {
        if let Some(values) = line.strip_prefix("VmHWM:") {
            return values
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .expect("unexpected VmHWM line");
        }
    }
    panic!("VmHWM absent from /proc/self/status");
}

fn main() -> AllocResult<()> {
    let charge = std::env::args().nth(1).unwrap_or_else(|| "stdin".to_string());
    // Library initialization, outside the measured block.
    let manager_ref = new_manager(INNER_NODE_CAPACITY, APPLY_CACHE_CAPACITY, THREADS);
    manager_ref.with_manager_exclusive(|manager| {
        manager.add_vars(MAX_VARIABLES);
    });
    // Measured block: input reading + construction + verification counting.
    let debut = std::time::Instant::now();
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).expect("failed to read stdin");
    let mut tokens = input.split_whitespace();
    let n: u32 = tokens
        .next()
        .expect("missing variable count")
        .parse()
        .expect("invalid variable count");
    assert!(n <= MAX_VARIABLES, "n exceeds the pre-declared variables");
    let table: Vec<bool> = tokens.map(|token| token == "1").collect();
    assert_eq!(table.len(), 1usize << n, "truth table size mismatch");
    let verifie = table.iter().filter(|value| **value).count() as u64;
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
    let nodes = function.node_count();
    let temps_ms = debut.elapsed().as_secs_f64() * 1000.0;
    let rss = rss_max_ko();
    println!(
        "{{\"outil\":\"oxidd\",\"variante\":\"bdd\",\"charge\":\"{charge}\",\"parametres\":{{\"n\":\"{n}\"}},\"temps_ms\":{temps_ms:.3},\"rss_max_ko\":{rss},\"noeuds\":{nodes},\"verifie\":{verifie}}}"
    );
    Ok(())
}
