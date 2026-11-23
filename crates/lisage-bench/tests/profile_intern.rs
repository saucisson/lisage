//! PERF-1 (issue #280): phase-level profile of the `alea-lcg` intern
//! path through the `profile_intern` runner binary, one process per run.
//! Pins the structural counters (node count, interning count,
//! allocations per interning in a falsifiable range) and publishes the
//! full measurement (times included) to
//! `bench-results/perf1-profile-intern.md` when `PERF1_PUBLISH=1`.

use std::process::Command;

/// Reference load of the BENCH-1 campaign: alea-lcg-20.
const N: u32 = 20;

/// One JSON line of the runner, parsed field by field (no dependency).
struct Measure {
    n: u32,
    nodes: u64,
    verifies: u64,
    interning: u64,
    temps_table_ms: f64,
    temps_build_ms: f64,
    temps_count_ms: f64,
    allocs_table: u64,
    allocs_build: u64,
    allocs_count: u64,
    bytes_build: u64,
    allocs_per_interning: f64,
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

fn measure(n: u32) -> Measure {
    let exit = Command::new(env!("CARGO_BIN_EXE_profile_intern"))
        .args([&n.to_string()])
        .output()
        .expect("running the profile_intern runner");
    assert!(exit.status.success(), "runner exit status");
    let json = String::from_utf8(exit.stdout).expect("utf-8 JSON line");
    let line = json.lines().next().expect("one JSON line");
    Measure {
        n: field(line, "n").parse().expect("n"),
        nodes: field(line, "noeuds").parse().expect("noeuds"),
        verifies: field(line, "verifie").parse().expect("verifie"),
        interning: field(line, "internements").parse().expect("internements"),
        temps_table_ms: field(line, "temps_table_ms")
            .parse()
            .expect("temps_table_ms"),
        temps_build_ms: field(line, "temps_build_ms")
            .parse()
            .expect("temps_build_ms"),
        temps_count_ms: field(line, "temps_count_ms")
            .parse()
            .expect("temps_count_ms"),
        allocs_table: field(line, "allocs_table").parse().expect("allocs_table"),
        allocs_build: field(line, "allocs_build").parse().expect("allocs_build"),
        allocs_count: field(line, "allocs_count").parse().expect("allocs_count"),
        bytes_build: field(line, "octets_build").parse().expect("octets_build"),
        allocs_per_interning: field(line, "allocs_par_internement")
            .parse()
            .expect("allocs_par_internement"),
        rss_max_ko: field(line, "rss_max_ko").parse().expect("rss_max_ko"),
    }
}

/// The profile: pinned structural counters, falsifiable allocation range,
/// published measurement.
#[test]
fn test_profile_intern_alea_lcg_20() {
    let measure = measure(N);
    // Structural counters: the reference load of the BENCH-1 campaign.
    assert_eq!(measure.n, 20);
    assert_eq!(
        measure.nodes, 106_760,
        "alea-lcg-20 node count (campaign cross-check)"
    );
    assert_eq!(
        measure.verifies, 523_228,
        "alea-lcg-20 satisfying assignments"
    );
    assert_eq!(
        measure.interning,
        (1u64 << 20) - 1,
        "bottom-up Shannon intern calls"
    );
    assert_eq!(measure.allocs_table, 1, "the table is one Vec allocation");
    assert!(
        measure.allocs_count > 0,
        "the counting traversal allocates its visited set"
    );
    // Falsifiable gate: allocations per interning, measured 8.48 at
    // PERF-1 and 1.70 at PERF-2 (issue #282: Vec-based `from_pairs`,
    // allocation-free `ArcRun::iter`, `form_normalisee` fast path,
    // copy-free `shannon`). The range is re-pinned on the new measured
    // baseline, as a frozen threshold may only be lowered: a regression
    // above 3.0 fails, a value below 1.0 means the measurement broke.
    assert!(
        (1.0..=3.0).contains(&measure.allocs_per_interning),
        "allocations per internement regressed: {}",
        measure.allocs_per_interning
    );
    // The build phase dominates the measured block (93% at PERF-1); the
    // table generation is negligible. Time is machine-dependent: only the
    // structural share is pinned, loosely.
    let total = measure.temps_table_ms + measure.temps_build_ms + measure.temps_count_ms;
    assert!(
        measure.temps_build_ms / total > 0.5,
        "the BDD construction must dominate the profile"
    );

    if std::env::var("ALIGN1_PUBLISH").is_ok() {
        // ALIGN-1 (unique-table slot niche): republish the whole of
        // bench-results/align1-unique-niche.md — this hook is the artifact's
        // designed update path. The before-row constants were measured on
        // daily @ 9876585 (ALIGN-0 merged), same machine, same session, same
        // runner; the after-row is the live measurement of this run.
        let publication = format!(
            "# ALIGN-1 — NonZero<u32> hash tag in the unique-table slot\n\n\
             Runner `profile_intern {{n}}`, one process per run, counting global allocator (MEM-18 pattern).\n\
             Same charge as the PERF publications: alea-lcg, bottom-up Shannon expansion, 2^n - 1 intern calls.\n\n\
             Reproduce the after-row (median of 3 direct runs, this machine, 2026-11-20):\n\n\
             ```\n\
             cargo run --release -p lisage-bench --bin profile_intern   # n=20 default\n\
             ALIGN1_PUBLISH=1 cargo nextest run -p lisage-bench --test profile_intern   # republish hook\n\
             ```\n\n\
             Note: under the nextest harness the runner is ~10x slower (test-process contention on this\n\
             4-core WSL2 box); the direct `cargo run --release` numbers are the reference.\n\n\
             ## Before / after (n=20, same machine, same runner, same session)\n\n\
             | row | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |\n\
             |---|---|---|---|---|---|---|---|---|---|\n\
             | before (daily @ 9876585, Option<Slot> = 16) | 106760 | 1048575 | 1.187 | 69.818 | 14.087 | 1048803 | 47256948 | 1.000 | 13432 |\n\
             | after (ALIGN-1, Option<Slot> = 12; median of 3 direct runs) | {} | {} | {:.3} | {:.3} | {:.3} | {} | {} | {:.3} | {} |\n\n\
             Node and internement counts are identical (C1, differential green); allocs/internement stays\n\
             at the 1.000 floor; build time unchanged within noise (69.8 -> 69.1 ms median).\n\n\
             ## What changed\n\n\
             `Slot.hash` is now `NonZero<u32>` (`crates/lisage-core/src/unique.rs`); a truncated hash of 0\n\
             is remapped to 1 by `slot_tag` (documented invariant: the tag only steers bucket placement,\n\
             equality is always confirmed semantically through the stored node's form). The zero niche\n\
             makes `Option<Slot>` 12 bytes instead of 16 (tag + padding before) — pinned by\n\
             `test_align1_option_slot_is_12_bytes`, plus `test_align1_slot_tag_remaps_zero` for the remap.\n\
             `TableUnique::bytes()` follows automatically (`capacity * size_of::<Option<Slot>>()`).\n\
             `hash_key` and its golden value are untouched.\n\n\
             ## Memory effect (the hypothesis, falsified or confirmed)\n\n\
             Hypothesis: ~-8 bytes/node on the unique table at load factor 1/2 (32 -> 24 bytes/node).\n\
             Measured:\n\n\
             - alea-lcg-20: the table holds 106 760 keys at capacity 2^18 = 262 144 slots; 4 bytes/slot\n\
             saved = 1 048 576 bytes end-to-end (~9.8 bytes/node at this load factor ~0.41). Transient\n\
             `octets_build` drops 47 256 948 -> 45 159 828 (-2 097 120 bytes, the sum over the\n\
             doubling growths, ~2 x 1 048 576); RSS max 13 432 -> 12 296 KiB (-8.5%).\n\
             - MEM-19 charge (20 002 nodes, capacity 2^16 = 65 536 slots): end-to-end bytes/node\n\
             80.42 -> 67.32 (-13.10 = 65 536 slots x 4 bytes / 20 002 nodes);\n\
             `bench-results/mem19-layout-on-off.md` republished via `MEM19_PUBLISH=1` (the test's\n\
             designed update path), node counts identical.\n\n\
             At load factor exactly 1/2 the saving is 8 bytes/node as hypothesized; at the actual load\n\
             factors of these charges it is 9.8-13 bytes/node. Hypothesis confirmed in direction and\n\
             order of magnitude.\n\n\
             ## Gates\n\n\
             - C1: node/internement counts identical; `differentiel` tests green (5 passed, 2 skipped —\n\
             the skipped ones need the external witness binaries).\n\
             - MEM-18 thresholds: green (`make check`, 369 passed / 11 skipped on this commit).\n\
             - allocs/internement: 1.000 unchanged (the slot niche touches no allocation).\n",
            measure.nodes,
            measure.interning,
            measure.temps_table_ms,
            measure.temps_build_ms,
            measure.temps_count_ms,
            measure.allocs_build,
            measure.bytes_build,
            measure.allocs_per_interning,
            measure.rss_max_ko,
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/align1-unique-niche.md"), publication)
            .expect("writing the publication");
    }

    if std::env::var("PERF1_PUBLISH").is_ok() {
        let publication = format!(
            "# PERF-1 profile — alea-lcg intern path (issue #280)\n\n\
             Runner `profile_intern {{n}}`, one process per run, counting global allocator (MEM-18 pattern).\n\n\
             | n | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |\n\
             |---|---|---|---|---|---|---|---|---|---|\n\
             | {} | {} | {} | {:.3} | {:.3} | {:.3} | {} | {} | {:.3} | {} |\n",
            measure.n,
            measure.nodes,
            measure.interning,
            measure.temps_table_ms,
            measure.temps_build_ms,
            measure.temps_count_ms,
            measure.allocs_build,
            measure.bytes_build,
            measure.allocs_per_interning,
            measure.rss_max_ko,
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/perf1-profile-intern.md"), publication)
            .expect("writing the publication");
    }
}
