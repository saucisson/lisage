//! P1-2 (issue #140): bench of the output-pool merge — k pools, each a
//! copy of a random Map diagram, merged sequentially vs in parallel
//! (pairwise tree reduction, cross-pool sharing). Runs the `bench_merge`
//! binary, pins the structural fields, and publishes the baselines to
//! `bench-results/p12-merge.md` when `P12_PUBLISH=1`.

use std::process::Command;

/// Anchored criterion: n = 8 (16 keys, domain {0,1,2}), 16 pools.
const N: u32 = 8;
const POOLS: u32 = 16;

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

/// The merge bench: structural fields pinned, wall times published (never
/// tightly pinned — machine-dependent).
#[test]
fn test_bench_merge_pools() {
    let output = Command::new(env!("CARGO_BIN_EXE_bench_merge"))
        .args([&N.to_string(), &POOLS.to_string()])
        .output()
        .expect("running the bench_merge runner");
    assert!(output.status.success(), "runner exit status");
    let json = String::from_utf8(output.stdout).expect("utf-8 JSON line");
    let line = json.lines().next().expect("one JSON line");
    let n: u64 = field(line, "n").parse().expect("n");
    let pools: u64 = field(line, "pools").parse().expect("pools");
    let input_live: u64 = field(line, "input_live").parse().expect("input_live");
    let merged_live: u64 = field(line, "merged_live").parse().expect("merged_live");
    let seq_ms: f64 = field(line, "seq_ms").parse().expect("seq_ms");
    let per_ms: f64 = field(line, "par_ms").parse().expect("par_ms");
    let speedup: f64 = field(line, "speedup").parse().expect("speedup");
    assert_eq!(n, u64::from(N));
    assert_eq!(pools, u64::from(POOLS));
    assert!(input_live > 0 && merged_live > 0, "non-empty diagrams");
    assert!(
        merged_live < input_live,
        "cross-pool sharing: merged ({merged_live}) < one copy ({input_live})"
    );
    assert!(seq_ms > 0.0 && per_ms > 0.0, "both paths measured");
    assert!(speedup.is_finite(), "speedup is finite");

    if std::env::var("P12_PUBLISH").is_ok() {
        let publication = format!(
            "# P1-2 merge bench — output-pool fusion (issue #140)\n\n\
             Runner `bench_merge {{n}} {{pools}}`: {pools} output pools, each a full copy of a random\n\
             Map diagram (n = {n}: 16 keys, domain {{0,1,2}}, 4000 words, {input_live} live nodes),\n\
             merged sequentially vs in parallel (pairwise tree reduction, cross-pool sharing).\n\
             PAR-1 sanity (same merged cardinality seq vs par) is asserted inside the runner.\n\n\
             | n | pools | input live | merged live | seq (ms) | par (ms) | speedup |\n\
             |---|---|---|---|---|---|---|\n\
             | {n} | {pools} | {input_live} | {merged_live} | {seq_ms:.3} | {per_ms:.3} | {speedup:.3} |\n"
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/p12-merge.md"), publication)
            .expect("writing the publication");
    }
}
