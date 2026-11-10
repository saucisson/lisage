//! Micro-benchs (M0-5 socle + M1-8 lignes de base) : valide que criterion
//! tourne en CI et ancre la ligne de base du BDD classique (C6).

use criterion::{Criterion, criterion_group, criterion_main};
use lisage_algebra::interface::Anchor;
use lisage_bench::temoin_bdd::MiniBdd;
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, compter_noeuds};
use lisage_core::observer::NoOpObserver;

/// Nombre de variables de la ligne de base BDD classique (M1-8).
const BASELINE_VARIABLES: u32 = 12;

fn bench_voie_a(c: &mut Criterion) {
    c.bench_function("voie_a_factice", |b| b.iter(|| lisage_bench::voie_a(41)));
}

/// Ligne de base M1-8 (C6) : table de vérité déterministe (LCG) sur
/// `BASELINE_VARIABLES` variables, construction du BDD classique dans
/// l'arène + comptage des nœuds ; le témoin mini-BDD doit donner le même
/// compte (assertion interne du bench).
fn bench_bdd_classique(c: &mut Criterion) {
    let table: Vec<bool> = {
        let mut etat: u64 = 0x9E37_79B9_7F4A_7C15;
        (0..(1usize << BASELINE_VARIABLES))
            .map(|_| {
                etat = etat
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (etat >> 63) == 1
            })
            .collect()
    };
    c.bench_function("bdd_classique_n12_compter", |b| {
        b.iter(|| {
            let keys = BddKeys::new(BASELINE_VARIABLES);
            let mut pool = Arena::new(
                BddController::new(BASELINE_VARIABLES),
                Box::new(NoOpObserver),
            );
            let racine = compiler_bdd(&mut pool, Anchor(0), &keys, &table);
            let noeuds = compter_noeuds(&pool, racine);
            let mut temoin = MiniBdd::new();
            let racine_temoin = temoin.compiler_table(BASELINE_VARIABLES, &table);
            assert_eq!(noeuds, temoin.compter_noeuds(racine_temoin));
            assert!(temoin.evaluer(racine_temoin, 0) == table[0]);
            noeuds
        })
    });
}

criterion_group!(benches, bench_voie_a, bench_bdd_classique);
criterion_main!(benches);
