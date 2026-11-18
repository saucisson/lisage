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
struct Mesure {
    layout: String,
    nodes: u64,
    bytes_par_noeud: f64,
    octets_arenas_par_noeud: f64,
    temps_ms: f64,
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

fn mesurer(layout: &str) -> Mesure {
    let sortie = Command::new(env!("CARGO_BIN_EXE_layout"))
        .args([layout, &N.to_string()])
        .output()
        .expect("running the layout runner");
    assert!(sortie.status.success(), "runner exit status");
    let json = String::from_utf8(sortie.stdout).expect("utf-8 JSON line");
    let ligne = json.lines().next().expect("one JSON line");
    Mesure {
        layout: champ(ligne, "layout"),
        nodes: champ(ligne, "nodes").parse().expect("nodes"),
        bytes_par_noeud: champ(ligne, "bytes_par_noeud").parse().expect("bytes/node"),
        octets_arenas_par_noeud: champ(ligne, "octets_arenas_par_noeud")
            .parse()
            .expect("arena bytes/node"),
        temps_ms: champ(ligne, "temps_ms").parse().expect("temps_ms"),
        rss_max_ko: champ(ligne, "rss_max_ko").parse().expect("rss"),
    }
}

/// The confrontation: same node counts (C1), pinned factors, published
/// measurement.
#[test]
fn test_confrontation_layout_on_off() {
    let on = mesurer("on");
    let off = mesurer("off");
    assert_eq!(on.layout, "on");
    assert_eq!(off.layout, "off");
    // C1: identical node counts — only the container differs.
    assert_eq!(on.nodes, off.nodes, "C1: same node count on/off");
    assert_eq!(on.nodes, N as u64 + 2, "n nodes + 2 terminals");
    let facteur_arenas = off.octets_arenas_par_noeud / on.octets_arenas_par_noeud;
    let facteur_bout_en_bout = off.bytes_par_noeud / on.bytes_par_noeud;
    let facteur_rss = off.rss_max_ko as f64 / on.rss_max_ko as f64;
    // Pinned, falsifiable ranges (measured at MEM-19: arenas 1.67x,
    // end-to-end ~1.2x, RSS ~1.26x — constant in n).
    assert!(
        (1.5..=2.0).contains(&facteur_arenas),
        "arena factor {facteur_arenas:.3} out of the pinned range"
    );
    assert!(
        (1.1..=1.5).contains(&facteur_bout_en_bout),
        "end-to-end factor {facteur_bout_en_bout:.3} out of the pinned range"
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
        on.bytes_par_noeud,
        on.octets_arenas_par_noeud,
        on.temps_ms,
        on.rss_max_ko,
        off.nodes,
        off.bytes_par_noeud,
        off.octets_arenas_par_noeud,
        off.temps_ms,
        off.rss_max_ko,
        facteur_arenas,
        facteur_bout_en_bout,
        facteur_rss
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
        for ligne in md
            .lines()
            .filter(|l| l.contains("| on") || l.contains("| off"))
        {
            let cellules: Vec<&str> = ligne.split('|').map(str::trim).collect();
            assert!(
                commitee.lines().any(|c| {
                    let cc: Vec<&str> = c.split('|').map(str::trim).collect();
                    cc.len() == cellules.len()
                        && cc[1] == cellules[1]
                        && cc[2] == cellules[2]
                        && cc[3] == cellules[3]
                        && cc[4] == cellules[4]
                }),
                "committed publication up to date for line: {ligne}"
            );
        }
    }
}
