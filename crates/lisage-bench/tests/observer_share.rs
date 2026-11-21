//! ALIGN-3: share of the observer dispatch on the intern path, through
//! the `observer_share` runner binary, one process per run, two modes
//! (`noop` = `NoOpObserver`, `counting` = wired observer tallying every
//! event). Pins the structural counters only: identical node and
//! internement counts across modes (C1), event tallies consistent with
//! the intern path. Times are machine-dependent and are never pinned —
//! they are published in `bench-results/align3-observer-share.md` with
//! the exact reproduction command (median of 5 direct release runs per
//! mode; under the nextest harness the runner is ~10x slower from
//! test-process contention on this 4-core WSL2 box).

use std::process::Command;

/// Reference charge of the BENCH-1 campaign: alea-lcg-20.
const N: u32 = 20;

/// One JSON line of the runner, parsed field by field (no dependency).
struct Measurement {
    mode: String,
    n: u32,
    nodes: u64,
    interns: u64,
    build_time_ms: f64,
    allocations_build: u64,
    bytes_build: u64,
    events: u64,
    creations: u64,
    shares: u64,
    reductions: u64,
    peak_rss_kib: u64,
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

fn measure(mode: &str, n: u32) -> Measurement {
    let output = Command::new(env!("CARGO_BIN_EXE_observer_share"))
        .args([mode, &n.to_string()])
        .output()
        .expect("running the observer_share runner");
    assert!(output.status.success(), "runner exit status");
    let json = String::from_utf8(output.stdout).expect("utf-8 JSON line");
    let line = json.lines().next().expect("one JSON line");
    Measurement {
        mode: field(line, "mode"),
        n: field(line, "n").parse().expect("n"),
        nodes: field(line, "nodes").parse().expect("nodes"),
        interns: field(line, "interns").parse().expect("interns"),
        build_time_ms: field(line, "build_time_ms").parse().expect("build_time_ms"),
        allocations_build: field(line, "allocations_build")
            .parse()
            .expect("allocations_build"),
        bytes_build: field(line, "bytes_build").parse().expect("bytes_build"),
        events: field(line, "events").parse().expect("events"),
        creations: field(line, "creations").parse().expect("creations"),
        shares: field(line, "shares").parse().expect("shares"),
        reductions: field(line, "reductions").parse().expect("reductions"),
        peak_rss_kib: field(line, "peak_rss_kib").parse().expect("peak_rss_kib"),
    }
}

/// Both observer modes on the reference charge: identical structure
/// (C1), consistent event tallies. The time share is published, not
/// pinned.
#[test]
fn test_observer_share_alea_lcg_20() {
    let noop = measure("noop", N);
    let counting = measure("counting", N);

    // Structural counters, identical across modes (C1): the observer
    // must not change the hash-consed diagram.
    assert_eq!(noop.mode, "noop");
    assert_eq!(counting.mode, "counting");
    for measurement in [&noop, &counting] {
        assert_eq!(measurement.n, 20);
        assert_eq!(
            measurement.nodes, 106_760,
            "alea-lcg-20 node count (campaign cross-check)"
        );
        assert_eq!(
            measurement.interns,
            (1u64 << 20) - 1,
            "bottom-up Shannon intern calls"
        );
        assert!(measurement.build_time_ms > 0.0, "build phase timed");
    }
    assert_eq!(
        noop.nodes, counting.nodes,
        "C1: observer mode must not change the node count"
    );
    assert_eq!(noop.interns, counting.interns);
    assert_eq!(
        noop.allocations_build, counting.allocations_build,
        "C1: the counting observer allocates nothing on the intern path"
    );
    assert_eq!(
        noop.bytes_build, counting.bytes_build,
        "C1: identical transient bytes across observer modes"
    );
    assert!(
        noop.peak_rss_kib > 0 && counting.peak_rss_kib > 0,
        "RSS reported"
    );

    // Event tallies of the wired observer: every created node emits one
    // NodeCreated, every intern call ends in a share, a creation or a
    // reduction (a default reduction may precede a share/creation, so
    // the total can exceed the intern count).
    assert_eq!(
        counting.creations, counting.nodes,
        "one NodeCreated per created node"
    );
    assert_eq!(
        counting.events,
        counting.creations + counting.shares + counting.reductions
    );
    assert!(
        counting.events >= counting.interns,
        "at least one event per intern call"
    );
    assert!(
        counting.shares > 0,
        "the charge is mostly hash-consing hits"
    );
    // The no-op mode reports zeroed tallies (it is the neutral baseline).
    assert_eq!(noop.events, 0);
    assert_eq!(noop.creations, 0);
}
