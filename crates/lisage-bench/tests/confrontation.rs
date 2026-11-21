//! Tests of the confrontation harness (ADR-0102): pinned cross-check
//! values (LCG, golden thread R_n, sliding window), typed errors,
//! deterministic JSON serialization. Time/memory measurements are not
//! pinned (they depend on the machine): only the *semantic* values
//! are — that is the explicit falsification of the harness.

use std::path::PathBuf;

use lisage_bench::confrontation::{
    ErrorConfrontation, MeasureConfrontation, TOOL, Variant, Workload, executer, serialiser_ligne,
};
use lisage_core::workshop::Encodage;

/// `think`.
const PENSE: u64 = 0;
/// `eat`.
const MANGE: u64 = 2;
/// Fork held by the current philosopher (he eats).
const TENUE: u64 = 1;
/// Fork held by the previous philosopher (he eats).
const PRECEDENTE: u64 = 2;
/// |R_n| of the golden thread for n = 3..7 (verified by simulation, M4-4).
const R_EXPECTEDS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Shared nodes of R_3 (didactic workshop, fragment 1: 33 nodes).
const NODES_R3: u64 = 33;
/// Paths of the test window: 10 bases × 4 positions.
const PATHS_WINDOW: u64 = 40;
/// Nodes of the absolute window (didactic workshop, fragment 2).
const NODES_WINDOW_ABSOLUE: u64 = 12;
/// Nodes of the relative window: a single shared form, O(1).
const NODES_WINDOW_RELATIVE: u64 = 3;
/// Satisfactions of the LCG table for n = 4 (pinned, deterministic).
const SATISFACTIONS_LCG_N4: u64 = 7;
/// Satisfactions of the LCG table for n = 12 (pinned, deterministic —
/// same LCG as the M1-8 baseline).
const SATISFACTIONS_LCG_N12: u64 = 2074;

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn fourchette(counter_programme: &[u64], number: usize, indice: usize) -> u64 {
    if counter_programme[indice] == MANGE {
        TENUE
    } else if counter_programme[(indice + number - 1) % number] == MANGE {
        PRECEDENTE
    } else {
        0
    }
}

/// Enumerates the extended states of R_n: circular words without adjacent
/// `eat` neighbors, derived forks, interleaved `pc_0 left_0 pc_1 …`. Same
/// algorithm as the Python orchestrator (`scripts/confrontation.py`): the
/// generation ↔ pins consistency is checked by a dedicated test.
fn etats_etendus(number: usize) -> Vec<Vec<u64>> {
    let mut mots = Vec::new();
    let mut mot = vec![PENSE; number];
    loop {
        let circulaire_ok = (0..number)
            .all(|indice| !(mot[indice] == MANGE && mot[(indice + 1) % number] == MANGE));
        if circulaire_ok {
            mots.push(mot.clone());
        }
        let mut indice = number;
        loop {
            if indice == 0 {
                return mots
                    .into_iter()
                    .map(|mot| {
                        (0..number)
                            .flat_map(|i| [mot[i], fourchette(&mot, number, i)])
                            .collect()
                    })
                    .collect();
            }
            indice -= 1;
            if mot[indice] < MANGE {
                mot[indice] += 1;
                break;
            }
            mot[indice] = PENSE;
        }
    }
}

/// Temporary-file counter: a unique name per call (under
/// `cargo test`, the tests of one binary run as threads of a
/// single process — the pid alone is not enough).
static COUNTER_FICHIERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Writes the shared extended-states file (same format as the
/// Python orchestrator: first line `n <n>`, then one state per line) and
/// returns its path.
fn write_fichier_etats(number: u64) -> PathBuf {
    let numero = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "confrontation-test-philosophers-{number}-{}-{numero}.txt",
        std::process::id()
    ));
    let etats = etats_etendus(number as usize);
    let mut content = format!("n {number}\n");
    for etat in etats {
        let ligne: Vec<String> = etat.iter().map(|value| value.to_string()).collect();
        content.push_str(&ligne.join(" "));
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
    for (n, satisfait) in [(4, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let measure = executer(&Workload::AleaLcg, &[n.to_string()], None).expect("workload run");
        assert_eq!(TOOL, "lisage");
        assert_eq!(measure.variant, Variant::Bdd);
        assert_eq!(measure.workload, Workload::AleaLcg);
        assert_eq!(measure.verifie, satisfait, "n={n}: pinned satisfactions");
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
            executer(&Workload::AleaLcg, &[n.to_string()], None).expect_err("bounds rejected");
        assert!(matches!(
            error,
            ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "alea-lcg"
        ));
    }
}

/// `philosophers` workload: |R_n| pinned over n = 3..7, nodes of R_3 pinned
/// (33), cardinality via pli == number of states in the file.
#[test]
fn test_philosophers_fil_rouge_epingle() {
    for &(number, expected) in R_EXPECTEDS {
        let path = write_fichier_etats(number);
        let parametres = vec![number.to_string(), path.display().to_string()];
        let measure = executer(&Workload::Philosophers, &parametres, None).expect("workload run");
        assert_eq!(measure.verifie, expected, "n={number}: pinned |R_n|");
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
fn test_philosophers_bound_refusee() {
    let path = write_fichier_etats(3);
    let parametres = vec!["33".to_string(), path.display().to_string()];
    let error = executer(&Workload::Philosophers, &parametres, None).expect_err("bound rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "philosophers"
    ));
}

/// `philosophers` workload: invalid file (empty, non-conforming first line,
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
    for (numero, content) in cas.iter().enumerate() {
        let uniq = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-invalide-{numero}-{}-{uniq}.txt",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("invalid file written");
        let parametres = vec!["3".to_string(), path.display().to_string()];
        let error = executer(&Workload::Philosophers, &parametres, None);
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
fn test_window_encodages_epingles() {
    let absolu = executer(
        &Workload::Window,
        &["4".to_string(), "10".to_string()],
        None,
    )
    .expect("absolute");
    let relatif = executer(
        &Workload::Window,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        None,
    )
    .expect("relative");
    assert_eq!(absolu.verifie, PATHS_WINDOW);
    assert_eq!(relatif.verifie, PATHS_WINDOW);
    assert_eq!(absolu.nodes, NODES_WINDOW_ABSOLUE);
    assert_eq!(relatif.nodes, NODES_WINDOW_RELATIVE);
    assert_eq!(absolu.encodage, Some(Encodage::Absolu));
    assert_eq!(relatif.encodage, Some(Encodage::Relatif));
}

/// Unknown workload (binary edge: parsed into an enum, `None`) and
/// missing parameters: typed errors.
#[test]
fn test_workloads_inconnues_refusees() {
    assert_eq!(Workload::depuis_name("inexistante"), None);
    assert_eq!(Workload::depuis_name("alea-lcg"), Some(Workload::AleaLcg));
    let error = executer(&Workload::AleaLcg, &[], None).expect_err("missing parameters");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "alea-lcg"
    ));
}

/// JSON serialization: one line, fixed key order, encoding present
/// only for the window. Pinned character by character.
#[test]
fn test_serialisation_ligne_epinglee() {
    let measure = MeasureConfrontation {
        variant: Variant::Mdd,
        workload: Workload::Window,
        parametres: vec![
            ("taille".to_string(), "4".to_string()),
            ("nb_bases".to_string(), "10".to_string()),
        ],
        encodage: Some(Encodage::Relatif),
        temps_ms: 1.25,
        rss_max_ko: 4096,
        nodes: 3,
        verifie: 40,
    };
    let expected = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"window\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\"encodage\":\"relatif\",\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialiser_ligne(&measure), expected);
    let without_encodage = MeasureConfrontation {
        encodage: None,
        ..measure
    };
    let expected_without = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"window\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialiser_ligne(&without_encodage), expected_without);
}

/// The harness's extended-state generation (same algorithm as
/// the Python orchestrator) covers the golden-thread pins: |R_n| for
/// n = 3..7, each state with 2n values in {0,1,2}.
#[test]
fn test_etats_etendus_coherents_with_fil_rouge() {
    for &(number, expected) in R_EXPECTEDS {
        let etats = etats_etendus(number as usize);
        assert_eq!(etats.len() as u64, expected, "n={number}: pinned |R_n|");
        assert!(etats.iter().all(|etat| {
            etat.len() == 2 * number as usize && etat.iter().all(|value| *value <= MANGE)
        }));
    }
}

/// Narration (ADR-0100 decision 6): each workload emits an
/// `Attr`/`Synthesized` event carrying the summary — pinned by a
/// collecting observer.
#[test]
fn test_narration_workload_epinglee() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{ChannelAttr, Observer, TraceEvent};

    #[derive(Clone)]
    struct Collector {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl Observer for Collector {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let collector = Collector {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let measure =
        executer(&Workload::AleaLcg, &["4".to_string()], Some(&collector)).expect("workload run");
    let expected = format!(
        "workload=alea-lcg nodes={} check={}",
        measure.nodes, SATISFACTIONS_LCG_N4
    );
    // Every workload emits its summary: philosophers, window, cnf and
    // pnml are pinned here too (a regression that would lose one of
    // them would fail this test).
    let path = write_fichier_etats(3);
    let measure_philo = executer(
        &Workload::Philosophers,
        &["3".to_string(), path.display().to_string()],
        Some(&collector),
    )
    .expect("philosophers workload run");
    let measure_window = executer(
        &Workload::Window,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        Some(&collector),
    )
    .expect("window workload run");
    let measure_cnf = executer(
        &Workload::Cnf,
        &[chemin_modele("uf20-01.cnf")],
        Some(&collector),
    )
    .expect("cnf workload run");
    let measure_pnml = executer(
        &Workload::Pnml,
        &[chemin_modele("SwimmingPool-1.pnml")],
        Some(&collector),
    )
    .expect("pnml workload run");
    let expected_list = [
        expected,
        format!(
            "workload=philosophers nodes={} check={}",
            measure_philo.nodes, measure_philo.verifie
        ),
        format!(
            "workload=window encoding=relatif nodes={} check={}",
            measure_window.nodes, measure_window.verifie
        ),
        format!(
            "workload=cnf nodes={} check={}",
            measure_cnf.nodes, measure_cnf.verifie
        ),
        format!(
            "workload=pnml nodes={} check={}",
            measure_pnml.nodes, measure_pnml.verifie
        ),
    ];
    let events = collector.evenements.lock().expect("mutex").clone();
    for expected in &expected_list {
        assert!(
            events.iter().any(|e| matches!(
                e,
                TraceEvent::Attr {
                    channel: ChannelAttr::Synthesized,
                    value,
                    ..
                } if value.rendre() == *expected
            )),
            "the narration event carries the summary: {expected}"
        );
    }
}

/// Path of a versioned model of `crates/lisage-bench/models/` (cargo runs
/// the tests with the crate root as working directory).
fn chemin_modele(nom: &str) -> String {
    format!("models/{nom}")
}

/// Pinned cross-check values of the real charges (M7-4, issue #114): BDD
/// node count (`nodes`) and satisfying assignments (`verifie`) on the
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

/// `cnf` workload on the real SATLIB charge `uf20-01.cnf`: pinned node
/// count and pinned satisfactions (cross-tool comparable), `bdd` variant.
#[test]
fn test_cnf_uf20_epingle() {
    let path = chemin_modele("uf20-01.cnf");
    let measure = executer(&Workload::Cnf, &[path], None).expect("cnf workload run");
    assert_eq!(measure.variant, Variant::Bdd);
    assert_eq!(measure.workload, Workload::Cnf);
    assert_eq!(measure.nodes, CNF_UF20_01_NODES, "uf20-01: pinned nodes");
    assert_eq!(
        measure.verifie, CNF_UF20_01_SATISFAITS,
        "uf20-01: pinned satisfactions"
    );
    assert!(measure.rss_max_ko > 0);
}

/// `pnml` workload on the real MCC charge `SwimmingPool-1.pnml`: pinned
/// node count of the transition relation (18 variables) and pinned number
/// of firing pairs.
#[test]
fn test_pnml_swimmingpool_epingle() {
    let path = chemin_modele("SwimmingPool-1.pnml");
    let measure = executer(&Workload::Pnml, &[path], None).expect("pnml workload run");
    assert_eq!(measure.variant, Variant::Bdd);
    assert_eq!(measure.workload, Workload::Pnml);
    assert_eq!(
        measure.nodes, PNML_SWIMMINGPOOL_NODES,
        "SwimmingPool-1: pinned nodes"
    );
    assert_eq!(
        measure.verifie, PNML_SWIMMINGPOOL_SATISFAITS,
        "SwimmingPool-1: pinned firing pairs"
    );
}

/// `pnml` workload on the heavy MCC charge `erk-000001.pnml` (22
/// variables, 2^22-entry truth table): pinned values, `#[ignore]`d —
/// several seconds even in release, run explicitly with
/// `cargo test -p lisage-bench -- --ignored`.
#[test]
#[ignore]
fn test_pnml_erk_epingle() {
    let path = chemin_modele("erk-000001.pnml");
    let measure = executer(&Workload::Pnml, &[path], None).expect("pnml workload run");
    assert_eq!(measure.nodes, PNML_ERK_NODES, "ERK-000001: pinned nodes");
    assert_eq!(
        measure.verifie, PNML_ERK_SATISFAITS,
        "ERK-000001: pinned firing pairs"
    );
}

/// The real charges serialize in the standard stdin protocol of the
/// external witnesses (`cudd_count`, `oxidd_count`): first line `n`, then
/// the `2^n` values `0`/`1`, 64 per line — pinned on a small table.
#[test]
fn test_serialiser_charge_stdin_epingle() {
    use lisage_bench::confrontation::serialiser_charge_stdin;
    let table = [false, true, true, false, false, false, true, true];
    let sortie = serialiser_charge_stdin(3, &table);
    assert_eq!(sortie, "3\n0 1 1 0 0 0 1 1\n");
    // 64 values per line: n = 7 wraps after the 64th value.
    let grande = vec![true; 128];
    let sortie = serialiser_charge_stdin(7, &grande);
    let lignes: Vec<&str> = sortie.lines().collect();
    assert_eq!(lignes[0], "7");
    assert_eq!(lignes[1].split(' ').count(), 64);
    assert_eq!(lignes[2].split(' ').count(), 64);
}

/// `cnf` workload: malformed DIMACS inputs (missing problem line,
/// unreadable literal, unterminated clause, literal beyond `n`) and
/// unreadable file → typed errors, never swallowed.
#[test]
fn test_cnf_inputs_invalides() {
    let cas = [
        "1 2 0\n",
        "p cnf 2 1\nx 0\n",
        "p cnf 2 1\n1 2\n",
        "p cnf 2 1\n3 0\n",
        "p sat 2 1\n1 0\n",
    ];
    for (numero, content) in cas.iter().enumerate() {
        let uniq = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-cnf-invalide-{numero}-{}-{uniq}.cnf",
            std::process::id()
        ));
        std::fs::write(&path, content).expect("invalid CNF written");
        let error = executer(&Workload::Cnf, &[path.display().to_string()], None);
        assert!(
            matches!(error, Err(ErrorConfrontation::InvalidInput { .. })),
            "typed error InvalidInput: {error:?}"
        );
    }
    let error = executer(&Workload::Cnf, &["models/inexistant.cnf".to_string()], None)
        .expect_err("missing file rejected");
    assert!(matches!(error, ErrorConfrontation::InvalidInput { .. }));
    let error = executer(&Workload::Cnf, &[], None).expect_err("missing parameter rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "cnf"
    ));
}

/// `pnml` workload: a synthetic 13-place net exceeds the 2p ≤ 24 guard →
/// typed error, no panic; unknown names and the new names at the binary
/// edge.
#[test]
fn test_pnml_bound_refusee_et_noms() {
    assert_eq!(Workload::depuis_name("cnf"), Some(Workload::Cnf));
    assert_eq!(Workload::depuis_name("pnml"), Some(Workload::Pnml));
    let mut contenu = String::from(
        "<?xml version=\"1.0\"?><pnml><net id=\"t\" \
         type=\"http://www.pnml.org/version-2009/grammar/ptnet\"><page id=\"p0\">",
    );
    for i in 0..13 {
        contenu.push_str(&format!("<place id=\"p{i}\"/>"));
    }
    contenu.push_str(
        "<transition id=\"t0\"/><arc id=\"a0\" source=\"p0\" target=\"t0\"/>\
                      <arc id=\"a1\" source=\"t0\" target=\"p1\"/></page></net></pnml>",
    );
    let uniq = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "confrontation-test-pnml-13places-{}-{uniq}.pnml",
        std::process::id()
    ));
    std::fs::write(&path, contenu).expect("13-place PNML written");
    let error =
        executer(&Workload::Pnml, &[path.display().to_string()], None).expect_err("bound rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "pnml"
    ));
    let error = executer(&Workload::Pnml, &[], None).expect_err("missing parameter rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "pnml"
    ));
}

/// Property (ADR-0012 property tier): on deterministic pseudo-random CNF
/// charges, the workload's cross-check value equals an independent
/// brute-force satisfaction count computed in the test (the LCG of the
/// `alea-lcg` workload drives the generation).
#[test]
fn test_cnf_propriete_verifie_contre_force_brute() {
    use lisage_bench::confrontation::{INCREMENT_LCG, MULTIPLICATEUR_LCG, SEED_LCG};
    let mut etat = SEED_LCG;
    let mut next_bit = move || {
        etat = etat
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        (etat >> 63) == 1
    };
    for essai in 0..8 {
        let n = 3 + (essai % 4) as u32; // 3..6 variables
        let m = 2 + essai as usize * 3; // 2..23 clauses
        let mut clauses: Vec<Vec<i64>> = Vec::new();
        let mut contenu = format!("p cnf {n} {m}\n");
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
            contenu.push_str(
                &clause
                    .iter()
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            contenu.push_str(" 0\n");
            clauses.push(clause);
        }
        let uniq = COUNTER_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "confrontation-test-cnf-propriete-{essai}-{}-{uniq}.cnf",
            std::process::id()
        ));
        std::fs::write(&path, &contenu).expect("random CNF written");
        let measure = executer(&Workload::Cnf, &[path.display().to_string()], None)
            .expect("cnf workload run");
        let attendu = (0..(1u64 << n))
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
            measure.verifie, attendu,
            "essai {essai}: verifie == brute-force count"
        );
        assert!(measure.nodes > 0);
    }
}

/// `serialiser_charge_depuis_fichier` (the `dump-charge` feed of the
/// witnesses): the serialized charge of a real input starts with `n` and
/// carries exactly `2^n` values; non-file workloads are rejected.
#[test]
fn test_serialiser_charge_depuis_fichier_epingle() {
    use lisage_bench::confrontation::serialiser_charge_depuis_fichier;
    for (workload, nom, n) in [
        (Workload::Cnf, "uf20-01.cnf", 20u32),
        (Workload::Pnml, "SwimmingPool-1.pnml", 18),
    ] {
        let sortie = serialiser_charge_depuis_fichier(&workload, &chemin_modele(nom))
            .expect("charge serialized");
        let mut lignes = sortie.lines();
        assert_eq!(lignes.next(), Some(n.to_string().as_str()));
        let valeurs: usize = lignes
            .flat_map(|l| l.split(' '))
            .filter(|t| !t.is_empty())
            .count();
        assert_eq!(valeurs, 1usize << n, "{nom}: 2^n values");
        assert!(
            sortie
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b' ' || b == b'\n')
        );
    }
    let error = serialiser_charge_depuis_fichier(&Workload::AleaLcg, "models/uf20-01.cnf")
        .expect_err("non-numeric alea-lcg argument rejected");
    assert!(matches!(
        error,
        ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "alea-lcg"
    ));
}

/// `dump-charge alea-lcg <n>` (MEM-7 pilot feed of the witnesses): the
/// serialized charge is the deterministic LCG table — first line `n`,
/// then exactly `2^n` values `0`/`1`, with the pinned number of
/// satisfactions; out-of-range `n` is a typed error.
#[test]
fn test_serialiser_charge_alea_lcg_pinned() {
    use lisage_bench::confrontation::serialiser_charge_depuis_fichier;
    for (n, satisfied) in [(4u32, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let output = serialiser_charge_depuis_fichier(&Workload::AleaLcg, &n.to_string())
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
        let error = serialiser_charge_depuis_fichier(&Workload::AleaLcg, n)
            .expect_err("invalid n rejected");
        assert!(matches!(
            error,
            ErrorConfrontation::ParametresInvalides { workload, .. } if workload == "alea-lcg"
        ));
    }
}
