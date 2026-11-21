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
const TOTAL_TIME_CEILING_MS: f64 = 30_000.0;

/// One JSON line of the runner, parsed field by field (no dependency).
struct Measurement {
    n: u64,
    nodes: u64,
    verified: u64,
    rounds: u64,
    creations: u64,
    shares: u64,
    rshare_hit_rate: u64,
    memo_hits: u64,
    memo_misses: u64,
    memo_hit_rate: f64,
    interns_per_level: Vec<u64>,
    build_ms: f64,
    saturation_ms: f64,
    count_ms: f64,
}

fn field(json: &str, key: &str) -> String {
    let marker = format!("\"{key}\":");
    let start = json.find(&marker).expect("field present") + marker.len();
    let rest = &json[start..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"').expect("closing quote");
        stripped[..end].to_string()
    } else {
        let end = rest.find([',', '}']).expect("field terminator");
        rest[..end].to_string()
    }
}

fn measure(n: u32) -> Measurement {
    let output = Command::new(env!("CARGO_BIN_EXE_bench_contention"))
        .args([&n.to_string()])
        .output()
        .expect("running the bench_contention runner");
    assert!(output.status.success(), "runner exit status");
    let json = String::from_utf8(output.stdout).expect("utf-8 JSON line");
    let line = json.lines().next().expect("one JSON line");
    // Array field: read from `[` to the matching `]` (field() stops at the
    // first comma, which is inside the array).
    let marker = "\"interns_per_level\":[";
    let start = line.find(marker).expect("array field present") + marker.len();
    let end = line[start..].find(']').expect("array closing bracket") + start;
    let interns: Vec<u64> = line[start..end]
        .split(',')
        .map(|v| v.trim().parse().expect("u64"))
        .collect();
    Measurement {
        n: field(line, "n").parse().expect("n"),
        nodes: field(line, "nodes").parse().expect("nodes"),
        verified: field(line, "verified").parse().expect("verified"),
        rounds: field(line, "rounds").parse().expect("rounds"),
        creations: field(line, "creations").parse().expect("creations"),
        shares: field(line, "shares").parse().expect("shares"),
        rshare_hit_rate: field(line, "rshare_hit_rate")
            .parse()
            .expect("rshare_hit_rate"),
        memo_hits: field(line, "memo_hits").parse().expect("memo_hits"),
        memo_misses: field(line, "memo_misses").parse().expect("memo_misses"),
        memo_hit_rate: field(line, "memo_hit_rate").parse().expect("memo_hit_rate"),
        interns_per_level: interns,
        build_ms: field(line, "build_ms").parse().expect("build_ms"),
        saturation_ms: field(line, "saturation_ms").parse().expect("saturation_ms"),
        count_ms: field(line, "count_ms").parse().expect("count_ms"),
    }
}

/// The contention bench: pinned structural counters, hit-rate ranges,
/// per-level distribution, frozen time ceiling, published baselines.
#[test]
fn test_bench_contention_philosophers_n5() {
    let measurement = measure(N);
    // Structural counters: R_5 = 152 members + 1 illegal member added by
    // the saturation step; 2 rounds (add, then fixpoint).
    assert_eq!(measurement.n, 5);
    assert_eq!(
        measurement.verified, 153,
        "|R_5| + the saturation-added member"
    );
    assert_eq!(measurement.rounds, 2, "singleton step: add then fixpoint");
    assert_eq!(
        measurement.creations, measurement.nodes,
        "creations == live nodes"
    );
    assert!(
        measurement.shares > 0,
        "R-share hits exist on this workload"
    );
    // Falsifiable hit-rate ranges (measured 65% R-share, 21% memo at P0-1).
    assert!(
        (50..=80).contains(&measurement.rshare_hit_rate),
        "R-share hit rate regressed: {}",
        measurement.rshare_hit_rate
    );
    assert!(
        (0.10..=0.40).contains(&measurement.memo_hit_rate),
        "memo hit rate regressed: {}",
        measurement.memo_hit_rate
    );
    assert!(
        measurement.memo_hits > 0 && measurement.memo_misses > 0,
        "the saturation exercises the memo both ways"
    );
    // Per-level intern distribution: a Map of 2n keys has 4n levels
    // (presence + value per key), all exercised.
    assert_eq!(
        measurement.interns_per_level.len(),
        4 * N as usize,
        "one counter per level of the interleaved schema"
    );
    assert!(
        measurement.interns_per_level.iter().all(|&v| v > 0),
        "every level sees intern calls: {:?}",
        measurement.interns_per_level
    );
    // Frozen time ceiling: CI refuses gross time regressions (algorithmic),
    // not machine noise — see TOTAL_TIME_CEILING_MS.
    let total = measurement.build_ms + measurement.saturation_ms + measurement.count_ms;
    assert!(
        total <= TOTAL_TIME_CEILING_MS,
        "REGRESSION: total time {total:.1} ms > ceiling {TOTAL_TIME_CEILING_MS} ms"
    );

    if std::env::var("P01_PUBLISH").is_ok() {
        let publication = format!(
            "# P0-1 contention bench — philosophers saturation (issue #136)\n\n\
             Runner `bench_contention {{n}}`, one process per run. Anchored criterion n = 5;\n\
             the saturation step is a synthetic contention step (one fixed illegal member),\n\
             exercising rounds/memo/union on the philosophers diagram.\n\n\
             | n | nodes | verified | rounds | creations | shares | R-share hit rate (%) | memo hits | memo misses | memo hit rate | build (ms) | saturation (ms) | count (ms) |\n\
             |---|---|---|---|---|---|---|---|---|---|---|---|---|\n\
             | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.4} | {:.3} | {:.3} | {:.3} |\n\n\
             Interns per level (n = {}): {:?}\n",
            measurement.n,
            measurement.nodes,
            measurement.verified,
            measurement.rounds,
            measurement.creations,
            measurement.shares,
            measurement.rshare_hit_rate,
            measurement.memo_hits,
            measurement.memo_misses,
            measurement.memo_hit_rate,
            measurement.build_ms,
            measurement.saturation_ms,
            measurement.count_ms,
            measurement.n,
            measurement.interns_per_level,
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/p01-contention.md"), publication)
            .expect("writing the publication");
    }
}
