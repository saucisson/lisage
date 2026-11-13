//! Tests of the confrontation harness (ADR-0102): pinned verification
//! values (LCG, golden thread R_n, sliding window), typed errors,
//! deterministic JSON serialization. The time/memory measurements are not
//! pinned (they depend on the machine): only the *semantic* values are —
//! that is the explicit falsification of the harness.

use std::path::PathBuf;

use lisage_bench::confrontation::{
    ErrorConfrontation, MeasureConfrontation, OUTIL, Variante, Workload, execute, serialize_line,
};
use lisage_core::workshop::Encoding;

/// `think`.
const PENSE: u64 = 0;
/// `eat`.
const EATS: u64 = 2;
/// Fork held by the current philosopher (he eats).
const TENUE: u64 = 1;
/// Fork held by the previous philosopher (he eats).
const PRECEDENTE: u64 = 2;
/// |R_n| of the golden thread for n = 3..7 (verified by simulation, M4-4).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Shared nodes of R_3 (didactic workshop, fragment 1: 33 nodes).
const NODES_R3: u64 = 33;
/// Paths of the test window: 10 bases × 4 positions.
const PATHS_FENETRE: u64 = 40;
/// Nodes of the absolute window (didactic workshop, fragment 2).
const NODES_FENETRE_ABSOLUE: u64 = 12;
/// Nodes of the relative window: a single shared form, O(1).
const NODES_FENETRE_RELATIVE: u64 = 3;
/// Satisfactions of the LCG table for n = 4 (pinned, deterministic).
const SATISFACTIONS_LCG_N4: u64 = 7;
/// Satisfactions of the LCG table for n = 12 (pinned, deterministic —
/// same LCG as the M1-8 baseline).
const SATISFACTIONS_LCG_N12: u64 = 2074;

/// Fork `left_i` derived from the `pc` word (golden thread § 3).
fn fourchette(counter_program: &[u64], number: usize, indice: usize) -> u64 {
    if counter_program[indice] == EATS {
        TENUE
    } else if counter_program[(indice + number - 1) % number] == EATS {
        PRECEDENTE
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
    let mut word = vec![PENSE; number];
    loop {
        let circulaire_ok = (0..number)
            .all(|indice| !(word[indice] == EATS && word[(indice + 1) % number] == EATS));
        if circulaire_ok {
            words.push(word.clone());
        }
        let mut indice = number;
        loop {
            if indice == 0 {
                return words
                    .into_iter()
                    .map(|word| {
                        (0..number)
                            .flat_map(|i| [word[i], fourchette(&word, number, i)])
                            .collect()
                    })
                    .collect();
            }
            indice -= 1;
            if word[indice] < EATS {
                word[indice] += 1;
                break;
            }
            word[indice] = PENSE;
        }
    }
}

/// Temporary-file counter: a unique name per call (under `cargo test`, the
/// tests of one binary run as threads of a single process — the pid alone
/// is not enough).
static COUNTER_FICHIERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Writes the shared extended-state file (same format as the Python
/// orchestrator: first line `n <n>`, then one state per line) and returns
/// its path.
fn write_file_states(number: u64) -> PathBuf {
    let file_index = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
    std::fs::write(&path, content).expect("temporary state file written");
    path
}

/// Workload `random-lcg`: pinned satisfactions (deterministic LCG), C6
/// checked internally by the runner (independent mini-BDD), measurement
/// returned with the `bdd` variant.
#[test]
fn test_random_lcg_satisfactions_pinned() {
    for (n, satisfies) in [(4, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let measure =
            execute(&Workload::RandomLcg, &[n.to_string()], None).expect("workload executed");
        assert_eq!(OUTIL, "lisage");
        assert_eq!(measure.variante, Variante::Bdd);
        assert_eq!(measure.workload, Workload::RandomLcg);
        assert_eq!(measure.verifie, satisfies, "n={n}: pinned satisfactions");
        assert!(measure.nodes > 0, "n={n}: not-empty diagram");
        assert!(measure.time_ms.is_finite(), "n={n}: measured time");
        assert!(measure.rss_max_ko > 0, "n={n}: VmHWM recorded");
    }
}

/// Workload `random-lcg`: refused bounds (zero and too large), typed error.
#[test]
fn test_random_lcg_bounds_refused() {
    for n in [0, 25] {
        let error =
            execute(&Workload::RandomLcg, &[n.to_string()], None).expect_err("bounds refused");
        assert!(matches!(
            error,
            ErrorConfrontation::ParametersInvalid { workload, .. } if workload == "random-lcg"
        ));
    }
}

/// Workload `philosophers`: |R_n| pinned over n = 3..7, R_3 nodes pinned
/// (33), fold cardinality == number of states in the file.
#[test]
fn test_philosophers_thread_red_pinned() {
    for &(number, expected) in R_ATTENDUS {
        let path = write_file_states(number);
        let parameters = vec![number.to_string(), path.display().to_string()];
        let measure =
            execute(&Workload::Philosophers, &parameters, None).expect("workload executed");
        assert_eq!(measure.verifie, expected, "n={number}: |R_n| pinned");
        assert_eq!(measure.variante, Variante::Mdd);
        if number == 3 {
            assert_eq!(measure.nodes, NODES_R3, "R_3: 33 shared nodes");
        }
        assert!(measure.rss_max_ko > 0);
    }
}

/// Workload `philosophers`: upper bound on n (anti-overflow guard of the
/// interleaved schema) — typed error, no panic.
#[test]
fn test_philosophers_bound_refused() {
    let path = write_file_states(3);
    let parameters = vec!["33".to_string(), path.display().to_string()];
    let error = execute(&Workload::Philosophers, &parameters, None).expect_err("bound refused");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametersInvalid { workload, .. } if workload == "philosophers"
    ));
}

/// Workload `philosophers`: invalid file (empty, not-conforming first line,
/// inconsistent state length, value outside the domain) → typed error,
/// never swallowed.
#[test]
fn test_philosophers_inputs_invalid() {
    let case = [
        "",
        "x 3\n",
        "n 3\n0 0\n",
        "n 2\n0 0 0 0 0 0\n",
        "n 3\n0 0 0 0 0 7\n",
    ];
    for (number, content) in case.iter().enumerate() {
        let uniq = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-invalid-{number}-{}-{uniq}.txt",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("invalid file written");
        let parameters = vec!["3".to_string(), path.display().to_string()];
        let error = execute(&Workload::Philosophers, &parameters, None);
        assert!(error.is_err(), "invalid input rejected: {content:?}");
        assert!(
            matches!(error, Err(ErrorConfrontation::InputInvalid { .. })),
            "typed error InputInvalid, not swallowed: {error:?}"
        );
    }
}

/// Workload `window`: same paths in both encodings (40 = 10×4),
/// nodes 12 (absolute) vs 3 (relative) — the relative sharing is visible
/// to the naked eye (didactic workshop, fragment 2).
#[test]
fn test_window_encodings_pins() {
    let absolute = execute(
        &Workload::Window,
        &["4".to_string(), "10".to_string()],
        None,
    )
    .expect("absolute");
    let relative = execute(
        &Workload::Window,
        &["4".to_string(), "10".to_string(), "relative".to_string()],
        None,
    )
    .expect("relative");
    assert_eq!(absolute.verifie, PATHS_FENETRE);
    assert_eq!(relative.verifie, PATHS_FENETRE);
    assert_eq!(absolute.nodes, NODES_FENETRE_ABSOLUE);
    assert_eq!(relative.nodes, NODES_FENETRE_RELATIVE);
    assert_eq!(absolute.encoding, Some(Encoding::Absolute));
    assert_eq!(relative.encoding, Some(Encoding::Relative));
}

/// Unknown workload (binary edge: parsed into an enum, `None`) and
/// missing parameters: typed errors.
#[test]
fn test_workloads_unknown_refused() {
    assert_eq!(Workload::since_name("nonexistent"), None);
    assert_eq!(
        Workload::since_name("random-lcg"),
        Some(Workload::RandomLcg)
    );
    let error = execute(&Workload::RandomLcg, &[], None).expect_err("missing parameters");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametersInvalid { workload, .. } if workload == "random-lcg"
    ));
}

/// JSON serialization: one line, fixed key order, encoding present only
/// for the window. Pinned character by character.
#[test]
fn test_serialization_line_pinned() {
    let measure = MeasureConfrontation {
        variante: Variante::Mdd,
        workload: Workload::Window,
        parameters: vec![
            ("size".to_string(), "4".to_string()),
            ("count_bases".to_string(), "10".to_string()),
        ],
        encoding: Some(Encoding::Relative),
        time_ms: 1.25,
        rss_max_ko: 4096,
        nodes: 3,
        verifie: 40,
    };
    let expected = "{\"tool\":\"lisage\",\"variant\":\"mdd\",\"workload\":\"window\",\
\"parameters\":{\"size\":\"4\",\"count_bases\":\"10\"},\"encoding\":\"relative\",\
\"time_ms\":1.250,\"rss_max_ko\":4096,\"nodes\":3,\"verified\":40}";
    assert_eq!(serialize_line(&measure), expected);
    let without_encoding = MeasureConfrontation {
        encoding: None,
        ..measure
    };
    let expected_without = "{\"tool\":\"lisage\",\"variant\":\"mdd\",\"workload\":\"window\",\
\"parameters\":{\"size\":\"4\",\"count_bases\":\"10\"},\
\"time_ms\":1.250,\"rss_max_ko\":4096,\"nodes\":3,\"verified\":40}";
    assert_eq!(serialize_line(&without_encoding), expected_without);
}

/// The harness's extended-state generation (same algorithm as the Python
/// orchestrator) covers the golden-thread pins: |R_n| for
/// n = 3..7, each state with 2n values in {0,1,2}.
#[test]
fn test_states_extended_consistent_with_thread_red() {
    for &(number, expected) in R_ATTENDUS {
        let states = states_extended(number as usize);
        assert_eq!(states.len() as u64, expected, "n={number}: |R_n| pinned");
        assert!(states.iter().all(|state| {
            state.len() == 2 * number as usize && state.iter().all(|value| *value <= EATS)
        }));
    }
}

/// Narration (ADR-0100 decision 6): each workload emits an
/// `Attribute`/`Synthesize` event carrying the summary — pinned by a
/// collecting observer.
#[test]
fn test_narration_workload_pinned() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttribute, Observer, TraceEvent};

    #[derive(Clone)]
    struct Collecteur {
        events: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.events.lock().expect("mutex").push(event.clone());
        }
    }

    let collecteur = Collecteur {
        events: Arc::new(Mutex::new(Vec::new())),
    };
    let measure = execute(&Workload::RandomLcg, &["4".to_string()], Some(&collecteur))
        .expect("workload executed");
    let expected = format!(
        "workload=random-lcg nodes={} verified={}",
        measure.nodes, SATISFACTIONS_LCG_N4
    );
    // The three workloads emit their summary: philosophers and
    // window are pinned here too (a regression that would lose them
    // would fail this test).
    let path = write_file_states(3);
    let measure_philo = execute(
        &Workload::Philosophers,
        &["3".to_string(), path.display().to_string()],
        Some(&collecteur),
    )
    .expect("philosophers workload executed");
    let measure_window = execute(
        &Workload::Window,
        &["4".to_string(), "10".to_string(), "relative".to_string()],
        Some(&collecteur),
    )
    .expect("window workload executed");
    let attendus = [
        expected,
        format!(
            "workload=philosophers nodes={} verified={}",
            measure_philo.nodes, measure_philo.verifie
        ),
        format!(
            "workload=window encoding=relative nodes={} verified={}",
            measure_window.nodes, measure_window.verifie
        ),
    ];
    let events = collecteur.events.lock().expect("mutex").clone();
    for expected in &attendus {
        assert!(
            events.iter().any(|e| matches!(
                e,
                TraceEvent::Attribute {
                    channel: ChannelAttribute::Synthesize,
                    value,
                    ..
                } if value == expected
            )),
            "the narration event carries the summary: {expected}"
        );
    }
}
