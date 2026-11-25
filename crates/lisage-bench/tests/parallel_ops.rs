//! P1-1 (issue #139): bench of the multi-query workload — k independent
//! counting transductions on a read-shared input pool (philosophers R_n),
//! sequential vs rayon parallel (INV10). Runs the `bench_parallel_ops`
//! binary, pins the structural fields, and publishes the baselines to
//! `bench-results/p11-parallel-ops.md` when `P11_PUBLISH=1`.

use std::process::Command;

/// Default anchored criterion: philosophers n = 8, 16 concurrent ops.
const N: u32 = 8;
const OPS: u32 = 16;

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

/// The multi-query bench: structural fields pinned, wall times published
/// (never tightly pinned — machine-dependent).
#[test]
fn test_bench_parallel_ops_multi_query() {
    let output = Command::new(env!("CARGO_BIN_EXE_bench_parallel_ops"))
        .args([&N.to_string(), &OPS.to_string()])
        .output()
        .expect("running the bench_parallel_ops runner");
    assert!(output.status.success(), "runner exit status");
    let json = String::from_utf8(output.stdout).expect("utf-8 JSON line");
    let line = json.lines().next().expect("one JSON line");
    let n: u64 = field(line, "n").parse().expect("n");
    let ops: u64 = field(line, "ops").parse().expect("ops");
    let members: u64 = field(line, "members").parse().expect("members");
    let seq_ms: f64 = field(line, "seq_ms").parse().expect("seq_ms");
    let per_ms: f64 = field(line, "par_ms").parse().expect("par_ms");
    let speedup: f64 = field(line, "speedup").parse().expect("speedup");
    assert_eq!(n, u64::from(N));
    assert_eq!(ops, u64::from(OPS));
    assert_eq!(members, 3104, "R_8 has 3104 members");
    assert!(seq_ms > 0.0 && per_ms > 0.0, "both paths measured");
    assert!(speedup > 0.0, "speedup is positive");

    if std::env::var("P11_PUBLISH").is_ok() {
        let publication = format!(
            "# P1-1 parallel ops bench — multi-query workload (issue #139)\n\n\
             Runner `bench_parallel_ops {{n}} {{ops}}`: {ops} independent counting transductions\n\
             on roots of the same read-shared input pool (philosophers R_{n}, {members} members),\n\
             sequential vs rayon parallel (INV10). PAR-1 sanity (same synthesized attributes\n\
             seq vs par) is asserted inside the runner.\n\n\
             | n | ops | members | seq (ms) | par (ms) | speedup |\n\
             |---|---|---|---|---|---|\n\
             | {n} | {ops} | {members} | {seq_ms:.3} | {per_ms:.3} | {speedup:.3} |\n"
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/p11-parallel-ops.md"), publication)
            .expect("writing the publication");
    }
}
