//! Philosophers benchmark (M4-4, statement C8): reference measurements of the
//! golden thread (docs/golden-thread-example-n-philosophers-end-to-end § 6-9; the measurement against external SDD/HSDD
//! is outside the CI perimeter). Falsifiable predictions checked here:
//! |R_n| = 20/56/152/416/1136 (n = 3..7), left-handed variant |R'_n| =
//! 72/196/536/1464 (n = 4..7) with ratio |R'_n|/|R_n| = 1,2887 **constant**,
//! 7 forms (5 legal cells + 2 terminals) independent of n, and the
//! flat prototype edge (union of maps) pinned exactly, growing with
//! n. Live peak, firings and rounds read via the observer. Any
//! contrary measurement fails the test: that is the explicit
//! falsification.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Test anchor.
const ANCRE_TEST: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `hungry`.
const AFFAME: u64 = 1;
/// `eat`.
const MANGE: u64 = 2;
/// `g`: intermediate state of the left-handed philosopher (position 0 only).
const GAUCHER: u64 = 3;
/// Expected |R_n| (golden thread § 0, verified by simulation up to n = 20).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// Expected |R'_n| for the left-handed variant (golden thread § 9.2, n = 4..7).
const R_GAUCHER_ATTENDUS: &[(u64, u64)] = &[(4, 72), (5, 196), (6, 536), (7, 1464)];
/// Predicted ratio |R'_n| / |R_n|, constant (golden thread § 9.4).
const RATIO_GAUCHER: f64 = 1.2887;
/// Tolerance on the ratio (constancy and value) — explicit falsification.
const TOLERANCE_RATIO: f64 = 0.01;
/// Number of legal cells of the minimal automaton (5 + 2 terminals = 7 forms).
const NB_CELLULES_LEGALES: usize = 5;
/// Step of the interleaving keys: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Offset of the `left_i` key after the `pc_i` key in the interleaving.
const DECALAGE_CLE_GAUCHE: u64 = 1;
/// Live nodes of the flat prototype (union of maps) per n — measured edge,
/// growing with n. The 4n − 4 of the golden thread concerns the flat prototype of
/// the encoded automaton (outside this bench's perimeter).
const BORD_PLAT: &[(u64, usize)] = &[(3, 206), (4, 646), (5, 1850), (6, 5198), (7, 14362)];

type PoolRef = Arena<ReferenceController>;

/// Recording observer: firings (NodeCreated) and rounds (Round).
#[derive(Default)]
struct RecordingObserver {
    firings: Mutex<u64>,
    passes: Mutex<u64>,
}

impl Observer for RecordingObserver {
    fn on_event(&self, event: &TraceEvent) {
        match event {
            TraceEvent::NodeCreated { .. } => *self.firings.lock().unwrap() += 1,
            TraceEvent::Round { .. } => *self.passes.lock().unwrap() += 1,
            _ => {}
        }
    }
}

/// Schema of the ring unrolled in interleaving (golden thread § 2): `2n` keys,
/// key `2i` = `pc_i`, key `2i+1` = `left_i`; domain {0,1,2} (+ g for the
/// left-handed).
fn schema_anneau(n: u64, gaucher: bool) -> Schema {
    let domaine = if gaucher {
        Domaine::nouveau(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE), Atom(GAUCHER)])
    } else {
        Domaine::nouveau(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE)])
    };
    Schema::Map((0..PAS_CLE_INTERCALAGE * n).map(Atom).collect(), domaine)
}

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn fourchette(compteur_programme: &[u64], n: usize, i: usize) -> u64 {
    const LIBRE: u64 = 0;
    const TENUE: u64 = 1;
    const PRECEDENTE: u64 = 2;
    if compteur_programme[i] == MANGE {
        TENUE
    } else if compteur_programme[(i + n - 1) % n] == MANGE {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Enumerates the reachable circular words (regular or left-handed).
fn mots_atteignables(n: usize, gaucher: bool) -> Vec<Vec<u64>> {
    let mut resultat = Vec::new();
    let mut mot = vec![PENSE; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(mot[i] == MANGE && mot[(i + 1) % n] == MANGE));
        let gaucher_ok = !gaucher || !(mot[0] == GAUCHER && mot[1] == MANGE);
        if circulaire_ok && gaucher_ok {
            resultat.push(mot.clone());
        }
        // Base-3 increment; the left-handed has a 4th value `g` at
        // position 0 only.
        let mut i = n;
        loop {
            if i == 0 {
                return resultat;
            }
            i -= 1;
            let max = if gaucher && i == 0 { GAUCHER } else { MANGE };
            if mot[i] < max {
                mot[i] += 1;
                break;
            }
            mot[i] = PENSE;
        }
    }
}

/// Builds R_n (or R'_n) in the pool: union of the compiled maps.
fn construire_r_n(pool: &mut PoolRef, n: u64, gaucher: bool) -> NodeId {
    let schema = schema_anneau(n, gaucher);
    let keys = SchemaKeys::new(schema.clone());
    let mut racine = pool.intern(ANCRE_TEST, Form::Terminal(false));
    for mot in mots_atteignables(n as usize, gaucher) {
        let map: BTreeMap<Value, Value> = (0..n)
            .flat_map(|i| {
                let i = i as usize;
                [
                    (PAS_CLE_INTERCALAGE * i as u64, mot[i]),
                    (
                        PAS_CLE_INTERCALAGE * i as u64 + DECALAGE_CLE_GAUCHE,
                        fourchette(&mot, n as usize, i),
                    ),
                ]
            })
            .map(|(cle, valeur)| (Value::Atom(Atom(cle)), Value::Atom(Atom(valeur))))
            .collect();
        let membre = compiler(pool, ANCRE_TEST, &schema, &Value::Map(map)).unwrap();
        racine = union(pool, &keys, ANCRE_TEST, racine, membre).unwrap();
    }
    racine
}

/// Counts |R_n| via the `(ℕ,+,×)` pli on the given schema.
fn compter_r_n(pool_entree: &PoolRef, schema: &Schema, racine: NodeId) -> u64 {
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    compter(pool_entree, &mut sortie, ANCRE_TEST, schema, racine).unwrap()
}

/// M4-4: |R_n| via DD pli == enumeration == golden-thread values
/// (20/56/152/416/1136). Explicit falsification if the measurement says
/// otherwise.
#[test]
fn test_bench_cardinaux_r_n() {
    for &(n, attendu) in R_ATTENDUS {
        let schema = schema_anneau(n, false);
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let racine = construire_r_n(&mut pool, n, false);
        let compte = compter_r_n(&pool, &schema, racine);
        let reference = mots_atteignables(n as usize, false).len() as u64;
        assert_eq!(compte, reference, "n={n}: DD pli == enumeration");
        assert_eq!(
            compte, attendu,
            "n={n}: |R_{n}| = {attendu} (golden thread)"
        );
    }
}

/// M4-4: left-handed variant — |R'_n| = 72/196/536/1464 (n = 4..7) and ratio
/// |R'_n| / |R_n| = 1,2887 **constant** (the irregular class shifts the
/// constant, not the exponent). Explicit falsification outside tolerance.
#[test]
fn test_bench_gaucher_ratio_constant() {
    let mut ratios = Vec::new();
    for &(n, attendu) in R_GAUCHER_ATTENDUS {
        let schema = schema_anneau(n, true);
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(NoOpObserver),
        );
        let racine = construire_r_n(&mut pool, n, true);
        // |R'_n| via DD pli == reference enumeration == golden thread § 9.2.
        let compte = compter_r_n(&pool, &schema, racine);
        let reference = mots_atteignables(n as usize, true).len() as u64;
        assert_eq!(
            compte, reference,
            "n={n}: DD pli == enumeration (left-handed)"
        );
        assert_eq!(
            reference, attendu,
            "n={n}: |R'_{n}| = {attendu} (golden thread § 9.2)"
        );
        // Regular |R_n| for the ratio.
        let schema_reg = schema_anneau(n, false);
        let mut pool_reg = Arena::new(
            ReferenceController::new(schema_reg.clone()),
            Box::new(NoOpObserver),
        );
        let racine_reg = construire_r_n(&mut pool_reg, n, false);
        let compte_reg = compter_r_n(&pool_reg, &schema_reg, racine_reg);
        let ratio = compte as f64 / compte_reg as f64;
        ratios.push(ratio);
        assert!(
            (ratio - RATIO_GAUCHER).abs() < TOLERANCE_RATIO,
            "n={n}: left-handed ratio {ratio} ≈ {RATIO_GAUCHER}"
        );
    }
    // Ratio constancy over n = 4..7 (explicit falsification).
    let min = ratios.iter().copied().fold(f64::INFINITY, f64::min);
    let max = ratios.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max - min < TOLERANCE_RATIO,
        "ratio constant: min={min}, max={max}"
    );
}

/// M4-4: the **7 forms** = 5 legal cells + 2 terminals, independent
/// of n (O(1)); in contrast, the flat prototype (union of maps) has a number
/// of live nodes that grows with n (the edge, 4n − 4). Live peak and
/// firings read via the observer.
#[test]
fn test_bench_sept_formes_o1_et_bord() {
    let observateur = Arc::new(RecordingObserver::default());
    struct Pont(Arc<RecordingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    let mut comptes_formes = Vec::new();
    let mut noeuds_plats = Vec::new();
    let mut creations = 0u64;
    for &(n, _) in R_ATTENDUS {
        let schema = schema_anneau(n, false);
        let mut pool = Arena::new(
            ReferenceController::new(schema.clone()),
            Box::new(Pont(observateur.clone())),
        );
        let racine = construire_r_n(&mut pool, n, false);
        // Distinct (pc, left) signatures appearing in R_n.
        let mut vus: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
        for mot in mots_atteignables(n as usize, false) {
            for i in 0..n as usize {
                vus.insert((mot[i], fourchette(&mot, n as usize, i)));
            }
        }
        assert_eq!(
            vus.len(),
            NB_CELLULES_LEGALES,
            "n={n}: exactly 5 legal cells (7 forms with ⊥/⊤)"
        );
        comptes_formes.push(vus.len());
        // Flat prototype edge: live nodes of the pool (no GC here,
        // the live peak = live after construction).
        noeuds_plats.push(pool.vivants());
        creations += pool.metriques().creations;
        let _ = racine;
    }
    // O(1): the number of forms does not depend on n.
    assert!(
        comptes_formes.windows(2).all(|w| w[0] == w[1]),
        "forms constant in n: {comptes_formes:?}"
    );
    // The flat prototype edge is pinned exactly (explicit falsification)
    // and grows with n.
    for (&(n, attendu), &mesure) in BORD_PLAT.iter().zip(&noeuds_plats) {
        assert_eq!(mesure, attendu, "n={n}: flat edge = {attendu} live");
    }
    assert!(
        noeuds_plats.windows(2).all(|w| w[1] > w[0]),
        "the flat prototype grows with n: {noeuds_plats:?}"
    );
    // Firings = NodeCreated events == pool creation counter.
    let firings = *observateur.firings.lock().unwrap();
    assert_eq!(firings, creations, "firings == hash-consing creations");
    // Rounds = Round events: no saturation during the union construction
    // (the Round narration must not silently disappear).
    let passes = *observateur.passes.lock().unwrap();
    assert_eq!(passes, 0, "no saturation round during the union");
}
