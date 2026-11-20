//! PERF-1 (issue #280): phase-level profile of the `alea-lcg` intern
//! path through the `profile_intern` runner binary, one process per run.
//! Pins the structural counters (node count, internement count,
//! allocations per internement in a falsifiable range) and publishes the
//! full measurement (times included) to
//! `bench-results/perf1-profile-intern.md` when `PERF1_PUBLISH=1`.

use std::process::Command;

/// Reference charge of the BENCH-1 campaign: alea-lcg-20.
const N: u32 = 20;

/// One JSON line of the runner, parsed field by field (no dependency).
struct Mesure {
    n: u32,
    nodes: u64,
    verifie: u64,
    internements: u64,
    temps_table_ms: f64,
    temps_build_ms: f64,
    temps_count_ms: f64,
    allocs_table: u64,
    allocs_build: u64,
    allocs_count: u64,
    octets_build: u64,
    allocs_par_internement: f64,
    rss_max_ko: u64,
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
    let sortie = Command::new(env!("CARGO_BIN_EXE_profile_intern"))
        .args([&n.to_string()])
        .output()
        .expect("running the profile_intern runner");
    assert!(sortie.status.success(), "runner exit status");
    let json = String::from_utf8(sortie.stdout).expect("utf-8 JSON line");
    let ligne = json.lines().next().expect("one JSON line");
    Mesure {
        n: champ(ligne, "n").parse().expect("n"),
        nodes: champ(ligne, "noeuds").parse().expect("noeuds"),
        verifie: champ(ligne, "verifie").parse().expect("verifie"),
        internements: champ(ligne, "internements").parse().expect("internements"),
        temps_table_ms: champ(ligne, "temps_table_ms")
            .parse()
            .expect("temps_table_ms"),
        temps_build_ms: champ(ligne, "temps_build_ms")
            .parse()
            .expect("temps_build_ms"),
        temps_count_ms: champ(ligne, "temps_count_ms")
            .parse()
            .expect("temps_count_ms"),
        allocs_table: champ(ligne, "allocs_table").parse().expect("allocs_table"),
        allocs_build: champ(ligne, "allocs_build").parse().expect("allocs_build"),
        allocs_count: champ(ligne, "allocs_count").parse().expect("allocs_count"),
        octets_build: champ(ligne, "octets_build").parse().expect("octets_build"),
        allocs_par_internement: champ(ligne, "allocs_par_internement")
            .parse()
            .expect("allocs_par_internement"),
        rss_max_ko: champ(ligne, "rss_max_ko").parse().expect("rss_max_ko"),
    }
}

/// The profile: pinned structural counters, falsifiable allocation range,
/// published measurement.
#[test]
fn test_profile_intern_alea_lcg_20() {
    let mesure = mesurer(N);
    // Structural counters: the reference charge of the BENCH-1 campaign.
    assert_eq!(mesure.n, 20);
    assert_eq!(
        mesure.nodes, 106_760,
        "alea-lcg-20 node count (campaign cross-check)"
    );
    assert_eq!(
        mesure.verifie, 523_228,
        "alea-lcg-20 satisfying assignments"
    );
    assert_eq!(
        mesure.internements,
        (1u64 << 20) - 1,
        "bottom-up Shannon intern calls"
    );
    assert_eq!(mesure.allocs_table, 1, "the table is one Vec allocation");
    assert!(
        mesure.allocs_count > 0,
        "the counting traversal allocates its visited set"
    );
    // Falsifiable gate: allocations per internement, measured 8.48 at
    // PERF-1 and 1.70 at PERF-2 (issue #282: Vec-based `from_pairs`,
    // allocation-free `ArcRun::iter`, `form_normalisee` fast path,
    // copy-free `shannon`). The range is re-pinned on the new measured
    // baseline, as a frozen threshold may only be lowered: a regression
    // above 3.0 fails, a value below 1.0 means the measurement broke.
    assert!(
        (1.0..=3.0).contains(&mesure.allocs_par_internement),
        "allocations per internement regressed: {}",
        mesure.allocs_par_internement
    );
    // The build phase dominates the measured block (93% at PERF-1); the
    // table generation is negligible. Time is machine-dependent: only the
    // structural share is pinned, loosely.
    let total = mesure.temps_table_ms + mesure.temps_build_ms + mesure.temps_count_ms;
    assert!(
        mesure.temps_build_ms / total > 0.5,
        "the BDD construction must dominate the profile"
    );

    if std::env::var("ALIGN1_PUBLISH").is_ok() {
        // ALIGN-1 (unique-table slot niche): republish
        // bench-results/align1-unique-niche.md. The before-row constants
        // were measured on daily @ 9876585 (ALIGN-0 merged), same machine,
        // same session, same runner.
        let publication = format!(
            "# ALIGN-1 — NonZero<u32> hash tag in the unique-table slot\n\n\
             Runner `profile_intern {{n}}`, one process per run, counting global allocator (MEM-18 pattern).\n\
             Same charge as the PERF publications: alea-lcg, bottom-up Shannon expansion, 2^n - 1 intern calls.\n\n\
             ## Before / after (n=20, same machine, same runner, same session)\n\n\
             | row | noeuds | internements | temps table (ms) | temps build (ms) | temps count (ms) | allocs build | octets build | allocs/internement | rss_max_ko |\n\
             |---|---|---|---|---|---|---|---|---|---|\n\
             | before (daily @ 9876585, Option<Slot> = 16) | 106760 | 1048575 | 1.187 | 69.818 | 14.087 | 1048803 | 47256948 | 1.000 | 13432 |\n\
             | after (ALIGN-1, Option<Slot> = 12) | {} | {} | {:.3} | {:.3} | {:.3} | {} | {} | {:.3} | {} |\n\n\
             Node and internement counts are identical (C1); allocs/internement stays at the 1.000 floor.\n\
             The unique table holds 106 760 keys at capacity 2^18 slots: 4 bytes/slot saved = 262 144 bytes\n\
             end-to-end (~2.46 bytes/node at this load factor; the MEM-19 charge at 20 002 nodes shows\n\
             80.42 -> 67.32 bytes/node, i.e. -13.10 = 65 536 slots x 4 bytes / 20 002 nodes).\n",
            mesure.nodes,
            mesure.internements,
            mesure.temps_table_ms,
            mesure.temps_build_ms,
            mesure.temps_count_ms,
            mesure.allocs_build,
            mesure.octets_build,
            mesure.allocs_par_internement,
            mesure.rss_max_ko,
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
            mesure.n,
            mesure.nodes,
            mesure.internements,
            mesure.temps_table_ms,
            mesure.temps_build_ms,
            mesure.temps_count_ms,
            mesure.allocs_build,
            mesure.octets_build,
            mesure.allocs_par_internement,
            mesure.rss_max_ko,
        );
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench-results");
        std::fs::write(format!("{directory}/perf1-profile-intern.md"), publication)
            .expect("writing the publication");
    }
}
