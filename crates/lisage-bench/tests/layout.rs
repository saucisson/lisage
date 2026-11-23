//! MEM-19: layout on/off confrontation. Runs the deterministic seeded
//! BDD workload twice (inline exact-class entries vs catch-all boxed
//! entries) through the `layout` runner binary, one process per layout
//! so the peak RSS is per-layout. Asserts the node counts are identical
//! (C1: same semantic content, only the container differs) and pins the
//! measured factors; publishes the measurement to
//! `bench-results/mem19-layout-on-off.md` when `MEM19_PUBLISH=1`.

use std::process::Command;

/// Workload size of the confrontation.
const N: u32 = 20000;

/// One JSON line of the runner, parsed field by field (no dependency).
struct Measure {
    layout: String,
    nodes: u64,
    bytes_per_node: f64,
    arena_bytes_per_node: f64,
    temps_ms: f64,
    rss_max_ko: u64,
}

fn field(json: &str, key: &str) -> String {
    let marker = format!("\"{key}\":");
    let start = json.find(&marker).expect("field present") + marker.len();
    let remaining = &json[start..];
    if let Some(stripped) = remaining.strip_prefix('"') {
        let end = stripped.find('"').expect("closing quote");
        stripped[..end].to_string()
    } else {
        let end = remaining.find([',', '}']).expect("field terminator");
        remaining[..end].to_string()
    }
}

fn measure(layout: &str) -> Measure {
    let exit = Command::new(env!("CARGO_BIN_EXE_layout"))
        .args([layout, &N.to_string()])
        .output()
        .expect("running the layout runner");
    assert!(exit.status.success(), "runner exit status");
    let json = String::from_utf8(exit.stdout).expect("utf-8 JSON line");
    let line = json.lines().next().expect("one JSON line");
    Measure {
        layout: field(line, "layout"),
        nodes: field(line, "nodes").parse().expect("nodes"),
        bytes_per_node: field(line, "bytes_par_noeud").parse().expect("bytes/node"),
        arena_bytes_per_node: field(line, "octets_arenas_par_noeud")
            .parse()
            .expect("arena bytes/node"),
        temps_ms: field(line, "temps_ms").parse().expect("temps_ms"),
        rss_max_ko: field(line, "rss_max_ko").parse().expect("rss"),
    }
}

/// The confrontation: same node counts (C1), pinned factors, published
/// measurement.
#[test]
fn test_confrontation_layout_on_off() {
    let on = measure("on");
    let off = measure("off");
    assert_eq!(on.layout, "on");
    assert_eq!(off.layout, "off");
    // C1: identical node counts — only the container differs.
    assert_eq!(on.nodes, off.nodes, "C1: same node count on/off");
    assert_eq!(on.nodes, N as u64 + 2, "n nodes + 2 terminals");
    let factor_arenas = off.arena_bytes_per_node / on.arena_bytes_per_node;
    let end_to_end_factor = off.bytes_per_node / on.bytes_per_node;
    let factor_rss = off.rss_max_ko as f64 / on.rss_max_ko as f64;
    // Pinned, falsifiable ranges (measured at MEM-19: arenas 1.67x,
    // end-to-end ~1.2x, RSS ~1.26x — constant in n). Re-pinned on the
    // ALIGN-2 measured baseline: the dense-family slots went from
    // Option<(u64, NodeId)> = 24 bytes to Option<NodeId> = 8 bytes and the
    // ArcRun enum from 24 to 16 bytes, so the arena factor is now 1.33x
    // (32.00/24.00 bytes/node on the n=20000 load) — a frozen range may
    // only be re-pinned on a measured baseline, never widened to pass.
    assert!(
        (1.2..=1.6).contains(&factor_arenas),
        "arena factor {factor_arenas:.3} out of the pinned range"
    );
    assert!(
        (1.1..=1.5).contains(&end_to_end_factor),
        "end-to-end factor {end_to_end_factor:.3} out of the pinned range"
    );
    // Publication: bench-results/mem19-layout-on-off.md carries the
    // structural values; rewrite only on explicit demand so a test run
    // never dirties the repo with machine-dependent RSS/time.
    let md = format!(
        "# MEM-19: layout on/off confrontation (issue #135)\n\n\
         Seeded BDD workload, n = {N} interned degree-2 nodes, one runner process per layout.\n\n\
         | layout | nodes | bytes/node (end-to-end) | arena bytes/node | temps_ms | rss_max_ko |\n\
         |---|---|---|---|---|---|\n\
         | on (inline exact classes, MEM-16) | {} | {:.2} | {:.2} | {:.3} | {} |\n\
         | off (catch-all boxed entries) | {} | {:.2} | {:.2} | {:.3} | {} |\n\n\
         Factors: arena {:.3}x, end-to-end {:.3}x, RSS {:.3}x. Node counts identical (C1).\n\n\
         Verdict: the factor is **constant** in n (1.67x on arena slots, ~1.2x end-to-end,\n\
         ~1.26x RSS) — the x5..x12 prediction applies to the cumulative MEM-B track against\n\
         the ~420 B/node pre-MEM-B baseline (measured 420/60.75 = 6.9x), not to the layout\n         toggle alone.\n",
        on.nodes,
        on.bytes_per_node,
        on.arena_bytes_per_node,
        on.temps_ms,
        on.rss_max_ko,
        off.nodes,
        off.bytes_per_node,
        off.arena_bytes_per_node,
        off.temps_ms,
        off.rss_max_ko,
        factor_arenas,
        end_to_end_factor,
        factor_rss
    );
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
    let path = format!("{directory}/mem19-layout-on-off.md");
    if std::env::var("MEM19_PUBLISH").is_ok() {
        std::fs::create_dir_all(directory).expect("creation of bench-results");
        std::fs::write(&path, &md).expect("writing the publication");
    } else {
        let commitee = std::fs::read_to_string(&path).expect("reading the committed publication");
        // The structural lines must be up to date (nodes, bytes/node,
        // arena bytes/node, factors); RSS/time are machine-dependent.
        for line in md
            .lines()
            .filter(|l| l.contains("| on") || l.contains("| off"))
        {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            assert!(
                commitee.lines().any(|c| {
                    let cc: Vec<&str> = c.split('|').map(str::trim).collect();
                    cc.len() == cells.len()
                        && cc[1] == cells[1]
                        && cc[2] == cells[2]
                        && cc[3] == cells[3]
                        && cc[4] == cells[4]
                }),
                "committed publication up to date for line: {line}"
            );
        }
    }
}
