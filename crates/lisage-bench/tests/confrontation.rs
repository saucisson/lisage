//! Tests of the confrontation harness (ADR-0102): pinned cross-check
//! values (LCG, golden thread R_n, sliding window), typed errors,
//! deterministic JSON serialization. Time/memory measurements are not
//! pinned (they depend on the machine): only the *semantic* values
//! are — that is the explicit falsification of the harness.

use std::path::PathBuf;

use lisage_bench::confrontation::{
    Charge, ErreurConfrontation, MesureConfrontation, OUTIL, Variante, executer, serialiser_ligne,
};
use lisage_core::atelier::Encodage;

/// `think`.
const PENSE: u64 = 0;
/// `eat`.
const MANGE: u64 = 2;
/// Fork held by the current philosopher (he eats).
const TENUE: u64 = 1;
/// Fork held by the previous philosopher (he eats).
const PRECEDENTE: u64 = 2;
/// |R_n| of the golden thread for n = 3..7 (verified by simulation, M4-4).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Shared nodes of R_3 (didactic workshop, fragment 1: 33 nodes).
const NOEUDS_R3: u64 = 33;
/// Paths of the test window: 10 bases × 4 positions.
const CHEMINS_FENETRE: u64 = 40;
/// Nodes of the absolute window (didactic workshop, fragment 2).
const NOEUDS_FENETRE_ABSOLUE: u64 = 12;
/// Nodes of the relative window: a single shared form, O(1).
const NOEUDS_FENETRE_RELATIVE: u64 = 3;
/// Satisfactions of the LCG table for n = 4 (pinned, deterministic).
const SATISFACTIONS_LCG_N4: u64 = 7;
/// Satisfactions of the LCG table for n = 12 (pinned, deterministic —
/// same LCG as the M1-8 baseline).
const SATISFACTIONS_LCG_N12: u64 = 2074;

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn fourchette(compteur_programme: &[u64], nombre: usize, indice: usize) -> u64 {
    if compteur_programme[indice] == MANGE {
        TENUE
    } else if compteur_programme[(indice + nombre - 1) % nombre] == MANGE {
        PRECEDENTE
    } else {
        0
    }
}

/// Enumerates the extended states of R_n: circular words without adjacent
/// `eat` neighbors, derived forks, interleaved `pc_0 left_0 pc_1 …`. Same
/// algorithm as the Python orchestrator (`scripts/confrontation.py`): the
/// generation ↔ pins consistency is checked by a dedicated test.
fn etats_etendus(nombre: usize) -> Vec<Vec<u64>> {
    let mut mots = Vec::new();
    let mut mot = vec![PENSE; nombre];
    loop {
        let circulaire_ok = (0..nombre)
            .all(|indice| !(mot[indice] == MANGE && mot[(indice + 1) % nombre] == MANGE));
        if circulaire_ok {
            mots.push(mot.clone());
        }
        let mut indice = nombre;
        loop {
            if indice == 0 {
                return mots
                    .into_iter()
                    .map(|mot| {
                        (0..nombre)
                            .flat_map(|i| [mot[i], fourchette(&mot, nombre, i)])
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
static COMPTEUR_FICHIERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Writes the shared extended-states file (same format as the
/// Python orchestrator: first line `n <n>`, then one state per line) and
/// returns its path.
fn ecrire_fichier_etats(nombre: u64) -> PathBuf {
    let numero = COMPTEUR_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let chemin = std::env::temp_dir().join(format!(
        "confrontation-test-philosophes-{nombre}-{}-{numero}.txt",
        std::process::id()
    ));
    let etats = etats_etendus(nombre as usize);
    let mut contenu = format!("n {nombre}\n");
    for etat in etats {
        let ligne: Vec<String> = etat.iter().map(|valeur| valeur.to_string()).collect();
        contenu.push_str(&ligne.join(" "));
        contenu.push('\n');
    }
    std::fs::write(&chemin, contenu).expect("temporary states file written");
    chemin
}

/// `alea-lcg` workload: pinned satisfactions (deterministic LCG), C6
/// checked internally by the runner (independent mini-BDD), measurement
/// returned with the `bdd` variant.
#[test]
fn test_alea_lcg_satisfactions_epinglees() {
    for (n, satisfait) in [(4, SATISFACTIONS_LCG_N4), (12, SATISFACTIONS_LCG_N12)] {
        let mesure = executer(&Charge::AleaLcg, &[n.to_string()], None).expect("workload run");
        assert_eq!(OUTIL, "lisage");
        assert_eq!(mesure.variante, Variante::Bdd);
        assert_eq!(mesure.charge, Charge::AleaLcg);
        assert_eq!(mesure.verifie, satisfait, "n={n}: pinned satisfactions");
        assert!(mesure.noeuds > 0, "n={n}: non-empty diagram");
        assert!(mesure.temps_ms.is_finite(), "n={n}: measured time");
        assert!(mesure.rss_max_ko > 0, "n={n}: VmHWM read");
    }
}

/// `alea-lcg` workload: rejected bounds (zero and too large), typed error.
#[test]
fn test_alea_lcg_bornes_refusees() {
    for n in [0, 25] {
        let erreur =
            executer(&Charge::AleaLcg, &[n.to_string()], None).expect_err("bounds rejected");
        assert!(matches!(
            erreur,
            ErreurConfrontation::ParametresInvalides { charge, .. } if charge == "alea-lcg"
        ));
    }
}

/// `philosophes` workload: |R_n| pinned over n = 3..7, nodes of R_3 pinned
/// (33), cardinal via pli == number of states in the file.
#[test]
fn test_philosophes_fil_rouge_epingle() {
    for &(nombre, attendu) in R_ATTENDUS {
        let chemin = ecrire_fichier_etats(nombre);
        let parametres = vec![nombre.to_string(), chemin.display().to_string()];
        let mesure = executer(&Charge::Philosophes, &parametres, None).expect("workload run");
        assert_eq!(mesure.verifie, attendu, "n={nombre}: pinned |R_n|");
        assert_eq!(mesure.variante, Variante::Mdd);
        if nombre == 3 {
            assert_eq!(mesure.noeuds, NOEUDS_R3, "R_3: 33 shared nodes");
        }
        assert!(mesure.rss_max_ko > 0);
    }
}

/// `philosophes` workload: upper bound on n (anti-overflow guard
/// of the interleaved schema) — typed error, no panic.
#[test]
fn test_philosophes_borne_refusee() {
    let chemin = ecrire_fichier_etats(3);
    let parametres = vec!["33".to_string(), chemin.display().to_string()];
    let erreur = executer(&Charge::Philosophes, &parametres, None).expect_err("bound rejected");
    assert!(matches!(
        erreur,
        ErreurConfrontation::ParametresInvalides { charge, .. } if charge == "philosophes"
    ));
}

/// `philosophes` workload: invalid file (empty, non-conforming first line,
/// inconsistent state length, out-of-domain value) → typed error,
/// never swallowed.
#[test]
fn test_philosophes_entrees_invalides() {
    let cas = [
        "",
        "x 3\n",
        "n 3\n0 0\n",
        "n 2\n0 0 0 0 0 0\n",
        "n 3\n0 0 0 0 0 7\n",
    ];
    for (numero, contenu) in cas.iter().enumerate() {
        let uniq = COMPTEUR_FICHIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let chemin = std::env::temp_dir().join(format!(
            "confrontation-test-invalide-{numero}-{}-{uniq}.txt",
            std::process::id()
        ));
        std::fs::write(&chemin, contenu).expect("invalid file written");
        let parametres = vec!["3".to_string(), chemin.display().to_string()];
        let erreur = executer(&Charge::Philosophes, &parametres, None);
        assert!(erreur.is_err(), "invalid input rejected: {contenu:?}");
        assert!(
            matches!(erreur, Err(ErreurConfrontation::EntreeInvalide { .. })),
            "typed error EntreeInvalide, not swallowed: {erreur:?}"
        );
    }
}

/// `fenetre` workload: same paths in both encodings (40 = 10×4),
/// nodes 12 (absolute) vs 3 (relative) — the relative sharing is
/// visible to the naked eye (didactic workshop, fragment 2).
#[test]
fn test_fenetre_encodages_epingles() {
    let absolu =
        executer(&Charge::Fenetre, &["4".to_string(), "10".to_string()], None).expect("absolute");
    let relatif = executer(
        &Charge::Fenetre,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        None,
    )
    .expect("relative");
    assert_eq!(absolu.verifie, CHEMINS_FENETRE);
    assert_eq!(relatif.verifie, CHEMINS_FENETRE);
    assert_eq!(absolu.noeuds, NOEUDS_FENETRE_ABSOLUE);
    assert_eq!(relatif.noeuds, NOEUDS_FENETRE_RELATIVE);
    assert_eq!(absolu.encodage, Some(Encodage::Absolu));
    assert_eq!(relatif.encodage, Some(Encodage::Relatif));
}

/// Unknown workload (binary edge: parsed into an enum, `None`) and
/// missing parameters: typed errors.
#[test]
fn test_charges_inconnues_refusees() {
    assert_eq!(Charge::depuis_nom("inexistante"), None);
    assert_eq!(Charge::depuis_nom("alea-lcg"), Some(Charge::AleaLcg));
    let erreur = executer(&Charge::AleaLcg, &[], None).expect_err("missing parameters");
    assert!(matches!(
        erreur,
        ErreurConfrontation::ParametresInvalides { charge, .. } if charge == "alea-lcg"
    ));
}

/// JSON serialization: one line, fixed key order, encoding present
/// only for the window. Pinned character by character.
#[test]
fn test_serialisation_ligne_epinglee() {
    let mesure = MesureConfrontation {
        variante: Variante::Mdd,
        charge: Charge::Fenetre,
        parametres: vec![
            ("taille".to_string(), "4".to_string()),
            ("nb_bases".to_string(), "10".to_string()),
        ],
        encodage: Some(Encodage::Relatif),
        temps_ms: 1.25,
        rss_max_ko: 4096,
        noeuds: 3,
        verifie: 40,
    };
    let attendu = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"fenetre\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\"encodage\":\"relatif\",\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialiser_ligne(&mesure), attendu);
    let sans_encodage = MesureConfrontation {
        encodage: None,
        ..mesure
    };
    let attendu_sans = "{\"outil\":\"lisage\",\"variante\":\"mdd\",\"charge\":\"fenetre\",\
\"parametres\":{\"taille\":\"4\",\"nb_bases\":\"10\"},\
\"temps_ms\":1.250,\"rss_max_ko\":4096,\"noeuds\":3,\"verifie\":40}";
    assert_eq!(serialiser_ligne(&sans_encodage), attendu_sans);
}

/// The harness's extended-state generation (same algorithm as
/// the Python orchestrator) covers the golden-thread pins: |R_n| for
/// n = 3..7, each state with 2n values in {0,1,2}.
#[test]
fn test_etats_etendus_coherents_avec_fil_rouge() {
    for &(nombre, attendu) in R_ATTENDUS {
        let etats = etats_etendus(nombre as usize);
        assert_eq!(etats.len() as u64, attendu, "n={nombre}: pinned |R_n|");
        assert!(etats.iter().all(|etat| {
            etat.len() == 2 * nombre as usize && etat.iter().all(|valeur| *valeur <= MANGE)
        }));
    }
}

/// Narration (ADR-0100 decision 6): each workload emits an
/// `Attr`/`Synthétise` event carrying the summary — pinned by a
/// collecting observer.
#[test]
fn test_narration_charge_epinglee() {
    use std::sync::{Arc, Mutex};

    use lisage_core::observer::{CanalAttr, Observer, TraceEvent};

    #[derive(Clone)]
    struct Collecteur {
        evenements: Arc<Mutex<Vec<TraceEvent>>>,
    }
    impl Observer for Collecteur {
        fn on_event(&self, event: &TraceEvent) {
            self.evenements.lock().expect("mutex").push(event.clone());
        }
    }

    let collecteur = Collecteur {
        evenements: Arc::new(Mutex::new(Vec::new())),
    };
    let mesure =
        executer(&Charge::AleaLcg, &["4".to_string()], Some(&collecteur)).expect("workload run");
    let attendu = format!(
        "workload=alea-lcg nodes={} check={}",
        mesure.noeuds, SATISFACTIONS_LCG_N4
    );
    // The three workloads emit their summary: philosophes and
    // fenetre are pinned here too (a regression that would lose them
    // would fail this test).
    let chemin = ecrire_fichier_etats(3);
    let mesure_philo = executer(
        &Charge::Philosophes,
        &["3".to_string(), chemin.display().to_string()],
        Some(&collecteur),
    )
    .expect("philosophes workload run");
    let mesure_fenetre = executer(
        &Charge::Fenetre,
        &["4".to_string(), "10".to_string(), "relatif".to_string()],
        Some(&collecteur),
    )
    .expect("fenetre workload run");
    let attendus = [
        attendu,
        format!(
            "workload=philosophes nodes={} check={}",
            mesure_philo.noeuds, mesure_philo.verifie
        ),
        format!(
            "workload=fenetre encoding=relatif nodes={} check={}",
            mesure_fenetre.noeuds, mesure_fenetre.verifie
        ),
    ];
    let events = collecteur.evenements.lock().expect("mutex").clone();
    for attendu in &attendus {
        assert!(
            events.iter().any(|e| matches!(
                e,
                TraceEvent::Attr {
                    canal: CanalAttr::Synthétise,
                    valeur,
                    ..
                } if valeur == attendu
            )),
            "the narration event carries the summary: {attendu}"
        );
    }
}
