//! Lisage ↔ external witnesses confrontation (ADR-0102, human decision of
//! 10 Nov. 2026: witnesses in local Docker containers, outside CI): on the
//! lisage side, the time/memory measurement harness over the BDD, MDD
//! variants and the sliding-window encodings. This module runs **one workload
//! exactly once** and returns a [`MesureConfrontation`]; the
//! `confrontation` binary serializes it as one JSON line on stdout; the
//! orchestrator `python -m scripts bench-confrontation` runs the repetitions
//! (a fresh process per measurement: the peak RSS of the whole process does
//! not divide into in-process repetitions) and aggregates.
//!
//! The same workloads are implemented at each witness (CUDD/Sylvan for
//! Boolean, libDDD for the Paris-school DDDs, Meddly for MDDs)
//! with the same convention: same input, same object built, same
//! cross-check value. The full protocol (fairness, metrics,
//! reproduction) lives in `docs/bench-confrontation-temoins-et-mesures.md`.
//!
//! Measured block, identical across all tools: input reading +
//! diagram construction + verification counting. Library initialization
//! (manager, workers) is outside the block: it varies from one
//! tool to another and is not the compared workload.

use std::collections::BTreeMap;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, Pool, PoolError};
use lisage_core::arena::Arena;
use lisage_core::atelier::{
    AtelierController, Encodage, Reductions, compter_chemins, compter_noeuds, construire_fenetre,
};
use lisage_core::bdd::{
    BddController, BddKeys, compiler_bdd, compter_noeuds as compter_noeuds_bdd,
};
use lisage_core::error::CoreError;
use lisage_core::observer::{CanalAttr, NoOpObserver, Observer, TraceEvent};
use lisage_core::pli::{compter as compter_pli, union};
use lisage_core::schema::{
    Domaine, ReferenceController, Schema, SchemaError, SchemaKeys, compiler,
};
use lisage_lang::value::{Atom, Value};

/// Measurement anchor (a single anchor per measurement, harness convention).
const ANCRE_MESURE: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `hungry` (intermediate value of the pc domain).
const AFFAME: u64 = 1;
/// `eat` (valeur interdite sur deux voisins circulairement).
const MANGE: u64 = 2;
/// Number of BDD variables of the `fenetre` workload controller (the
/// sliding window is read over 2 levels: root + window).
const VARIABLES_BDD_FENETRE: u32 = 2;
/// Step of the interleaving keys: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Seed of the deterministic truth-table LCG (same LCG as the M1-8 baseline:
/// `benches/micro.rs` and `cudd_local.rs`).
pub const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier (Knuth).
pub const MULTIPLICATEUR_LCG: u64 = 6_364_136_223_846_793_005;
/// LCG increment.
pub const INCREMENT_LCG: u64 = 1_442_695_040_888_963_407;
/// Maximum number of variables accepted by the `alea-lcg` workload (guard:
/// the truth table is `2^n` entries in memory).
const VARIABLES_MAXIMALES_ALEA: u32 = 24;
/// Maximum number of philosophers accepted by the `philosophes` workload
/// (guard: the interleaved schema has `2n` keys, R_n grows fast — the bench
/// measures n ≤ 7).
const PHILOSOPHES_MAXIMAUX: u64 = 32;
/// Seconds → milliseconds conversion of the measured block.
const MILLIS_PAR_SECONDE: f64 = 1_000.0;

/// Tool measured by this runner (fixed: the binary only measures lisage; the
/// external witnesses have their own binary with their own name).
pub const OUTIL: &str = "lisage";

/// Workload of the confrontation (closed set, ADR-0006).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Charge {
    /// Deterministic LCG truth table, BDD by Shannon expansion.
    AleaLcg,
    /// R_n of the red thread, union of maps over the interleaved schema.
    Philosophes,
    /// Sliding window, absolute or relative encoding.
    Fenetre,
}

impl Charge {
    /// Parses the workload name (binary edge); `None` if unknown.
    pub fn depuis_nom(nom: &str) -> Option<Charge> {
        match nom {
            "alea-lcg" => Some(Charge::AleaLcg),
            "philosophes" => Some(Charge::Philosophes),
            "fenetre" => Some(Charge::Fenetre),
            _ => None,
        }
    }

    /// JSON protocol label.
    pub fn etiquette(&self) -> &'static str {
        match self {
            Charge::AleaLcg => "alea-lcg",
            Charge::Philosophes => "philosophes",
            Charge::Fenetre => "fenetre",
        }
    }
}

/// Measured diagram variant (closed set).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variante {
    /// Boolean BDD.
    Bdd,
    /// MDD (finite-domain schema).
    Mdd,
}

impl Variante {
    /// JSON protocol label.
    pub fn etiquette(&self) -> &'static str {
        match self {
            Variante::Bdd => "bdd",
            Variante::Mdd => "mdd",
        }
    }
}

/// JSON label of a window encoding.
fn etiquette_encodage(encodage: &Encodage) -> &'static str {
    match encodage {
        Encodage::Absolu => "absolu",
        Encodage::Relatif => "relatif",
    }
}

/// One confrontation measurement: the result of a workload run once.
#[derive(Clone, PartialEq, Debug)]
pub struct MesureConfrontation {
    /// Measured diagram variant.
    pub variante: Variante,
    /// Workload run.
    pub charge: Charge,
    /// Workload parameters, in a stable order (serialized as an object).
    pub parametres: Vec<(String, String)>,
    /// Encoding of the `fenetre` workload only.
    pub encodage: Option<Encodage>,
    /// Time of the measured block, in milliseconds.
    pub temps_ms: f64,
    /// Peak process RSS, in kibibytes (`VmHWM`).
    pub rss_max_ko: u64,
    /// Nodes of the built diagram, terminals included (convention
    /// documented per tool in the protocol).
    pub noeuds: u64,
    /// Cross-check value: identical across all tools for the
    /// same workload (counted cardinal, satisfactions). Any divergence
    /// between tools invalidates the measurement.
    pub verifie: u64,
}

/// Typed error of the confrontation harness.
#[derive(Clone, PartialEq, Debug)]
pub enum ErreurConfrontation {
    /// Invalid parameters for the workload (number, type, bounds).
    ParametresInvalides {
        /// The workload concerned.
        charge: &'static str,
        /// The detail of the invalidity.
        detail: String,
    },
    /// Unreadable input (status file, `/proc` absent).
    EntreeInvalide {
        /// The detail of the invalidity.
        detail: String,
    },
    /// The internal cross-check failed: the counted value does
    /// not match the expected value.
    VerifieEchoue {
        /// The expected value.
        attendu: u64,
        /// The obtained value.
        obtenu: u64,
    },
    /// Core error (pool, pli, atelier), cause preserved.
    Interne {
        /// The core error.
        source: CoreError,
    },
    /// Schema error (compiling a value outside the domain), cause
    /// preserved.
    Schema {
        /// The schema error.
        source: SchemaError,
    },
}

impl std::fmt::Display for ErreurConfrontation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurConfrontation::ParametresInvalides { charge, detail } => {
                write!(formatter, "invalid parameters for {charge}: {detail}")
            }
            ErreurConfrontation::EntreeInvalide { detail } => {
                write!(formatter, "invalid input: {detail}")
            }
            ErreurConfrontation::VerifieEchoue { attendu, obtenu } => {
                write!(
                    formatter,
                    "cross-check failed: expected {attendu}, got {obtenu}"
                )
            }
            ErreurConfrontation::Interne { source } => {
                write!(formatter, "core error: {source}")
            }
            ErreurConfrontation::Schema { source } => {
                write!(formatter, "schema error: {source}")
            }
        }
    }
}

impl std::error::Error for ErreurConfrontation {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErreurConfrontation::Interne { source } => Some(source),
            ErreurConfrontation::Schema { source } => Some(source),
            _ => None,
        }
    }
}

impl From<CoreError> for ErreurConfrontation {
    fn from(source: CoreError) -> Self {
        ErreurConfrontation::Interne { source }
    }
}

impl From<PoolError> for ErreurConfrontation {
    fn from(source: PoolError) -> Self {
        ErreurConfrontation::Interne {
            source: CoreError::Pool(source),
        }
    }
}

impl From<SchemaError> for ErreurConfrontation {
    fn from(source: SchemaError) -> Self {
        ErreurConfrontation::Schema { source }
    }
}

/// Escapes a string for JSON (same minimal escaping as
/// `lisage-liseuse::trace`: the measured values are a controlled
/// vocabulary, the contract stays correct).
fn echapper_json(chaine: &str) -> String {
    let mut sortie = String::with_capacity(chaine.len() + 2);
    for caractere in chaine.chars() {
        match caractere {
            '"' => sortie.push_str("\\\""),
            '\\' => sortie.push_str("\\\\"),
            '\n' => sortie.push_str("\\n"),
            '\r' => sortie.push_str("\\r"),
            '\t' => sortie.push_str("\\t"),
            caractere_bas if (caractere_bas as u32) < 0x20 => {
                sortie.push_str(&format!("\\u{:04x}", caractere_bas as u32));
            }
            caractere_simple => sortie.push(caractere_simple),
        }
    }
    sortie
}

/// Serializes a measurement as one JSON line (without the trailing `\n`). The
/// key order is fixed and pinned by test: `outil`, `variante`, `charge`,
/// `parametres`, `encodage` (window only), `temps_ms`, `rss_max_ko`,
/// `noeuds`, `verifie`.
pub fn serialiser_ligne(mesure: &MesureConfrontation) -> String {
    let mut ligne = String::with_capacity(256);
    ligne.push('{');
    ligne.push_str(&format!(
        "\"outil\":\"{}\",\"variante\":\"{}\",\"charge\":\"{}\"",
        echapper_json(OUTIL),
        mesure.variante.etiquette(),
        mesure.charge.etiquette()
    ));
    ligne.push_str(",\"parametres\":{");
    for (indice, (cle, valeur)) in mesure.parametres.iter().enumerate() {
        if indice > 0 {
            ligne.push(',');
        }
        ligne.push_str(&format!(
            "\"{}\":\"{}\"",
            echapper_json(cle),
            echapper_json(valeur)
        ));
    }
    ligne.push('}');
    if let Some(encodage) = &mesure.encodage {
        ligne.push_str(&format!(
            ",\"encodage\":\"{}\"",
            etiquette_encodage(encodage)
        ));
    }
    ligne.push_str(&format!(
        ",\"temps_ms\":{:.3},\"rss_max_ko\":{},\"noeuds\":{},\"verifie\":{}",
        mesure.temps_ms, mesure.rss_max_ko, mesure.noeuds, mesure.verifie
    ));
    ligne.push('}');
    ligne
}

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
/// Linux only, documented in the protocol: local runs and the witnesses'
/// containers are Linux.
fn rss_max_ko() -> Result<u64, ErreurConfrontation> {
    const MARQUEUR: &str = "VmHWM:";
    let status = std::fs::read_to_string("/proc/self/status").map_err(|erreur| {
        ErreurConfrontation::EntreeInvalide {
            detail: format!("/proc/self/status unreadable: {erreur}"),
        }
    })?;
    for ligne in status.lines() {
        if let Some(valeurs) = ligne.strip_prefix(MARQUEUR) {
            return valeurs
                .split_whitespace()
                .next()
                .and_then(|valeur| valeur.parse::<u64>().ok())
                .ok_or_else(|| ErreurConfrontation::EntreeInvalide {
                    detail: format!("unexpected {MARQUEUR} line: {ligne}"),
                });
        }
    }
    Err(ErreurConfrontation::EntreeInvalide {
        detail: "VmHWM absent from /proc/self/status (non-Linux platform)".to_string(),
    })
}

/// Generates the deterministic LCG truth table over `n` variables (high bit
/// of the state, same LCG as the M1-8 baseline).
fn table_lcg(nombre_variables: u32) -> Vec<bool> {
    let mut etat = GRAINE_LCG;
    (0..(1usize << nombre_variables))
        .map(|_| {
            etat = etat
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            (etat >> 63) == 1
        })
        .collect()
}

/// `alea-lcg` workload: classic BDD of a deterministic random
/// truth table, built by Shannon expansion. The internal C6 falsification
/// is run: the arena's node count must equal the independent mini-BDD
/// witness's, and the evaluation must follow the table.
fn charge_alea_lcg(
    nombre_variables: u32,
    observateur: Option<&dyn Observer>,
) -> Result<MesureConfrontation, ErreurConfrontation> {
    if nombre_variables == 0 || nombre_variables > VARIABLES_MAXIMALES_ALEA {
        return Err(ErreurConfrontation::ParametresInvalides {
            charge: Charge::AleaLcg.etiquette(),
            detail: format!("n must be in 1..={VARIABLES_MAXIMALES_ALEA}"),
        });
    }
    let keys = BddKeys::new(nombre_variables);
    let mut pool = Arena::new(BddController::new(nombre_variables), Box::new(NoOpObserver));
    let debut = Instant::now();
    let table = table_lcg(nombre_variables);
    let expected_satisfactions = table.iter().filter(|valeur| **valeur).count() as u64;
    let racine = compiler_bdd(&mut pool, ANCRE_MESURE, &keys, &table);
    let noeuds = compter_noeuds_bdd(&pool, racine);
    let duree = debut.elapsed();
    // C6 falsification: the arena and the independent mini-BDD witness must
    // count the same number of nodes on the same table.
    let mut temoin = crate::temoin_bdd::MiniBdd::new();
    let racine_temoin = temoin.compiler_table(nombre_variables, &table);
    let noeuds_temoin = temoin.compter_noeuds(racine_temoin);
    if noeuds != noeuds_temoin {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu: noeuds_temoin,
            obtenu: noeuds,
        });
    }
    if temoin.evaluer(racine_temoin, 0) != table[0] {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu: table[0] as u64,
            obtenu: temoin.evaluer(racine_temoin, 0) as u64,
        });
    }
    if let Some(observateur) = observateur {
        observateur.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine,
            valeur: format!(
                "workload={} nodes={} check={}",
                Charge::AleaLcg.etiquette(),
                noeuds,
                expected_satisfactions
            ),
        });
    }
    Ok(MesureConfrontation {
        variante: Variante::Bdd,
        charge: Charge::AleaLcg,
        parametres: vec![("n".to_string(), nombre_variables.to_string())],
        encodage: None,
        temps_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        noeuds,
        verifie: expected_satisfactions,
    })
}

/// Extended state of a philosopher: `pc_i` (think/hungry/eat) and `left_i`
/// derived (free/held/previous), interleaved `pc_0 left_0 pc_1 …`.
type EtatEtendu = Vec<u64>;

/// `philosophes` workload: R_n of the golden thread (docs/golden-thread-example-n-philosophers-end-to-end) read from
/// the extended-states file shared by all tools (first line
/// `n <n>`, then one extended state per line, values separated by
/// spaces). Construction: union of the maps compiled over the
/// interleaved schema; the cardinal via the `(ℕ,+,×)` pli must be the number
/// of states in the file.
fn charge_philosophes(
    nombre_philosophes: u64,
    chemin: &str,
    observateur: Option<&dyn Observer>,
) -> Result<MesureConfrontation, ErreurConfrontation> {
    if nombre_philosophes == 0 || nombre_philosophes > PHILOSOPHES_MAXIMAUX {
        return Err(ErreurConfrontation::ParametresInvalides {
            charge: Charge::Philosophes.etiquette(),
            detail: format!("n must be in 1..={PHILOSOPHES_MAXIMAUX}"),
        });
    }
    // Init outside the measured block (schema, keys, arenas) — like the
    // manager init at the witnesses. Measured block: input reading +
    // construction + counting.
    let schema = Schema::Map(
        (0..PAS_CLE_INTERCALAGE * nombre_philosophes)
            .map(Atom)
            .collect(),
        Domaine::nouveau(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let debut = Instant::now();
    let contenu =
        std::fs::read_to_string(chemin).map_err(|erreur| ErreurConfrontation::EntreeInvalide {
            detail: format!("states file {chemin} unreadable: {erreur}"),
        })?;
    let etats = lire_etats(nombre_philosophes, &contenu)?;
    let attendu = etats.len() as u64;
    let mut racine = pool.intern(ANCRE_MESURE, Form::Terminal(false));
    for etat in &etats {
        let map: BTreeMap<Value, Value> = etat
            .iter()
            .enumerate()
            .map(|(indice, valeur)| (Value::Atom(Atom(indice as u64)), Value::Atom(Atom(*valeur))))
            .collect();
        let membre = compiler(&mut pool, ANCRE_MESURE, &schema, &Value::Map(map))?;
        racine = union(&mut pool, &keys, ANCRE_MESURE, racine, membre)?;
    }
    let verifie = compter_pli(&pool, &mut sortie, ANCRE_MESURE, &schema, racine)?;
    let noeuds = compter_noeuds(&pool, racine)?;
    let duree = debut.elapsed();
    if verifie != attendu {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu,
            obtenu: verifie,
        });
    }
    if let Some(observateur) = observateur {
        observateur.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine,
            valeur: format!(
                "workload={} nodes={} check={}",
                Charge::Philosophes.etiquette(),
                noeuds,
                verifie
            ),
        });
    }
    Ok(MesureConfrontation {
        variante: Variante::Mdd,
        charge: Charge::Philosophes,
        parametres: vec![("n".to_string(), nombre_philosophes.to_string())],
        encodage: None,
        temps_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        noeuds,
        verifie,
    })
}

/// Reads and validates the extended-states file: first line `n <n>`, then
/// `2n` values per line in `{0, 1, 2}`.
fn lire_etats(
    nombre_philosophes: u64,
    contenu: &str,
) -> Result<Vec<EtatEtendu>, ErreurConfrontation> {
    let nombre_valeurs = PAS_CLE_INTERCALAGE as usize * nombre_philosophes as usize;
    let mut lignes = contenu.lines();
    let premiere = lignes
        .next()
        .ok_or_else(|| ErreurConfrontation::EntreeInvalide {
            detail: "empty states file".to_string(),
        })?;
    let champs_premiere: Vec<&str> = premiere.split_whitespace().collect();
    if champs_premiere.len() != 2 || champs_premiere[0] != "n" {
        return Err(ErreurConfrontation::EntreeInvalide {
            detail: format!("expected first line \"n <n>\", got: {premiere}"),
        });
    }
    let n_declare: u64 =
        champs_premiere[1]
            .parse()
            .map_err(|_| ErreurConfrontation::EntreeInvalide {
                detail: format!("unreadable n: {}", champs_premiere[1]),
            })?;
    if n_declare != nombre_philosophes {
        return Err(ErreurConfrontation::EntreeInvalide {
            detail: format!("declared n {n_declare} ≠ requested n {nombre_philosophes}"),
        });
    }
    let mut etats = Vec::new();
    for (numero, ligne) in lignes.enumerate() {
        if ligne.trim().is_empty() {
            continue;
        }
        let etat: Result<Vec<u64>, _> = ligne
            .split_whitespace()
            .map(|valeur| valeur.parse::<u64>())
            .collect();
        let etat = etat.map_err(|_| ErreurConfrontation::EntreeInvalide {
            detail: format!("line {}: unreadable value", numero + 2),
        })?;
        if etat.len() != nombre_valeurs {
            return Err(ErreurConfrontation::EntreeInvalide {
                detail: format!(
                    "line {}: {} values, {} expected",
                    numero + 2,
                    etat.len(),
                    nombre_valeurs
                ),
            });
        }
        if etat.iter().any(|valeur| *valeur > MANGE) {
            return Err(ErreurConfrontation::EntreeInvalide {
                detail: format!("line {}: value outside the domain {{0,1,2}}", numero + 2),
            });
        }
        etats.push(etat);
    }
    Ok(etats)
}

/// `fenetre` workload: the sliding window of golden thread n° 2 in the
/// requested encoding. Check: the root → terminal paths must be
/// `taille × nb_bases` (one window per base, `taille` positions per
/// window), whatever the encoding — only the number of shared nodes
/// differs (absolute: one node per base; relative: a single form, O(1) in
/// `nb_bases`).
fn charge_fenetre(
    taille: u64,
    nb_bases: u64,
    encodage: Encodage,
    observateur: Option<&dyn Observer>,
) -> Result<MesureConfrontation, ErreurConfrontation> {
    if taille == 0 || nb_bases == 0 {
        return Err(ErreurConfrontation::ParametresInvalides {
            charge: Charge::Fenetre.etiquette(),
            detail: "taille and nb_bases must be ≥ 1".to_string(),
        });
    }
    let attendu = taille.saturating_mul(nb_bases);
    let mut pool = Arena::new(
        AtelierController::nouveau(
            BddController::new(VARIABLES_BDD_FENETRE),
            Reductions::sans_collapse(),
        ),
        Box::new(NoOpObserver),
    );
    let debut = Instant::now();
    let racine = construire_fenetre(&mut pool, ANCRE_MESURE, encodage, taille, nb_bases);
    let verifie = compter_chemins(&pool, racine)?;
    let noeuds = compter_noeuds(&pool, racine)?;
    let duree = debut.elapsed();
    if verifie != attendu {
        return Err(ErreurConfrontation::VerifieEchoue {
            attendu,
            obtenu: verifie,
        });
    }
    if let Some(observateur) = observateur {
        observateur.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine,
            valeur: format!(
                "workload={} encoding={} nodes={} check={}",
                Charge::Fenetre.etiquette(),
                etiquette_encodage(&encodage),
                noeuds,
                verifie
            ),
        });
    }
    Ok(MesureConfrontation {
        variante: Variante::Mdd,
        charge: Charge::Fenetre,
        parametres: vec![
            ("taille".to_string(), taille.to_string()),
            ("nb_bases".to_string(), nb_bases.to_string()),
        ],
        encodage: Some(encodage),
        temps_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        noeuds,
        verifie,
    })
}

/// Runs a workload once and returns the measurement. Parameters:
/// `alea-lcg <n>`, `philosophes <n> <path>`,
/// `fenetre <taille> <nb_bases> [absolu|relatif]` (absolu by default).
/// `observateur`: narration channel (ADR-0100) — an
/// `Attr`/`Synthétise` event carries the workload summary.
pub fn executer(
    charge: &Charge,
    parametres: &[String],
    observateur: Option<&dyn Observer>,
) -> Result<MesureConfrontation, ErreurConfrontation> {
    match charge {
        Charge::AleaLcg => {
            let seul =
                parametres
                    .first()
                    .ok_or_else(|| ErreurConfrontation::ParametresInvalides {
                        charge: Charge::AleaLcg.etiquette(),
                        detail: "expected: alea-lcg <n>".to_string(),
                    })?;
            let nombre_variables =
                seul.parse::<u32>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: Charge::AleaLcg.etiquette(),
                        detail: format!("unreadable n: {seul}"),
                    })?;
            charge_alea_lcg(nombre_variables, observateur)
        }
        Charge::Philosophes => {
            let (premier, second) = parametres.first().zip(parametres.get(1)).ok_or_else(|| {
                ErreurConfrontation::ParametresInvalides {
                    charge: Charge::Philosophes.etiquette(),
                    detail: "expected: philosophes <n> <path>".to_string(),
                }
            })?;
            let nombre_philosophes =
                premier
                    .parse::<u64>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: Charge::Philosophes.etiquette(),
                        detail: format!("unreadable n: {premier}"),
                    })?;
            charge_philosophes(nombre_philosophes, second, observateur)
        }
        Charge::Fenetre => {
            let (premier, second) = parametres.first().zip(parametres.get(1)).ok_or_else(|| {
                ErreurConfrontation::ParametresInvalides {
                    charge: Charge::Fenetre.etiquette(),
                    detail: "expected: fenetre <taille> <nb_bases> [absolu|relatif]".to_string(),
                }
            })?;
            let taille =
                premier
                    .parse::<u64>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: Charge::Fenetre.etiquette(),
                        detail: format!("unreadable taille: {premier}"),
                    })?;
            let nb_bases =
                second
                    .parse::<u64>()
                    .map_err(|_| ErreurConfrontation::ParametresInvalides {
                        charge: Charge::Fenetre.etiquette(),
                        detail: format!("unreadable nb_bases: {second}"),
                    })?;
            let encodage = match parametres.get(2).map(String::as_str) {
                None | Some("absolu") => Encodage::Absolu,
                Some("relatif") => Encodage::Relatif,
                Some(autre) => {
                    return Err(ErreurConfrontation::ParametresInvalides {
                        charge: Charge::Fenetre.etiquette(),
                        detail: format!("unknown encoding: {autre}"),
                    });
                }
            };
            charge_fenetre(taille, nb_bases, encodage, observateur)
        }
    }
}
