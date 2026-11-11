//! Benchmark philosophes (M4-4, énoncé C8) : mesure contre SDD/HSDD,
//! docs/exemple-fil-rouge § 6-9. Prédictions falsifiables vérifiées ici :
//! |R_n| = 20/56/152/416/1136 (n = 3..7), variante gaucher |R'_n| =
//! 72/196/536/1464 (n = 4..7) avec ratio |R'_n|/|R_n| = 1,2887 **constant**,
//! 7 formes (5 cellules légales + 2 terminaux) indépendantes de n, et le
//! bord du prototype plat qui croît avec n (4n − 4 vivants). Pic vivant,
//! firings relevés via l'observateur. Toute mesure contraire fait échouer
//! le test : c'est la falsification explicite.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_algebra::interface::{Anchor, Form, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::{NoOpObserver, Observer, TraceEvent};
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

/// Ancre de test.
const ANCRE_TEST: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `hungry`.
const AFFAME: u64 = 1;
/// `eat`.
const MANGE: u64 = 2;
/// `g` : état intermédiaire du philosophe gaucher (position 0 seule).
const GAUCHER: u64 = 3;
/// |R_n| attendus (fil rouge § 0, vérifiés par simulation jusqu'à n = 20).
const R_ATTENDUS: &[(u64, u64)] = &[(3, 20), (4, 56), (5, 152), (6, 416), (7, 1136)];
/// |R'_n| attendus pour la variante gaucher (fil rouge § 9.2, n = 4..7).
const R_GAUCHER_ATTENDUS: &[(u64, u64)] = &[(4, 72), (5, 196), (6, 536), (7, 1464)];
/// Ratio prédit |R'_n| / |R_n|, constant (fil rouge § 9.4).
const RATIO_GAUCHER: f64 = 1.2887;
/// Tolérance sur le ratio (constance et valeur) — falsification explicite.
const TOLERANCE_RATIO: f64 = 0.01;
/// Nombre de cellules légales de l'automate minimal (5 + 2 terminaux = 7 formes).
const NB_CELLULES_LEGALES: usize = 5;
/// Pas des clés de l'intercalage : clé `2i` = `pc_i`, clé `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Nœuds vivants du prototype plat (union de maps) par n — bord mesuré,
/// croissant en n. Le 4n − 4 du fil rouge concerne le prototype plat de
/// l'automate encodé (hors périmètre de ce banc).
const BORD_PLAT: &[(u64, usize)] = &[(3, 206), (4, 646), (5, 1850), (6, 5198), (7, 14362)];

type PoolRef = Arena<ReferenceController>;

/// Observateur enregistreur : firings (NodeCreated) et passes (Round).
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

/// Schéma de l'anneau déroulé en intercalage (fil rouge § 2) : `2n` clés,
/// clé `2i` = `pc_i`, clé `2i+1` = `left_i` ; domaine {0,1,2} (+ g pour le
/// gaucher).
fn schema_anneau(n: u64, gaucher: bool) -> Schema {
    let domaine = if gaucher {
        Domaine::nouveau(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE), Atom(GAUCHER)])
    } else {
        Domaine::nouveau(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE)])
    };
    Schema::Map((0..PAS_CLE_INTERCALAGE * n).map(Atom).collect(), domaine)
}

/// Fourchette `left_i` dérivée du mot `pc` (fil rouge § 3).
fn fourchette(pc: &[u64], n: usize, i: usize) -> u64 {
    const LIBRE: u64 = 0;
    const TENUE: u64 = 1;
    const PRECEDENTE: u64 = 2;
    if pc[i] == MANGE {
        TENUE
    } else if pc[(i + n - 1) % n] == MANGE {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Énumère les mots circulaires atteignables (régulier ou gaucher).
fn mots_atteignables(n: usize, gaucher: bool) -> Vec<Vec<u64>> {
    let mut resultat = Vec::new();
    let mut mot = vec![PENSE; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(mot[i] == MANGE && mot[(i + 1) % n] == MANGE));
        let gaucher_ok = !gaucher || !(mot[0] == GAUCHER && mot[1] == MANGE);
        if circulaire_ok && gaucher_ok {
            resultat.push(mot.clone());
        }
        // Incrémentation en base 3 ; le gaucher a une 4e valeur `g` en
        // position 0 seule.
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
            mot[i] = 0;
        }
    }
}

/// Construit R_n (ou R'_n) dans le pool : union des maps compilées.
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
                        PAS_CLE_INTERCALAGE * i as u64 + 1,
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

/// Compte |R_n| par pli `(ℕ,+,×)` sur le schéma donné.
fn compter_r_n(pool_entree: &PoolRef, schema: &Schema, racine: NodeId) -> u64 {
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    compter(pool_entree, &mut sortie, ANCRE_TEST, schema, racine).unwrap()
}

/// M4-4 : |R_n| par pli DD == énumération == valeurs du fil rouge
/// (20/56/152/416/1136). Falsification explicite si la mesure dit autre
/// chose.
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
        assert_eq!(compte, reference, "n={n} : pli DD == énumération");
        assert_eq!(compte, attendu, "n={n} : |R_{n}| = {attendu} (fil rouge)");
    }
}

/// M4-4 : variante gaucher — |R'_n| = 72/196/536/1464 (n = 4..7) et ratio
/// |R'_n| / |R_n| = 1,2887 **constant** (la classe irrégulière décale la
/// constante, pas l'exposant). Falsification explicite hors tolérance.
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
        // |R'_n| par pli DD == énumération de référence == fil rouge § 9.2.
        let compte = compter_r_n(&pool, &schema, racine);
        let reference = mots_atteignables(n as usize, true).len() as u64;
        assert_eq!(compte, reference, "n={n} : pli DD == énumération (gaucher)");
        assert_eq!(
            reference, attendu,
            "n={n} : |R'_{n}| = {attendu} (fil rouge § 9.2)"
        );
        // |R_n| régulier pour le ratio.
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
            "n={n} : ratio gaucher {ratio} ≈ {RATIO_GAUCHER}"
        );
    }
    // Constance du ratio sur n = 4..7 (falsification explicite).
    let min = ratios.iter().copied().fold(f64::INFINITY, f64::min);
    let max = ratios.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max - min < TOLERANCE_RATIO,
        "ratio constant: min={min}, max={max}"
    );
}

/// M4-4 : les **7 formes** = 5 cellules légales + 2 terminaux, indépendantes
/// de n (O(1)) ; en contraste, le prototype plat (union de maps) a un nombre
/// de nœuds vivants qui croît avec n (le bord, 4n − 4). Pic vivant et
/// firings relevés via l'observateur.
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
        // Signatures (pc, left) distinctes apparaissant dans R_n.
        let mut vus: std::collections::BTreeSet<(u64, u64)> = std::collections::BTreeSet::new();
        for mot in mots_atteignables(n as usize, false) {
            for i in 0..n as usize {
                vus.insert((mot[i], fourchette(&mot, n as usize, i)));
            }
        }
        assert_eq!(
            vus.len(),
            NB_CELLULES_LEGALES,
            "n={n} : exactement 5 cellules légales (7 formes avec ⊥/⊤)"
        );
        comptes_formes.push(vus.len());
        // Bord du prototype plat : nœuds vivants du pool (pas de GC ici,
        // le pic vivant = vivants après construction).
        noeuds_plats.push(pool.vivants());
        creations += pool.metriques().creations;
        let _ = racine;
    }
    // O(1) : le nombre de formes ne dépend pas de n.
    assert!(
        comptes_formes.windows(2).all(|w| w[0] == w[1]),
        "formes constantes en n: {comptes_formes:?}"
    );
    // Le bord du prototype plat est épinglé exactement (falsification
    // explicite) et croît avec n.
    for (&(n, attendu), &mesure) in BORD_PLAT.iter().zip(&noeuds_plats) {
        assert_eq!(mesure, attendu, "n={n} : bord plat = {attendu} vivants");
    }
    assert!(
        noeuds_plats.windows(2).all(|w| w[1] > w[0]),
        "le prototype plat croît avec n: {noeuds_plats:?}"
    );
    // Firings = événements NodeCreated == compteur de créations du pool.
    let firings = *observateur.firings.lock().unwrap();
    assert_eq!(firings, creations, "firings == créations du hash-consing");
}
