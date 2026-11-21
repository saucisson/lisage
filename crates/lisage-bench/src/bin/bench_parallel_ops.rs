//! P1-1 runner (issue #139): multi-query workload — k independent counting
//! transductions on roots of the same read-shared input pool (the
//! philosophers R_n, golden thread), run sequentially vs in parallel
//! (rayon, INV10). Prints one JSON line with both wall times and the
//! speedup.
//!
//! Usage: `bench_parallel_ops [n] [ops]` (defaults n = 8, ops = 16).

use std::collections::BTreeMap;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::parallel_ops::{IndependentOp, run_independent_par, run_independent_seq};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::{Atom, Value};

const ANCHOR: Anchor = Anchor(0);
const THINK: u64 = 0;
const HUNGRY: u64 = 1;
const EAT: u64 = 2;
const KEY_STRIDE: u64 = 2;
const LEFT_KEY_OFFSET: u64 = 1;

type PoolRef = Arena<ReferenceController>;

/// Counting transduction: synthesized attribute = number of accepting
/// paths.
#[derive(Clone)]
struct CountPaths;

impl Transduction for CountPaths {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        7002
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}
    fn phi_aggr(&self, _level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).sum()
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<()>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memory::EffetMemory::new(),
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, u64) {
        (
            pool.intern(anchor, Form::Terminal(payload)),
            u64::from(payload),
        )
    }
}

/// Schema of the ring unrolled in interleaving (golden thread § 2).
fn ring_schema(n: u64) -> Schema {
    Schema::Map(
        (0..KEY_STRIDE * n).map(Atom).collect(),
        Domain::new(vec![Atom(THINK), Atom(HUNGRY), Atom(EAT)]),
    )
}

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn left_fork(pc: &[u64], n: usize, i: usize) -> u64 {
    const FREE: u64 = 0;
    const HELD: u64 = 1;
    const PREVIOUS: u64 = 2;
    if pc[i] == EAT {
        HELD
    } else if pc[(i + n - 1) % n] == EAT {
        PREVIOUS
    } else {
        FREE
    }
}

/// Enumerates the reachable circular words (no two adjacent eaters).
fn reachable_words(n: usize) -> Vec<Vec<u64>> {
    let mut result = Vec::new();
    let mut word = vec![THINK; n];
    loop {
        let circular_ok = (0..n).all(|i| !(word[i] == EAT && word[(i + 1) % n] == EAT));
        if circular_ok {
            result.push(word.clone());
        }
        let mut i = n;
        loop {
            if i == 0 {
                return result;
            }
            i -= 1;
            if word[i] < EAT {
                word[i] += 1;
                break;
            }
            word[i] = THINK;
        }
    }
}

/// Compiles one extended state into a member map.
fn compile_member(pool: &mut PoolRef, schema: &Schema, word: &[u64], n: u64) -> NodeId {
    let map: BTreeMap<Value, Value> = (0..n)
        .flat_map(|i| {
            let i = i as usize;
            [
                (KEY_STRIDE * i as u64, word[i]),
                (
                    KEY_STRIDE * i as u64 + LEFT_KEY_OFFSET,
                    left_fork(word, n as usize, i),
                ),
            ]
        })
        .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
        .collect();
    compiler(pool, ANCHOR, schema, &Value::Map(map)).unwrap()
}

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8".to_string())
        .parse()
        .expect("n is an integer");
    let number_ops: usize = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "16".to_string())
        .parse()
        .expect("ops is an integer");
    assert!((1..=8).contains(&n), "n in 1..=8");
    assert!((1..=64).contains(&number_ops), "ops in 1..=64");

    let schema = ring_schema(n);
    let keys = SchemaKeys::new(schema.clone());
    let mut input = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let words = reachable_words(n as usize);
    let mut roots = Vec::new();
    let mut root = input.intern(ANCHOR, Form::Terminal(false));
    for word in &words {
        let member = compile_member(&mut input, &schema, word, n);
        root = lisage_core::pli::union(&mut input, &keys, ANCHOR, root, member).unwrap();
        roots.push(member);
    }

    // Multi-query workload: number_ops counting ops on the shared input pool
    // — the union root (the heavy query) and distinct member roots.
    let ops: Vec<IndependentOp<CountPaths>> = (0..number_ops)
        .map(|i| IndependentOp {
            transduction: CountPaths,
            root: if i % 2 == 0 {
                root
            } else {
                roots[i % roots.len()]
            },
        })
        .collect();

    let start = Instant::now();
    let seq = run_independent_seq(&input, &schema, ANCHOR, &ops).unwrap();
    let seq_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();
    let par = run_independent_par(&input, &schema, ANCHOR, &ops).unwrap();
    let par_ms = start.elapsed().as_secs_f64() * 1000.0;

    // PAR-1 sanity inside the bench: same synthesized attributes.
    for ((_, _, attr_seq), (_, _, attr_par)) in seq.iter().zip(par.iter()) {
        assert_eq!(attr_seq, attr_par, "PAR-1: same attribute seq vs par");
    }

    let speedup = if par_ms > 0.0 { seq_ms / par_ms } else { 0.0 };
    println!(
        "{{\"tool\":\"lisage-parallel-ops\",\"n\":{n},\"ops\":{number_ops},\"members\":{},\"seq_ms\":{seq_ms:.3},\"par_ms\":{par_ms:.3},\"speedup\":{speedup:.3}}}",
        roots.len()
    );
}
