//! Micro-benchmarks (M0-5 base + M1-8 baselines): validates that criterion
//! runs in CI and anchors the classic-BDD baseline (C6).

use criterion::{Criterion, criterion_group, criterion_main};
use lisage_algebra::interface::Anchor;
use lisage_bench::witness_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compile_bdd, count_nodes};
use lisage_core::observer::NoOpObserver;

/// Number of variables of the classic-BDD baseline (M1-8).
const BASELINE_VARIABLES: u32 = 12;
/// Seed of the deterministic truth-table LCG.
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier (Knuth).
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// LCG increment.
const INCREMENT_LCG: u64 = 1442695040888963407;

fn bench_lane_a(c: &mut Criterion) {
    c.bench_function("lane_a_mock", |b| b.iter(|| lisage_bench::lane_a(41)));
}

/// M1-8 baseline (C6): deterministic truth table (LCG) over
/// `BASELINE_VARIABLES` variables, classic-BDD construction in the arena +
/// node counting; the mini-BDD witness must give the same count (internal
/// assertion of the bench).
fn bench_bdd_classique(c: &mut Criterion) {
    let table: Vec<bool> = {
        let mut state: u64 = GRAINE_LCG;
        (0..(1usize << BASELINE_VARIABLES))
            .map(|_| {
                state = state
                    .wrapping_mul(MULTIPLICATEUR_LCG)
                    .wrapping_add(INCREMENT_LCG);
                (state >> 63) == 1
            })
            .collect()
    };
    c.bench_function("bdd_classique_n12_count", |b| {
        b.iter(|| {
            let keys = BddKeys::new(BASELINE_VARIABLES);
            let mut pool = Arena::new(
                BddController::new(BASELINE_VARIABLES),
                Box::new(NoOpObserver),
            );
            let root = compile_bdd(&mut pool, Anchor(0), &keys, &table);
            let nodes = count_nodes(&pool, root);
            let mut witness = MiniBdd::new();
            let root_witness = witness.compile_table(BASELINE_VARIABLES, &table);
            assert_eq!(nodes, witness.count_nodes(root_witness));
            assert!(witness.evaluer(root_witness, 0) == table[0]);
            nodes
        })
    });
}

criterion_group!(benches, bench_lane_a, bench_bdd_classique);
criterion_main!(benches);
