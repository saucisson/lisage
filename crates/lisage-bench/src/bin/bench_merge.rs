//! P1-2 runner (issue #140): merge of k output pools — sequential fold vs
//! parallel pairwise tree reduction (merge at barriers), with cross-pool
//! sharing. The k pools are the outputs of k independent counting
//! transductions on the philosophers R_n (golden thread). Prints one JSON
//! line with both wall times and the speedup.
//!
//! Usage: `bench_merge [n] [pools]` (defaults n = 8, pools = 16).

use std::collections::BTreeMap;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::parallel_ops::{merge_pools_parallel, merge_pools_seq};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

const ANCHOR: Anchor = Anchor(0);
/// LCG multiplier of the deterministic word generator.
const LCG_MULTIPLIER: u64 = 6364136223846793005;
/// LCG increment of the deterministic word generator.
const LCG_INCREMENT: u64 = 1442695040888963407;
/// Seed of the deterministic word generator.
const LCG_SEED: u64 = 0x5EED;
/// Number of random words in the merge input diagram.
const NUMBER_WORDS: usize = 4000;
/// Domain size of the random words ({0, 1, 2}).
const DOMAIN_SIZE: u64 = 3;

type PoolRef = Arena<ReferenceController>;

/// Deterministic pseudo-random words over the domain {0,1,2} (LCG).
fn random_words(number_words: usize, keys: u64, seed: u64) -> Vec<Vec<u64>> {
    let mut state = seed;
    let mut next = move || {
        state = state
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        (state >> 33) % DOMAIN_SIZE
    };
    (0..number_words)
        .map(|_| (0..keys).map(|_| next()).collect())
        .collect()
}

/// Builds a random Map diagram: union of `number_words` random words over
/// `2n` interleaved keys with domain {0,1,2} — a heavy merge input.
fn build_input(n: u64, number_words: usize) -> (PoolRef, Schema, NodeId) {
    let schema = Schema::Map(
        (0..2 * n).map(Atom).collect(),
        Domain::new(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut root = pool.intern(ANCHOR, Form::Terminal(false));
    for word in random_words(number_words, 2 * n, LCG_SEED) {
        let map: BTreeMap<Value, Value> = word
            .iter()
            .enumerate()
            .map(|(i, &v)| (Value::Atom(Atom(i as u64)), Value::Atom(Atom(v))))
            .collect();
        let member = compiler(&mut pool, ANCHOR, &schema, &Value::Map(map)).unwrap();
        root = lisage_core::fold::union(&mut pool, &keys, ANCHOR, root, member).unwrap();
    }
    (pool, schema, root)
}

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8".to_string())
        .parse()
        .expect("n is an integer");
    let number_pools: usize = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "16".to_string())
        .parse()
        .expect("pools is an integer");
    assert!((1..=8).contains(&n), "n in 1..=8");
    assert!((1..=64).contains(&number_pools), "pools in 1..=64");

    let (input, schema, union_root) = build_input(n, NUMBER_WORDS);

    // k output pools, each a full copy of the input diagram (cross-pool
    // sharing at merge). Built twice: the merge functions consume them.
    let make = || {
        let mut pools = Vec::new();
        let mut roots = Vec::new();
        for _ in 0..number_pools {
            let mut pool = Arena::new(
                ReferenceController::new(schema.clone()),
                Box::new(NoOpObserver),
            );
            let root = lisage_core::parallel_ops::copy_into(&input, &mut pool, ANCHOR, union_root)
                .unwrap();
            pools.push(pool);
            roots.push(root);
        }
        (pools, roots)
    };
    let (pools_seq, roots_seq) = make();
    let (pools_parallel, roots_parallel) = make();

    let start = Instant::now();
    let (merged_seq, root_seq) = merge_pools_seq(&schema, ANCHOR, &pools_seq, &roots_seq).unwrap();
    let seq_ms = start.elapsed().as_secs_f64() * 1000.0;

    let start = Instant::now();
    let (merged_parallel, root_parallel) =
        merge_pools_parallel(&schema, ANCHOR, pools_parallel, roots_parallel).unwrap();
    let per_ms = start.elapsed().as_secs_f64() * 1000.0;

    // PER-1 sanity: same merged cardinality, seq vs parallel.
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let card_seq =
        lisage_core::fold::count(&merged_seq, &mut output, ANCHOR, &schema, root_seq).unwrap();
    let card_parallel = lisage_core::fold::count(
        &merged_parallel,
        &mut output,
        ANCHOR,
        &schema,
        root_parallel,
    )
    .unwrap();
    assert_eq!(
        card_seq, card_parallel,
        "PAR-1: same merged cardinality seq vs par"
    );

    let speedup = if per_ms > 0.0 { seq_ms / per_ms } else { 0.0 };
    println!(
        "{{\"tool\":\"lisage-merge\",\"n\":{n},\"pools\":{number_pools},\"input_live\":{},\"merged_live\":{},\"seq_ms\":{seq_ms:.3},\"par_ms\":{per_ms:.3},\"speedup\":{speedup:.3}}}",
        input.live(),
        merged_seq.live()
    );
}
