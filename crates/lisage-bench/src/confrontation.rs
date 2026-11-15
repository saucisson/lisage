//! Lisage ↔ external witnesses confrontation (ADR-0102, human decision of
//! 10 Nov. 2026: witnesses in local Docker containers, outside CI): on the
//! lisage side, the time/memory measurement harness over the BDD, MDD
//! variants and the sliding-window encodings. This module runs **one workload
//! exactly once** and returns a [`MeasureConfrontation`]; the
//! `confrontation` binary serializes it as one JSON line on stdout; the
//! orchestrator `python -m scripts bench-confrontation` runs the repetitions
//! (a fresh process per measurement: the peak RSS of the whole process does
//! not divide into in-process repetitions) and aggregates.
//!
//! The same workloads are implemented at each witness (CUDD/Sylvan for
//! Boolean, libDDD for the Paris-school DDDs, Meddly for MDDs)
//! with the same convention: same input, same object built, same
//! cross-check value. The full protocol (fairness, metrics,
//! reproduction) lives in `docs/bench-confrontation-witnesses-and-measures.md`.
//!
//! Measured block, identical across all tools: input reading +
//! diagram construction + verification counting. Library initialization
//! (manager, workers) is outside the block: it varies from one
//! tool to another and is not the compared workload.

use std::collections::BTreeMap;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, Pool, PoolError};
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compiler_bdd, count_nodes as count_nodes_bdd};
use lisage_core::error::CoreError;
use lisage_core::observer::{ChannelAttr, NoOpObserver, Observer, TraceEvent};
use lisage_core::pli::{count as count_pli, union};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaError, SchemaKeys, compiler};
use lisage_core::workshop::{
    Encodage, Reductions, WorkshopController, build_window, count_nodes, count_paths,
};
use lisage_lang::value::{Atom, Value};

/// Measurement anchor (a single anchor per measurement, harness convention).
const ANCHOR_MEASURE: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `hungry` (intermediate value of the pc domain).
const AFFAME: u64 = 1;
/// `eat` (value interdite sur deux voisins circulairement).
const MANGE: u64 = 2;
/// Number of BDD variables of the `window` workload controller (the
/// sliding window is read over 2 levels: root + window).
const VARIABLES_BDD_WINDOW: u32 = 2;
/// Step of the interleaving keys: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Seed of the deterministic truth-table LCG (same LCG as the M1-8 baseline:
/// `benches/micro.rs` and `cudd_local.rs`).
pub const SEED_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier (Knuth).
pub const MULTIPLICATEUR_LCG: u64 = 6_364_136_223_846_793_005;
/// LCG increment.
pub const INCREMENT_LCG: u64 = 1_442_695_040_888_963_407;
/// Maximum number of variables accepted by the `alea-lcg` workload (guard:
/// the truth table is `2^n` entries in memory).
const VARIABLES_MAXIMALES_ALEA: u32 = 24;
/// Maximum number of philosophers accepted by the `philosophers` workload
/// (guard: the interleaved schema has `2n` keys, R_n grows fast — the bench
/// measures n ≤ 7).
const PHILOSOPHERS_MAXIMAUX: u64 = 32;
/// Seconds → milliseconds conversion of the measured block.
const MILLIS_PAR_SECONDE: f64 = 1_000.0;

/// Tool measured by this runner (fixed: the binary only measures lisage; the
/// external witnesses have their own binary with their own name).
pub const TOOL: &str = "lisage";

/// Workload of the confrontation (closed set, ADR-0006).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Workload {
    /// Deterministic LCG truth table, BDD by Shannon expansion.
    AleaLcg,
    /// R_n of the red thread, union of maps over the interleaved schema.
    Philosophers,
    /// Sliding window, absolute or relative encoding.
    Window,
}

impl Workload {
    /// Parses the workload name (binary edge); `None` if unknown.
    pub fn depuis_name(name: &str) -> Option<Workload> {
        match name {
            "alea-lcg" => Some(Workload::AleaLcg),
            "philosophers" => Some(Workload::Philosophers),
            "window" => Some(Workload::Window),
            _ => None,
        }
    }

    /// JSON protocol label.
    pub fn label(&self) -> &'static str {
        match self {
            Workload::AleaLcg => "alea-lcg",
            Workload::Philosophers => "philosophers",
            Workload::Window => "window",
        }
    }
}

/// Measured diagram variant (closed set).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// Boolean BDD.
    Bdd,
    /// MDD (finite-domain schema).
    Mdd,
}

impl Variant {
    /// JSON protocol label.
    pub fn label(&self) -> &'static str {
        match self {
            Variant::Bdd => "bdd",
            Variant::Mdd => "mdd",
        }
    }
}

/// JSON label of a window encoding.
fn label_encodage(encodage: &Encodage) -> &'static str {
    match encodage {
        Encodage::Absolu => "absolu",
        Encodage::Relatif => "relatif",
    }
}

/// One confrontation measurement: the result of a workload run once.
#[derive(Clone, PartialEq, Debug)]
pub struct MeasureConfrontation {
    /// Measured diagram variant.
    pub variant: Variant,
    /// Workload run.
    pub workload: Workload,
    /// Workload parameters, in a stable order (serialized as an object).
    pub parametres: Vec<(String, String)>,
    /// Encoding of the `window` workload only.
    pub encodage: Option<Encodage>,
    /// Time of the measured block, in milliseconds.
    pub temps_ms: f64,
    /// Peak process RSS, in kibibytes (`VmHWM`).
    pub rss_max_ko: u64,
    /// Nodes of the built diagram, terminals included (convention
    /// documented per tool in the protocol).
    pub nodes: u64,
    /// Cross-check value: identical across all tools for the
    /// same workload (counted cardinality, satisfactions). Any divergence
    /// between tools invalidates the measurement.
    pub verifie: u64,
}

/// Typed error of the confrontation harness.
#[derive(Clone, PartialEq, Debug)]
pub enum ErrorConfrontation {
    /// Invalid parameters for the workload (number, type, bounds).
    ParametresInvalides {
        /// The workload concerned.
        workload: &'static str,
        /// The detail of the invalidity.
        detail: String,
    },
    /// Unreadable input (status file, `/proc` absent).
    InvalidInput {
        /// The detail of the invalidity.
        detail: String,
    },
    /// The internal cross-check failed: the counted value does
    /// not match the expected value.
    VerifieEchoue {
        /// The expected value.
        expected: u64,
        /// The obtained value.
        obtenu: u64,
    },
    /// Core error (pool, pli, workshop), cause preserved.
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

impl std::fmt::Display for ErrorConfrontation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorConfrontation::ParametresInvalides { workload, detail } => {
                write!(formatter, "invalid parameters for {workload}: {detail}")
            }
            ErrorConfrontation::InvalidInput { detail } => {
                write!(formatter, "invalid input: {detail}")
            }
            ErrorConfrontation::VerifieEchoue { expected, obtenu } => {
                write!(
                    formatter,
                    "cross-check failed: expected {expected}, got {obtenu}"
                )
            }
            ErrorConfrontation::Interne { source } => {
                write!(formatter, "core error: {source}")
            }
            ErrorConfrontation::Schema { source } => {
                write!(formatter, "schema error: {source}")
            }
        }
    }
}

impl std::error::Error for ErrorConfrontation {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrorConfrontation::Interne { source } => Some(source),
            ErrorConfrontation::Schema { source } => Some(source),
            _ => None,
        }
    }
}

impl From<CoreError> for ErrorConfrontation {
    fn from(source: CoreError) -> Self {
        ErrorConfrontation::Interne { source }
    }
}

impl From<PoolError> for ErrorConfrontation {
    fn from(source: PoolError) -> Self {
        ErrorConfrontation::Interne {
            source: CoreError::Pool(source),
        }
    }
}

impl From<SchemaError> for ErrorConfrontation {
    fn from(source: SchemaError) -> Self {
        ErrorConfrontation::Schema { source }
    }
}

/// Escapes a string for JSON (same minimal escaping as
/// `lisage-liseuse::trace`: the measured values are a controlled
/// vocabulary, the contract stays correct).
fn echapper_json(chaine: &str) -> String {
    let mut output = String::with_capacity(chaine.len() + 2);
    for caractere in chaine.chars() {
        match caractere {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            caractere_bas if (caractere_bas as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04x}", caractere_bas as u32));
            }
            caractere_simple => output.push(caractere_simple),
        }
    }
    output
}

/// Serializes a measurement as one JSON line (without the trailing `\n`). The
/// key order is fixed and pinned by test: `outil`, `variante`, `charge`,
/// `parametres`, `encodage` (window only), `temps_ms`, `rss_max_ko`,
/// `nodes`, `verifie`.
pub fn serialiser_ligne(measure: &MeasureConfrontation) -> String {
    let mut ligne = String::with_capacity(256);
    ligne.push('{');
    ligne.push_str(&format!(
        "\"outil\":\"{}\",\"variante\":\"{}\",\"charge\":\"{}\"",
        echapper_json(TOOL),
        measure.variant.label(),
        measure.workload.label()
    ));
    ligne.push_str(",\"parametres\":{");
    for (indice, (key, value)) in measure.parametres.iter().enumerate() {
        if indice > 0 {
            ligne.push(',');
        }
        ligne.push_str(&format!(
            "\"{}\":\"{}\"",
            echapper_json(key),
            echapper_json(value)
        ));
    }
    ligne.push('}');
    if let Some(encodage) = &measure.encodage {
        ligne.push_str(&format!(",\"encodage\":\"{}\"", label_encodage(encodage)));
    }
    ligne.push_str(&format!(
        ",\"temps_ms\":{:.3},\"rss_max_ko\":{},\"noeuds\":{},\"verifie\":{}",
        measure.temps_ms, measure.rss_max_ko, measure.nodes, measure.verifie
    ));
    ligne.push('}');
    ligne
}

/// Peak process RSS in kibibytes (`VmHWM` of `/proc/self/status`).
/// Linux only, documented in the protocol: local runs and the witnesses'
/// containers are Linux.
fn rss_max_ko() -> Result<u64, ErrorConfrontation> {
    const MARKER: &str = "VmHWM:";
    let status = std::fs::read_to_string("/proc/self/status").map_err(|error| {
        ErrorConfrontation::InvalidInput {
            detail: format!("/proc/self/status unreadable: {error}"),
        }
    })?;
    for ligne in status.lines() {
        if let Some(values) = ligne.strip_prefix(MARKER) {
            return values
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| ErrorConfrontation::InvalidInput {
                    detail: format!("unexpected {MARKER} line: {ligne}"),
                });
        }
    }
    Err(ErrorConfrontation::InvalidInput {
        detail: "VmHWM absent from /proc/self/status (non-Linux platform)".to_string(),
    })
}

/// Generates the deterministic LCG truth table over `n` variables (high bit
/// of the state, same LCG as the M1-8 baseline).
fn table_lcg(number_variables: u32) -> Vec<bool> {
    let mut etat = SEED_LCG;
    (0..(1usize << number_variables))
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
fn workload_alea_lcg(
    number_variables: u32,
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    if number_variables == 0 || number_variables > VARIABLES_MAXIMALES_ALEA {
        return Err(ErrorConfrontation::ParametresInvalides {
            workload: Workload::AleaLcg.label(),
            detail: format!("n must be in 1..={VARIABLES_MAXIMALES_ALEA}"),
        });
    }
    let keys = BddKeys::new(number_variables);
    let mut pool = Arena::new(BddController::new(number_variables), Box::new(NoOpObserver));
    let debut = Instant::now();
    let table = table_lcg(number_variables);
    let expected_satisfactions = table.iter().filter(|value| **value).count() as u64;
    let root = compiler_bdd(&mut pool, ANCHOR_MEASURE, &keys, &table);
    let nodes = count_nodes_bdd(&pool, root);
    let duree = debut.elapsed();
    // C6 falsification: the arena and the independent mini-BDD witness must
    // count the same number of nodes on the same table.
    let mut temoin = crate::temoin_bdd::MiniBdd::new();
    let root_temoin = temoin.compiler_table(number_variables, &table);
    let nodes_temoin = temoin.count_nodes(root_temoin);
    if nodes != nodes_temoin {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected: nodes_temoin,
            obtenu: nodes,
        });
    }
    if temoin.evaluate(root_temoin, 0) != table[0] {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected: table[0] as u64,
            obtenu: temoin.evaluate(root_temoin, 0) as u64,
        });
    }
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node: root,
            value: format!(
                "workload={} nodes={} check={}",
                Workload::AleaLcg.label(),
                nodes,
                expected_satisfactions
            ),
        });
    }
    Ok(MeasureConfrontation {
        variant: Variant::Bdd,
        workload: Workload::AleaLcg,
        parametres: vec![("n".to_string(), number_variables.to_string())],
        encodage: None,
        temps_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        nodes,
        verifie: expected_satisfactions,
    })
}

/// Extended state of a philosopher: `pc_i` (think/hungry/eat) and `left_i`
/// derived (free/held/previous), interleaved `pc_0 left_0 pc_1 …`.
type EtatEtendu = Vec<u64>;

/// `philosophers` workload: R_n of the golden thread (docs/golden-thread-example-n-philosophers-end-to-end) read from
/// the extended-states file shared by all tools (first line
/// `n <n>`, then one extended state per line, values separated by
/// spaces). Construction: union of the maps compiled over the
/// interleaved schema; the cardinality via the `(ℕ,+,×)` pli must be the number
/// of states in the file.
fn workload_philosophers(
    number_philosophers: u64,
    path: &str,
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    if number_philosophers == 0 || number_philosophers > PHILOSOPHERS_MAXIMAUX {
        return Err(ErrorConfrontation::ParametresInvalides {
            workload: Workload::Philosophers.label(),
            detail: format!("n must be in 1..={PHILOSOPHERS_MAXIMAUX}"),
        });
    }
    // Init outside the measured block (schema, keys, arenas) — like the
    // manager init at the witnesses. Measured block: input reading +
    // construction + counting.
    let schema = Schema::Map(
        (0..PAS_CLE_INTERCALAGE * number_philosophers)
            .map(Atom)
            .collect(),
        Domain::new(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let debut = Instant::now();
    let content =
        std::fs::read_to_string(path).map_err(|error| ErrorConfrontation::InvalidInput {
            detail: format!("states file {path} unreadable: {error}"),
        })?;
    let etats = read_etats(number_philosophers, &content)?;
    let expected = etats.len() as u64;
    let mut root = pool.intern(ANCHOR_MEASURE, Form::Terminal(false));
    for etat in &etats {
        let map: BTreeMap<Value, Value> = etat
            .iter()
            .enumerate()
            .map(|(indice, value)| (Value::Atom(Atom(indice as u64)), Value::Atom(Atom(*value))))
            .collect();
        let membre = compiler(&mut pool, ANCHOR_MEASURE, &schema, &Value::Map(map))?;
        root = union(&mut pool, &keys, ANCHOR_MEASURE, root, membre)?;
    }
    let verifie = count_pli(&pool, &mut output, ANCHOR_MEASURE, &schema, root)?;
    let nodes = count_nodes(&pool, root)?;
    let duree = debut.elapsed();
    if verifie != expected {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected,
            obtenu: verifie,
        });
    }
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node: root,
            value: format!(
                "workload={} nodes={} check={}",
                Workload::Philosophers.label(),
                nodes,
                verifie
            ),
        });
    }
    Ok(MeasureConfrontation {
        variant: Variant::Mdd,
        workload: Workload::Philosophers,
        parametres: vec![("n".to_string(), number_philosophers.to_string())],
        encodage: None,
        temps_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        nodes,
        verifie,
    })
}

/// Reads and validates the extended-states file: first line `n <n>`, then
/// `2n` values per line in `{0, 1, 2}`.
fn read_etats(
    number_philosophers: u64,
    content: &str,
) -> Result<Vec<EtatEtendu>, ErrorConfrontation> {
    let number_values = PAS_CLE_INTERCALAGE as usize * number_philosophers as usize;
    let mut lignes = content.lines();
    let first = lignes
        .next()
        .ok_or_else(|| ErrorConfrontation::InvalidInput {
            detail: "empty states file".to_string(),
        })?;
    let champs_first: Vec<&str> = first.split_whitespace().collect();
    if champs_first.len() != 2 || champs_first[0] != "n" {
        return Err(ErrorConfrontation::InvalidInput {
            detail: format!("expected first line \"n <n>\", got: {first}"),
        });
    }
    let n_declare: u64 = champs_first[1]
        .parse()
        .map_err(|_| ErrorConfrontation::InvalidInput {
            detail: format!("unreadable n: {}", champs_first[1]),
        })?;
    if n_declare != number_philosophers {
        return Err(ErrorConfrontation::InvalidInput {
            detail: format!("declared n {n_declare} ≠ requested n {number_philosophers}"),
        });
    }
    let mut etats = Vec::new();
    for (numero, ligne) in lignes.enumerate() {
        if ligne.trim().is_empty() {
            continue;
        }
        let etat: Result<Vec<u64>, _> = ligne
            .split_whitespace()
            .map(|value| value.parse::<u64>())
            .collect();
        let etat = etat.map_err(|_| ErrorConfrontation::InvalidInput {
            detail: format!("line {}: unreadable value", numero + 2),
        })?;
        if etat.len() != number_values {
            return Err(ErrorConfrontation::InvalidInput {
                detail: format!(
                    "line {}: {} values, {} expected",
                    numero + 2,
                    etat.len(),
                    number_values
                ),
            });
        }
        if etat.iter().any(|value| *value > MANGE) {
            return Err(ErrorConfrontation::InvalidInput {
                detail: format!("line {}: value outside the domain {{0,1,2}}", numero + 2),
            });
        }
        etats.push(etat);
    }
    Ok(etats)
}

/// `window` workload: the sliding window of golden thread n° 2 in the
/// requested encoding. Check: the root → terminal paths must be
/// `size × nb_bases` (one window per base, `size` positions per
/// window), whatever the encoding — only the number of shared nodes
/// differs (absolute: one node per base; relative: a single form, O(1) in
/// `nb_bases`).
fn workload_window(
    size: u64,
    nb_bases: u64,
    encodage: Encodage,
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    if size == 0 || nb_bases == 0 {
        return Err(ErrorConfrontation::ParametresInvalides {
            workload: Workload::Window.label(),
            detail: "size and nb_bases must be ≥ 1".to_string(),
        });
    }
    let expected = size.saturating_mul(nb_bases);
    let mut pool = Arena::new(
        WorkshopController::new(
            BddController::new(VARIABLES_BDD_WINDOW),
            Reductions::without_collapse(),
        ),
        Box::new(NoOpObserver),
    );
    let debut = Instant::now();
    let root = build_window(&mut pool, ANCHOR_MEASURE, encodage, size, nb_bases);
    let verifie = count_paths(&pool, root)?;
    let nodes = count_nodes(&pool, root)?;
    let duree = debut.elapsed();
    if verifie != expected {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected,
            obtenu: verifie,
        });
    }
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::Attr {
            channel: ChannelAttr::Synthesized,
            node: root,
            value: format!(
                "workload={} encoding={} nodes={} check={}",
                Workload::Window.label(),
                label_encodage(&encodage),
                nodes,
                verifie
            ),
        });
    }
    Ok(MeasureConfrontation {
        variant: Variant::Mdd,
        workload: Workload::Window,
        parametres: vec![
            ("taille".to_string(), size.to_string()),
            ("nb_bases".to_string(), nb_bases.to_string()),
        ],
        encodage: Some(encodage),
        temps_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        nodes,
        verifie,
    })
}

/// Runs a workload once and returns the measurement. Parameters:
/// `alea-lcg <n>`, `philosophers <n> <path>`,
/// `window <size> <nb_bases> [absolu|relatif]` (absolu by default).
/// `observer`: narration channel (ADR-0100) — an
/// `Attr`/`Synthesized` event carries the workload summary.
pub fn executer(
    workload: &Workload,
    parametres: &[String],
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    match workload {
        Workload::AleaLcg => {
            let seul =
                parametres
                    .first()
                    .ok_or_else(|| ErrorConfrontation::ParametresInvalides {
                        workload: Workload::AleaLcg.label(),
                        detail: "expected: alea-lcg <n>".to_string(),
                    })?;
            let number_variables =
                seul.parse::<u32>()
                    .map_err(|_| ErrorConfrontation::ParametresInvalides {
                        workload: Workload::AleaLcg.label(),
                        detail: format!("unreadable n: {seul}"),
                    })?;
            workload_alea_lcg(number_variables, observer)
        }
        Workload::Philosophers => {
            let (first, second) = parametres.first().zip(parametres.get(1)).ok_or_else(|| {
                ErrorConfrontation::ParametresInvalides {
                    workload: Workload::Philosophers.label(),
                    detail: "expected: philosophers <n> <path>".to_string(),
                }
            })?;
            let number_philosophers =
                first
                    .parse::<u64>()
                    .map_err(|_| ErrorConfrontation::ParametresInvalides {
                        workload: Workload::Philosophers.label(),
                        detail: format!("unreadable n: {first}"),
                    })?;
            workload_philosophers(number_philosophers, second, observer)
        }
        Workload::Window => {
            let (first, second) = parametres.first().zip(parametres.get(1)).ok_or_else(|| {
                ErrorConfrontation::ParametresInvalides {
                    workload: Workload::Window.label(),
                    detail: "expected: window <size> <nb_bases> [absolu|relatif]".to_string(),
                }
            })?;
            let size =
                first
                    .parse::<u64>()
                    .map_err(|_| ErrorConfrontation::ParametresInvalides {
                        workload: Workload::Window.label(),
                        detail: format!("unreadable size: {first}"),
                    })?;
            let nb_bases =
                second
                    .parse::<u64>()
                    .map_err(|_| ErrorConfrontation::ParametresInvalides {
                        workload: Workload::Window.label(),
                        detail: format!("unreadable nb_bases: {second}"),
                    })?;
            let encodage = match parametres.get(2).map(String::as_str) {
                None | Some("absolu") => Encodage::Absolu,
                Some("relatif") => Encodage::Relatif,
                Some(other) => {
                    return Err(ErrorConfrontation::ParametresInvalides {
                        workload: Workload::Window.label(),
                        detail: format!("unknown encoding: {other}"),
                    });
                }
            };
            workload_window(size, nb_bases, encodage, observer)
        }
    }
}
