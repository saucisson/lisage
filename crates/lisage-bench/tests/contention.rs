//! P0-1 (issue #136): contention bench anchored on the philosophers
//! workload at n = 5 with saturation, through the `bench_contention` runner
//! binary, one process per run. Pins the structural counters (nodes,
//! cardinality, rounds, hit-rate ranges, per-level intern distribution) and
//! a generous frozen time ceiling (CI refuses gross time regressions), and
//! publishes the baselines to `bench-results/p01-contention.md` when
//! `P01_PUBLISH=1`.

use std::process::Command;

/// Anchored criterion: philosophers n = 5 (saturation n = 5+).
const N: u32 = 5;

/// Frozen time ceiling in ms for the whole runner at n = 5 (build +
/// saturation + count). Measured ~3 ms locally; the ceiling is ~10 000x
/// that: it only catches algorithmic regressions (a complexity change),
/// never machine noise — wall time is otherwise machine-dependent and only
/// published, never pinned tightly.
const SEUIL_TEMPS_TOTAL_MS: f64 = 30_000.0;

/// One JSON line of the runner, parsed field by field (no dependency).
struct Mesure {
    n: u64,
    nodes: u64,
    verifie: u64,
    rounds: u64,
    creations: u64,
    partages: u64,
    taux_hit_rshare: u64,
    memo_hits: u64,
    memo_misses: u64,
    taux_hit_memo: f64,
    interns_par_niveau: Vec<u64>,
    temps_build_ms: f64,
    temps_saturation_ms: f64,
    temps_count_ms: f64,
}

fn champ(json: &str, cle: &str) -> String {
    let marqueur = format!("\"{cle}\":");
    let debut = json.find(&marqueur).expect("field present") + marqueur.len();
    let reste = &json[debut..];
    if let Some(stripped) = reste.strip_prefix('"') {
        let fin = stripped.find('"').expect("closing quote");
        stripped[..fin].to_string()
    } else {
        let fin = reste.find([',', '}']).expect("field terminator");
        reste[..fin].to_string()
    }
}

fn mesurer(n: u32) -> Mesure {
    let sortie = Command::new(env!("CARGO_BIN_EXE_bench_contention"))
        .args([&n.to_string()])
        .output()
        .expect("running the bench_contention runner");
    assert!(sortie.status.success(), "runner exit status");
    let json = String::from_utf8(sortie.stdout).expect("utf-8 JSON line");
    let ligne = json.lines().next().expect("one JSON line");
    // Array field: read from `[` to the matching `]` (champ() stops at the
    // first comma, which is inside the array).
    let marqueur = "\"interns_par_niveau\":[";
    let debut = ligne.find(marqueur).expect("array field present") + marqueur.len();
    let fin = ligne[debut..].find(']').expect("array closing bracket") + debut;
    let interns: Vec<u64> = ligne[debut..fin]
        .split(',')
        .map(|v| v.trim().parse().expect("u64"))
        .collect();
    Mesure {
        n: champ(ligne, "n").parse().expect("n"),
        nodes: champ(ligne, "noeuds").parse().expect("noeuds"),
        verifie: champ(ligne, "verifie").parse().expect("verifie"),
        rounds: champ(ligne, "rounds").parse().expect("rounds"),
        creations: champ(ligne, "creations").parse().expect("creations"),
        partages: champ(ligne, "partages").parse().expect("partages"),
        taux_hit_rshare: champ(ligne, "taux_hit_rshare")
            .parse()
            .expect("taux_hit_rshare"),
        memo_hits: champ(ligne, "memo_hits").parse().expect("memo_hits"),
        memo_misses: champ(ligne, "memo_misses").parse().expect("memo_misses"),
        taux_hit_memo: champ(ligne, "taux_hit_memo")
            .parse()
            .expect("taux_hit_memo"),
        interns_par_niveau: interns,
        temps_build_ms: champ(ligne, "temps_build_ms")
            .parse()
            .expect("temps_build_ms"),
        temps_saturation_ms: champ(ligne, "temps_saturation_ms")
            .parse()
            .expect("temps_saturation_ms"),
        temps_count_ms: champ(ligne, "temps_count_ms")
            .parse()
            .expect("temps_count_ms"),
    }
}

/// The contention bench: pinned structural counters, hit-rate ranges,
/// per-level distribution, frozen time ceiling, published baselines.
#[test]
fn test_bench_contention_philosophers_n5() {
    let mesure = mesurer(N);
    // Structural counters: R_5 = 152 members + 1 illegal member added by
    // the saturation step; 2 rounds (add, then fixpoint).
    assert_eq!(mesure.n, 5);
    assert_eq!(mesure.verifie, 153, "|R_5| + the saturation-added member");
    assert_eq!(mesure.rounds, 2, "singleton step: add then fixpoint");
    assert_eq!(mesure.creations, mesure.nodes, "creations == live nodes");
    assert!(mesure.partages > 0, "R-share hits exist on this workload");
    // Falsifiable hit-rate ranges (measured 65% R-share, 21% memo at P0-1).
    assert!(
        (50..=80).contains(&mesure.taux_hit_rshare),
        "R-share hit rate regressed: {}",
        mesure.taux_hit_rshare
    );
    assert!(
        (0.10..=0.40).contains(&mesure.taux_hit_memo),
        "memo hit rate regressed: {}",
        mesure.taux_hit_memo
    );
    assert!(
        mesure.memo_hits > 0 && mesure.memo_misses > 0,
        "the saturation exercises the memo both ways"
    );
    // Per-level intern distribution: a Map of 2n keys has 4n levels
    // (presence + value per key), all exercised.
    assert_eq!(
        mesure.interns_par_niveau.len(),
        4 * N as usize,
        "one counter per level of the interleaved schema"
    );
    assert!(
        mesure.interns_par_niveau.iter().all(|&v| v > 0),
        "every level sees intern calls: {:?}",
        mesure.interns_par_niveau
    );
    // Frozen time ceiling: CI refuses gross time regressions (algorithmic),
    // not machine noise — see SEUIL_TEMPS_TOTAL_MS.
    let total = mesure.temps_build_ms + mesure.temps_saturation_ms + mesure.temps_count_ms;
    assert!(
        total <= SEUIL_TEMPS_TOTAL_MS,
        "REGRESSION: total time {total:.1} ms > ceiling {SEUIL_TEMPS_TOTAL_MS} ms"
    );

    if std::env::var("P01_PUBLISH").is_ok() {
        let publication = format!(
            "# P0-1 contention bench — philosophers saturation (issue #136)\n\n\
             Runner `bench_contention {{n}}`, one process per run. Anchored criterion n = 5;\n\
             the saturation step is a synthetic contention step (one fixed illegal member),\n\
             exercising rounds/memo/union on the philosophers diagram.\n\n\
             | n | noeuds | verifie | rounds | creations | partages | taux hit R-share (%) | memo hits | memo misses | taux hit memo | temps build (ms) | temps saturation (ms) | temps count (ms) |\n\
             |---|---|---|---|---|---|---|---|---|---|---|---|---|\n\
             | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.4} | {:.3} | {:.3} | {:.3} |\n\n\
             Interns per level (n = {}): {:?}\n",
            mesure.n,
            mesure.nodes,
            mesure.verifie,
            mesure.rounds,
            mesure.creations,
            mesure.partages,
            mesure.taux_hit_rshare,
            mesure.memo_hits,
            mesure.memo_misses,
            mesure.taux_hit_memo,
            mesure.temps_build_ms,
            mesure.temps_saturation_ms,
            mesure.temps_count_ms,
            mesure.n,
            mesure.interns_par_niveau,
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/p01-contention.md"), publication)
            .expect("writing the publication");
    }
}
