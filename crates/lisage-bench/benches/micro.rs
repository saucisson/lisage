//! Micro-bench factice (M0-5) : valide que criterion tourne en CI.

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_voie_a(c: &mut Criterion) {
    c.bench_function("voie_a_factice", |b| b.iter(|| lisage_bench::voie_a(41)));
}

criterion_group!(benches, bench_voie_a);
criterion_main!(benches);
