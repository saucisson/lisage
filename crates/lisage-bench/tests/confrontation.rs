//! Tests of the confrontation harness (ADR-0102): pinned cross-check
//! values (LCG, golden thread R_n, sliding window), typed errors,
//! deterministic JSON serialization. Time/memory measurements are not
//! pinned (they depend on the machine): only the *semantic* values
//! are — that is the explicit falsification of the harness.

use std::path::PathBuf;

use lisage_bench::confrontation::{
    ErrorConfrontation, MeasureConfrontation, TOOL, Variant, Workload, execute, serialize_line,
};
use lisage_core::workshop::Encoding;

/// `think`.
const PENSE: u64 = 0;
/// `eat`.
const EATS: u64 = 2;
/// Fork held by the current philosopher (he eats).
const TENUE: u64 = 1;
/// Fork held by the previous philosopher (he eats).
const PREVIOUS: u64 = 2;
/// |R_n| of the golden thread for n = 3..7 (verified by simulation, M4-4).
const R_EXPECTEDS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Shared nodes of R_3 (didactic workshop, fragment 1: 33 nodes).
const NODES_R3: u64 = 33;
/// Paths of the test window: 10 bases × 4 positions.
const PATHS_WINDOW: u64 = 40;
/// Nodes of the absolute window (didactic workshop, fragment 2).
const ABSOLUTE_WINDOW_NODES: u64 = 12;
/// Nodes of the relative window: a single shared form, O(1).
const NODES_WINDOW_RELATIVE: u64 = 3;
/// Satisfactions of the LCG table for n = 4 (pinned, deterministic).
const SATISFACTIONS_LCG_N4: u64 = 7;
/// Satisfactions of the LCG table for n = 12 (pinned, deterministic —
/// same LCG as the M1-8 baseline).
const SATISFACTIONS_LCG_N12: u64 = 2074;

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn fourchette(counter_programme: &[u64], number: usize, index: usize) -> u64 {
    if counter_programme[index] == EATS {
        TENUE
    } else if counter_programme[(index + number - 1) % number] == EATS {
        PREVIOUS
    } else {
        0
    }
}

/// Enumerates the extended states of R_n: circular words without adjacent
/// `eat` neighbors, derived forks, interleaved `pc_0 left_0 pc_1 …`. Same
/// algorithm as the Python orchestrator (`scripts/confrontation.py`): the
/// generation ↔ pins consistency is checked by a dedicated test.
fn states_extended(number: usize) -> Vec<Vec<u64>> {
    let mut words = Vec::new();
    let mut mot = vec![PENSE; number];
    loop {
        let circulaire_ok =
            (0..number).all(|index| !(mot[index] == EATS && mot[(index + 1) % number] == EATS));
        if circulaire_ok {
            words.push(mot.clone());
        }
        let mut index = number;
        loop {
            if index == 0 {
                return words
                    .into_iter()
                    .map(|mot| {
                        (0..number)
                            .flat_map(|i| [mot[i], fourchette(&mot, number, i)])
                            .collect()
                    })
                    .collect();
            }
            index -= 1;
            if mot[index] < EATS {
                mot[index] += 1;
                break;
            }
            mot[index] = PENSE;
        }
    }
}

/// Temporary-file counter: a unique name per call (under
/// `cargo test`, the tests of one binary run as threads of a
/// single process — the pid alone is not enough).
static COUNTER_FILES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Writes the shared extended-states file (same format as the
/// Python orchestrator: first line `n <n>`, then one state per line) and
/// returns its path.
fn write_file_states(number: u64) -> PathBuf {
    let file_index = COUNTER_FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "confrontation-test-philosophers-{number}-{}-{file_index}.txt",
        std::process::id()
    ));
    let states = states_extended(number as usize);
    let mut content = format!("n {number}\n");
    for state in states {
        let line: Vec<String> = state.iter().map(|value| value.to_string()).collect();
        content.push_str(&line.join(" "));
        content.push('\n');
    }
    std::fs::write(&path, content).expect("temporary states file written");
    path
}

/// `alea-lcg` workload: pinned satisfactions (deterministic LCG), C6
/// checked internally by the runner (independent mini-BDD), measurement
/// returned with the `bdd` variant.
#[test]
fn test_alea_lcg_satisfactions_epinglees() {
    for (n, satisfied) in [(4, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let measure = execute(&Workload::AleaLcg, &[n.to_string()], None).expect("workload run");
        assert_eq!(TOOL, "lisage");
        assert_eq!(measure.variant, Variant::Bdd);
        assert_eq!(measure.workload, Workload::AleaLcg);
        assert_eq!(measure.verifies, satisfied, "n={n}: pinned satisfactions");
        assert!(measure.nodes > 0, "n={n}: non-empty diagram");
        assert!(measure.temps_ms.is_finite(), "n={n}: measured time");
        assert!(measure.rss_max_ko > 0, "n={n}: VmHWM read");
    }
}

/// `alea-lcg` workload: rejected bounds (zero and too large), typed error.
#[test]
fn test_alea_lcg_bounds_refusees() {
    for n in [0, 25] {
        let error =
            execute(&Workload::AleaLcg, &[n.to_string()], None).expect_err("bounds rejected");
        assert!(matches!(
            error,
            ErrorConfrontation::InvalidParameters { workload, .. } if workload == "alea-lcg"
        ));
    }
}

/// `philosophers` workload: |R_n| pinned over n = 3..7, nodes of R_3 pinned
/// (33), cardinality via fold == number of states in the file.
#[test]
fn test_philosophers_thread_red_pinned() {
    for &(number, expected) in R_EXPECTEDS {
        let path = write_file_states(number);
        let parameters = vec![number.to_string(), path.display().to_string()];
        let measure = execute(&Workload::Philosophers, &parameters, None).expect("workload run");
        assert_eq!(measure.verifies, expected, "n={number}: pinned |R_n|");
        assert_eq!(measure.variant, Variant::Mdd);
        if number == 3 {
            assert_eq!(measure.nodes, NODES_R3, "R_3: 33 shared nodes");
        }
        assert!(measure.rss_max_ko > 0);
    }
}

/// `philosophers` workload: upper bound on n (anti-overflow guard
/// of the interleaved schema) — typed error, no panic.
#[test]
fn test_philosophers_bound_refused() {
    let path = write_file_states(3);
    let parameters = vec!["33".to_string(), path.display().to_string()];
    let error = execute(&Workload::Philosophers, &parameters, None).expect_err("bound rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::InvalidParameters { workload, .. } if workload == "philosophers"
    ));
}

/// `philosophers` workload: invalid file (empty, not-conforming first line,
/// inconsistent state length, out-of-domain value) → typed error,
/// never swallowed.
#[test]
fn test_philosophers_inputs_invalid() {
    let cas = [
        "",
        "x 3\n",
        "n 3\n0 0\n",
        "n 2\n0 0 0 0 0 0\n",
        "n 3\n0 0 0 0 0 7\n",
    ];
    for (number, content) in cas.iter().enumerate() {
        let uniq = COUNTER_FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-invalide-{number}-{}-{uniq}.txt",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("invalid file written");
        let parameters = vec!["3".to_string(), path.display().to_string()];
        let error = execute(&Workload::Philosophers, &parameters, None);
        assert!(error.is_err(), "invalid input rejected: {content:?}");
        assert!(
            matches!(error, Err(ErrorConfrontation::InvalidInput { .. })),
            "typed error InvalidInput, not swallowed: {error:?}"
        );
    }
}

/// `window` workload: same paths in both encodings (40 = 10×4),
/// nodes 12 (absolute) vs 3 (relative) — the relative sharing is
/// visible to the naked eye (didactic workshop, fragment 2).
#[test]
fn test_window_encodages_pinned() {
    let absolute = execute(
        &Workload::Window,
        &["4".to_string(), "10".to_string()],
        None,
    )
    .expect("absolute");
    let relative = execute(
        &Workload::Window,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        None,
    )
    .expect("relative");
    assert_eq!(absolute.verifies, PATHS_WINDOW);
    assert_eq!(relative.verifies, PATHS_WINDOW);
    assert_eq!(absolute.nodes, ABSOLUTE_WINDOW_NODES);
    assert_eq!(relative.nodes, NODES_WINDOW_RELATIVE);
    assert_eq!(absolute.encoding, Some(Encoding::Absolute));
    assert_eq!(relative.encoding, Some(Encoding::Relative));
}

/// Unknown workload (binary edge: parsed into an enum, `None`) and
/// missing parameters: typed errors.
#[test]
fn test_workloads_inconnues_refusees() {
    assert_eq!(Workload::from_name("inexistante"), None);
    assert_eq!(Workload::from_name("alea-lcg"), Some(Workload::AleaLcg));
    let error = execute(&Workload::AleaLcg, &[], None).expect_err("missing parameters");
    assert!(matches!(
        error,
        ErrorConfrontation::InvalidParameters { workload, .. } if workload == "alea-lcg"
    ));
}

/// JSON serialization: one line, fixed key order, encoding present
/// only for the window. Pinned character by character.
#[test]
fn test_serialization_line_pinned() {
    let measure = MeasureConfrontation {
        variant: Variant::Mdd,
        workload: Workload::Window,
        parameters: vec![
            ("taille".to_string(), "4".to_string()),
            ("nb_bases".to_string(), "10".to_string()),
        ],
        encoding: Some(Encoding::Relative),
        temps_ms: 1.25,
        rss_max_ko: 4096,
        nodes: 3,
        verifies: 40,
    };
    let expected = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"window\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\"encodage\":\"relatif\",\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialize_line(&measure), expected);
    let without_encoding = MeasureConfrontation {
        encoding: None,
        ..measure
    };
    let expected_without = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"window\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialize_line(&without_encoding), expected_without);
}

/// The harness's extended-state generation (same algorithm as
/// the Python orchestrator) covers the golden-thread pins: |R_n| for
/// n = 3..7, each state with 2n values in {0,1,2}.
#[test]
fn test_states_extended_consistent_with_thread_red() {
    for &(number, expected) in R_EXPECTEDS {
        let states = states_extended(number as usize);
        assert_eq!(states.len() as u64, expected, "n={number}: pinned |R_n|");
        assert!(states.iter().all(|state| {
            state.len() == 2 * number as usize && state.iter().all(|value| *value <= EATS)
        }));
    }
}

/// Narration (ADR-0100 decision 6): each workload emits an
/// `Attr`/`Synthesized` event carrying the summary — pinned by a
/// collecting observer.
#[test]
fn test_narration_workload_pinned() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttr, Observer, TraceEvent};

    #[derive(Clone)]
    struct Collector {
        events: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl Observer for Collector {
        fn on_event(&self, event: &TraceEvent) {
            self.events.lock().expect("mutex").push(event.clone());
        }
    }

    let collector = Collector {
        events: Arc::new(Mutex::new(Vec::new())),
    };
    let measure =
        execute(&Workload::AleaLcg, &["4".to_string()], Some(&collector)).expect("workload run");
    let expected = format!(
        "workload=alea-lcg nodes={} check={}",
        measure.nodes, SATISFACTIONS_LCG_N4
    );
    // Every workload emits its summary: philosophers, window, cnf and
    // pnml are pinned here too (a regression that would lose one of
    // them would fail this test).
    let path = write_file_states(3);
    let measure_philo = execute(
        &Workload::Philosophers,
        &["3".to_string(), path.display().to_string()],
        Some(&collector),
    )
    .expect("philosophers workload run");
    let measure_window = execute(
        &Workload::Window,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        Some(&collector),
    )
    .expect("window workload run");
    let measure_cnf = execute(
        &Workload::Cnf,
        &[path_model("uf20-01.cnf")],
        Some(&collector),
    )
    .expect("cnf workload run");
    let measure_pnml = execute(
        &Workload::Pnml,
        &[path_model("SwimmingPool-1.pnml")],
        Some(&collector),
    )
    .expect("pnml workload run");
    let expected_list = [
        expected,
        format!(
            "workload=philosophers nodes={} check={}",
            measure_philo.nodes, measure_philo.verifies
        ),
        format!(
            "workload=window encoding=relatif nodes={} check={}",
            measure_window.nodes, measure_window.verifies
        ),
        format!(
            "workload=cnf nodes={} check={}",
            measure_cnf.nodes, measure_cnf.verifies
        ),
        format!(
            "workload=pnml nodes={} check={}",
            measure_pnml.nodes, measure_pnml.verifies
        ),
    ];
    let events = collector.events.lock().expect("mutex").clone();
    for expected in &expected_list {
        assert!(
            events.iter().any(|e| matches!(
                e,
                TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    value,
                    ..
                } if value.render() == *expected
            )),
            "the narration event carries the summary: {expected}"
        );
    }
}

/// Path of a versioned model of `crates/lisage-bench/models/` (cargo runs
/// the tests with the crate root as working directory).
fn path_model(name: &str) -> String {
    format!("models/{name}")
}

/// Pinned cross-check values of the real loads (M7-4, issue #114): BDD
/// node count (`nodes`) and satisfying assignments (`verifies`) on the
/// versioned inputs of `models/`, cross-checked against the OxiDD witness
/// (same truth-table stdin protocol). SATLIB `uf20-01.cnf`: 20 variables,
/// 91 clauses, phase-transition 3-SAT.
const CNF_UF20_01_NODES: u64 = 51;
/// Satisfactions of `uf20-01.cnf`.
const CNF_UF20_01_SATISFAITS: u64 = 8;
/// MCC `SwimmingPool-PT-01`: 9 places → 18 variables.
const PNML_SWIMMINGPOOL_NODES: u64 = 1859;
/// Firing pairs of `SwimmingPool-PT-01`.
const PNML_SWIMMINGPOOL_SATISFAITS: u64 = 1408;
/// MCC `ERK-PT-000001`: 11 places → 22 variables (heavy).
const PNML_ERK_NODES: u64 = 9721;
/// Firing pairs of `ERK-PT-000001`.
const PNML_ERK_SATISFAITS: u64 = 8672;

/// `cnf` workload on the real SATLIB load `uf20-01.cnf`: pinned node
/// count and pinned satisfactions (cross-tool comparable), `bdd` variant.
#[test]
fn test_cnf_uf20_pinned() {
    let path = path_model("uf20-01.cnf");
    let measure = execute(&Workload::Cnf, &[path], None).expect("cnf workload run");
    assert_eq!(measure.variant, Variant::Bdd);
    assert_eq!(measure.workload, Workload::Cnf);
    assert_eq!(measure.nodes, CNF_UF20_01_NODES, "uf20-01: pinned nodes");
    assert_eq!(
        measure.verifies, CNF_UF20_01_SATISFAITS,
        "uf20-01: pinned satisfactions"
    );
    assert!(measure.rss_max_ko > 0);
}

/// `pnml` workload on the real MCC load `SwimmingPool-1.pnml`: pinned
/// node count of the transition relation (18 variables) and pinned number
/// of firing pairs.
#[test]
fn test_pnml_swimmingpool_pinned() {
    let path = path_model("SwimmingPool-1.pnml");
    let measure = execute(&Workload::Pnml, &[path], None).expect("pnml workload run");
    assert_eq!(measure.variant, Variant::Bdd);
    assert_eq!(measure.workload, Workload::Pnml);
    assert_eq!(
        measure.nodes, PNML_SWIMMINGPOOL_NODES,
        "SwimmingPool-1: pinned nodes"
    );
    assert_eq!(
        measure.verifies, PNML_SWIMMINGPOOL_SATISFAITS,
        "SwimmingPool-1: pinned firing pairs"
    );
}

/// `pnml` workload on the heavy MCC load `erk-000001.pnml` (22
/// variables, 2^22-entry truth table): pinned values, `#[ignore]`d —
/// several seconds even in release, run explicitly with
/// `cargo test -p lisage-bench -- --ignored`.
#[test]
#[ignore]
fn test_pnml_erk_pinned() {
    let path = path_model("erk-000001.pnml");
    let measure = execute(&Workload::Pnml, &[path], None).expect("pnml workload run");
    assert_eq!(measure.nodes, PNML_ERK_NODES, "ERK-000001: pinned nodes");
    assert_eq!(
        measure.verifies, PNML_ERK_SATISFAITS,
        "ERK-000001: pinned firing pairs"
    );
}

/// The real loads serialize in the standard stdin protocol of the
/// external witnesses (`cudd_count`, `oxidd_count`): first line `n`, then
/// the `2^n` values `0`/`1`, 64 per line — pinned on a small table.
#[test]
fn test_serialize_load_stdin_pinned() {
    use lisage_bench::confrontation::serialize_load_stdin;
    let table = [false, true, true, false, false, false, true, true];
    let exit = serialize_load_stdin(3, &table);
    assert_eq!(exit, "3\n0 1 1 0 0 0 1 1\n");
    // 64 values per line: n = 7 wraps after the 64th value.
    let large = vec![true; 128];
    let exit = serialize_load_stdin(7, &large);
    let lines: Vec<&str> = exit.lines().collect();
    assert_eq!(lines[0], "7");
    assert_eq!(lines[1].split(' ').count(), 64);
    assert_eq!(lines[2].split(' ').count(), 64);
}

/// `cnf` workload: malformed DIMACS inputs (missing problem line,
/// unreadable literal, unterminated clause, literal beyond `n`) and
/// unreadable file → typed errors, never swallowed.
#[test]
fn test_cnf_inputs_invalid() {
    let cas = [
        "1 2 0\n",
        "p cnf 2 1\nx 0\n",
        "p cnf 2 1\n1 2\n",
        "p cnf 2 1\n3 0\n",
        "p sat 2 1\n1 0\n",
    ];
    for (number, content) in cas.iter().enumerate() {
        let uniq = COUNTER_FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-cnf-invalide-{number}-{}-{uniq}.cnf",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("invalid CNF written");
        let error = execute(&Workload::Cnf, &[path.display().to_string()], None);
        assert!(
            matches!(error, Err(ErrorConfrontation::InvalidInput { .. })),
            "typed error InvalidInput: {error:?}"
        );
    }
    let error = execute(&Workload::Cnf, &["models/inexistant.cnf".to_string()], None)
        .expect_err("missing file rejected");
    assert!(matches!(error, ErrorConfrontation::InvalidInput { .. }));
    let error = execute(&Workload::Cnf, &[], None).expect_err("missing parameter rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::InvalidParameters { workload, .. } if workload == "cnf"
    ));
}

/// `pnml` workload: a synthetic 13-place net exceeds the 2p ≤ 24 guard →
/// typed error, no panic; unknown names and the new names at the binary
/// edge.
#[test]
fn test_pnml_bound_refused_and_names() {
    assert_eq!(Workload::from_name("cnf"), Some(Workload::Cnf));
    assert_eq!(Workload::from_name("pnml"), Some(Workload::Pnml));
    let mut content = String::from(
        "<?xml version=\"1.0\"?><pnml><net id=\"t\" \
         type=\"http://www.pnml.org/version-2009/grammar/ptnet\"><page id=\"p0\">",
    );
    for i in 0..13 {
        content.push_str(&format!("<place id=\"p{i}\"/>"));
    }
    content.push_str(
        "<transition id=\"t0\"/><arc id=\"a0\" source=\"p0\" target=\"t0\"/>\
                      <arc id=\"a1\" source=\"t0\" target=\"p1\"/></page></net></pnml>",
    );
    let uniq = COUNTER_FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "confrontation-test-pnml-13places-{}-{uniq}.pnml",
        std::process::id()
    ));
    std::fs::write(&path, content).expect("13-place PNML written");
    let error =
        execute(&Workload::Pnml, &[path.display().to_string()], None).expect_err("bound rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::InvalidParameters { workload, .. } if workload == "pnml"
    ));
    let error = execute(&Workload::Pnml, &[], None).expect_err("missing parameter rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::InvalidParameters { workload, .. } if workload == "pnml"
    ));
}

/// Property (ADR-0012 property tier): on deterministic pseudo-random CNF
/// loads, the workload's cross-check value equals an independent
/// brute-force satisfaction count computed in the test (the LCG of the
/// `alea-lcg` workload drives the generation).
#[test]
fn test_cnf_property_verifies_against_brute_force() {
    use lisage_bench::confrontation::{LCG_INCREMENT, LCG_MULTIPLIER, SEED_LCG};
    let mut state = SEED_LCG;
    let mut next_bit = move || {
        state = state
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        (state >> 63) == 1
    };
    for trial in 0..8 {
        let n = 3 + (trial % 4) as u32; // 3..6 variables
        let m = 2 + trial as usize * 3; // 2..23 clauses
        let mut clauses: Vec<Vec<i64>> = Vec::new();
        let mut content = format!("p cnf {n} {m}\n");
        for _ in 0..m {
            let mut clause: Vec<i64> = Vec::new();
            for _ in 0..3 {
                let variable = 1 + (next_bit() as u32) * n / 2 + (next_bit() as u32) * (n - n / 2);
                let variable = 1 + (variable - 1) % n;
                clause.push(if next_bit() {
                    variable as i64
                } else {
                    -(variable as i64)
                });
            }
            content.push_str(
                &clause
                    .iter()
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            content.push_str(" 0\n");
            clauses.push(clause);
        }
        let uniq = COUNTER_FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-cnf-property-{trial}-{}-{uniq}.cnf",
            std::process::id()
        ));
        std::fs::write(&path, &content).expect("random CNF written");
        let measure =
            execute(&Workload::Cnf, &[path.display().to_string()], None).expect("cnf workload run");
        let expected = (0..(1u64 << n))
            .filter(|index| {
                clauses.iter().all(|clause| {
                    clause.iter().any(|&litteral| {
                        let bit = (index >> (litteral.unsigned_abs() - 1)) & 1 == 1;
                        if litteral > 0 { bit } else { !bit }
                    })
                })
            })
            .count() as u64;
        assert_eq!(
            measure.verifies, expected,
            "trial {trial}: verifies == brute-force count"
        );
        assert!(measure.nodes > 0);
    }
}

/// `serialize_load_from_file` (the `dump-load` feed of the
/// witnesses): the serialized load of a real input starts with `n` and
/// carries exactly `2^n` values; not-file workloads are rejected.
#[test]
fn test_serialize_load_from_file_pinned() {
    use lisage_bench::confrontation::serialize_load_from_file;
    for (workload, name, n) in [
        (Workload::Cnf, "uf20-01.cnf", 20u32),
        (Workload::Pnml, "SwimmingPool-1.pnml", 18),
    ] {
        let exit =
            serialize_load_from_file(&workload, &path_model(name)).expect("charge serialized");
        let mut lines = exit.lines();
        assert_eq!(lines.next(), Some(n.to_string().as_str()));
        let values: usize = lines
            .flat_map(|l| l.split(' '))
            .filter(|t| !t.is_empty())
            .count();
        assert_eq!(values, 1usize << n, "{name}: 2^n values");
        assert!(
            exit.bytes()
                .all(|b| b.is_ascii_digit() || b == b' ' || b == b'\n')
        );
    }
    let error = serialize_load_from_file(&Workload::AleaLcg, "models/uf20-01.cnf")
        .expect_err("non-numeric alea-lcg argument rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::InvalidParameters { workload, .. } if workload == "alea-lcg"
    ));
}

/// `dump-load alea-lcg <n>` (MEM-7 pilot feed of the witnesses): the
/// serialized load is the deterministic LCG table — first line `n`,
/// then exactly `2^n` values `0`/`1`, with the pinned number of
/// satisfactions; out-of-range `n` is a typed error.
#[test]
fn test_serialize_load_alea_lcg_pinned() {
    use lisage_bench::confrontation::serialize_load_from_file;
    for (n, satisfied) in [(4u32, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let output = serialize_load_from_file(&Workload::AleaLcg, &n.to_string())
            .expect("alea-lcg charge serialized");
        let mut lines = output.lines();
        assert_eq!(lines.next(), Some(n.to_string().as_str()));
        let values: Vec<&str> = lines
            .flat_map(|l| l.split(' '))
            .filter(|t| !t.is_empty())
            .collect();
        assert_eq!(values.len(), 1usize << n, "n={n}: 2^n values");
        let ones = values.iter().filter(|&&v| v == "1").count();
        assert_eq!(ones, satisfied as usize, "n={n}: pinned LCG satisfactions");
    }
    for n in ["0", "25", "abc"] {
        let error =
            serialize_load_from_file(&Workload::AleaLcg, n).expect_err("invalid n rejected");
        assert!(matches!(
            error,
            ErrorConfrontation::InvalidParameters { workload, .. } if workload == "alea-lcg"
        ));
    }
}
