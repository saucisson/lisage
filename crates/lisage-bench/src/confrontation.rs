//! Lisage ↔ external witnesses confrontation (ADR-0102, human decision of
//! 2026-11-10: witnesses in local Docker containers, outside CI): the
//! lisage side of the time/memory measurement harness over BDD and MDD
//! variants and sliding-window encodings. This module runs **one workload
//! exactly once** and returns a [`MeasureConfrontation`]; the
//! `confrontation` binary serializes it as one JSON line on stdout; the
//! orchestrator `python -m scripts bench-confrontation` runs the repetitions
//! (one fresh process per measurement: whole-process peak RSS does not
//! divide into in-process repetitions) and aggregates.
//!
//! The same workloads are implemented at each witness (CUDD/Sylvan for
//! Boolean, libDDD for the Paris-school DDDs, Meddly for MDDs) with the
//! same convention: same input, same object built, same cross-check value.
//! The full protocol (fairness, metrics, reproduction) lives in
//! `docs/bench-confrontation-temoins-and-mesures.md`.
//!
//! Measured block, identical across all tools: input reading + diagram
//! construction + verification counting. Library initialization (manager,
//! workers) is outside the block: it varies from tool to tool and is not
//! the compared workload.

use std::collections::BTreeMap;
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, Pool, PoolError};
use lisage_core::arena::Arena;
use lisage_core::bdd::{BddController, BddKeys, compile_bdd, count_nodes as count_nodes_bdd};
use lisage_core::error::CoreError;
use lisage_core::fold::{count as count_fold, union};
use lisage_core::observer::{ChannelAttribute, NoOpObserver, Observer, TraceEvent};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaError, SchemaKeys, compiled};
use lisage_core::workshop::{
    Encoding, Reductions, WorkshopController, build_window, count_nodes, count_paths,
};
use lisage_lang::value::{Atom, Value};

/// Measurement anchor (a single anchor per measurement, harness convention).
const ANCRE_MESURE: Anchor = Anchor(0);
/// `think`.
const PENSE: u64 = 0;
/// `hungry` (intermediate value of the pc domain).
const AFFAME: u64 = 1;
/// `eat` (value forbidden on two circular neighbors).
const EATS: u64 = 2;
/// Number of BDD variables of the `window` workload controller (the
/// sliding window is read over 2 levels: root + window).
const VARIABLES_BDD_FENETRE: u32 = 2;
/// Stride of the interleaving: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Seed of the deterministic truth-table LCG (same LCG as the M1-8
/// baseline: `benches/micro.rs` and `cudd_local.rs`).
pub const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier (Knuth).
pub const MULTIPLICATEUR_LCG: u64 = 6_364_136_223_846_793_005;
/// LCG increment.
pub const INCREMENT_LCG: u64 = 1_442_695_040_888_963_407;
/// Maximum number of variables accepted by the `random-lcg` workload
/// (guard: the truth table is `2^n` entries in memory).
const VARIABLES_MAXIMALES_ALEA: u32 = 24;
/// Maximum number of philosophers accepted by the `philosophers` workload
/// (guard: the interleaved schema has `2n` keys, R_n grows fast — the bench
/// measures n ≤ 7).
const PHILOSOPHES_MAXIMAUX: u64 = 32;
/// Seconds → milliseconds conversion of the measured block.
const MILLIS_PAR_SECONDE: f64 = 1_000.0;

/// Tool measured by this runner (fixed: the binary only measures lisage;
/// external witnesses have their own binary with their own name).
pub const OUTIL: &str = "lisage";

/// Workload of the confrontation (closed set, ADR-0006).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Workload {
    /// Deterministic LCG truth table, BDD by Shannon expansion.
    RandomLcg,
    /// R_n of the golden thread, union of maps over the interleaved schema.
    Philosophers,
    /// Sliding window, absolute or relative encoding.
    Window,
}

impl Workload {
    /// Parses the workload name (binary edge); `None` if unknown.
    pub fn since_name(name: &str) -> Option<Workload> {
        match name {
            "random-lcg" => Some(Workload::RandomLcg),
            "philosophers" => Some(Workload::Philosophers),
            "window" => Some(Workload::Window),
            _ => None,
        }
    }

    /// JSON protocol label.
    pub fn label(&self) -> &'static str {
        match self {
            Workload::RandomLcg => "random-lcg",
            Workload::Philosophers => "philosophers",
            Workload::Window => "window",
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
    pub fn label(&self) -> &'static str {
        match self {
            Variante::Bdd => "bdd",
            Variante::Mdd => "mdd",
        }
    }
}

/// JSON label of a window encoding.
fn label_encoding(encoding: &Encoding) -> &'static str {
    match encoding {
        Encoding::Absolute => "absolute",
        Encoding::Relative => "relative",
    }
}

/// One confrontation measurement: the result of a workload run once.
#[derive(Clone, PartialEq, Debug)]
pub struct MeasureConfrontation {
    /// Measured diagram variant.
    pub variante: Variante,
    /// Workload executed.
    pub workload: Workload,
    /// Workload parameters, in a stable order (serialized as an object).
    pub parameters: Vec<(String, String)>,
    /// Encoding of the `window` workload only.
    pub encoding: Option<Encoding>,
    /// Time of the measured block, in milliseconds.
    pub time_ms: f64,
    /// Maximum RSS of the process, in kilobytes (`VmHWM`).
    pub rss_max_ko: u64,
    /// Nodes of the built diagram, terminals included (convention
    /// documented per tool in the protocol).
    pub nodes: u64,
    /// Cross-check value: identical across all tools for the same workload
    /// (counted cardinality, satisfactions). Any divergence between tools
    /// invalidates the measurement.
    pub verifie: u64,
}

/// Typed error of the confrontation harness.
#[derive(Clone, PartialEq, Debug)]
pub enum ErrorConfrontation {
    /// Invalid parameters for the workload (count, type, bounds).
    ParametersInvalid {
        /// The workload concerned.
        workload: &'static str,
        /// The detail of the invalidity.
        detail: String,
    },
    /// Unreadable input (state file, `/proc` missing).
    InputInvalid {
        /// The detail of the invalidity.
        detail: String,
    },
    /// The internal cross-check failed: the counted value does not match
    /// the expected value.
    VerifieEchoue {
        /// The expected value.
        expected: u64,
        /// The obtained value.
        obtenu: u64,
    },
    /// Core error (pool, fold, workshop), cause preserved.
    Internal {
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
            ErrorConfrontation::ParametersInvalid { workload, detail } => {
                write!(formatter, "invalid parameters for {workload}: {detail}")
            }
            ErrorConfrontation::InputInvalid { detail } => {
                write!(formatter, "invalid input: {detail}")
            }
            ErrorConfrontation::VerifieEchoue { expected, obtenu } => {
                write!(
                    formatter,
                    "cross-check failed: expected {expected}, got {obtenu}"
                )
            }
            ErrorConfrontation::Internal { source } => {
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
            ErrorConfrontation::Internal { source } => Some(source),
            ErrorConfrontation::Schema { source } => Some(source),
            _ => None,
        }
    }
}

impl From<CoreError> for ErrorConfrontation {
    fn from(source: CoreError) -> Self {
        ErrorConfrontation::Internal { source }
    }
}

impl From<PoolError> for ErrorConfrontation {
    fn from(source: PoolError) -> Self {
        ErrorConfrontation::Internal {
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
/// `lisage-reader::trace`: the measured values are a controlled
/// vocabulary, the contract stays correct).
fn escape_json(string: &str) -> String {
    let mut output = String::with_capacity(string.len() + 2);
    for character in string.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character_low if (character_low as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04x}", character_low as u32));
            }
            character_simple => output.push(character_simple),
        }
    }
    output
}

/// Serializes a measurement as one JSON line (without the trailing `\n`).
/// The key order is fixed and pinned by test: `tool`, `variant`, `workload`,
/// `parameters`, `encoding` (window only), `time_ms`, `rss_max_ko`,
/// `nodes`, `verified`.
pub fn serialize_line(measure: &MeasureConfrontation) -> String {
    let mut line = String::with_capacity(256);
    line.push('{');
    line.push_str(&format!(
        "\"tool\":\"{}\",\"variant\":\"{}\",\"workload\":\"{}\"",
        escape_json(OUTIL),
        measure.variante.label(),
        measure.workload.label()
    ));
    line.push_str(",\"parameters\":{");
    for (indice, (key, value)) in measure.parameters.iter().enumerate() {
        if indice > 0 {
            line.push(',');
        }
        line.push_str(&format!(
            "\"{}\":\"{}\"",
            escape_json(key),
            escape_json(value)
        ));
    }
    line.push('}');
    if let Some(encoding) = &measure.encoding {
        line.push_str(&format!(",\"encoding\":\"{}\"", label_encoding(encoding)));
    }
    line.push_str(&format!(
        ",\"time_ms\":{:.3},\"rss_max_ko\":{},\"nodes\":{},\"verified\":{}",
        measure.time_ms, measure.rss_max_ko, measure.nodes, measure.verifie
    ));
    line.push('}');
    line
}

/// Maximum RSS of the process in kilobytes (`VmHWM` from
/// `/proc/self/status`). Linux only, documented in the protocol: local runs
/// and witness containers are Linux.
fn rss_max_ko() -> Result<u64, ErrorConfrontation> {
    const MARKER: &str = "VmHWM:";
    let status = std::fs::read_to_string("/proc/self/status").map_err(|error| {
        ErrorConfrontation::InputInvalid {
            detail: format!("/proc/self/status unreadable: {error}"),
        }
    })?;
    for line in status.lines() {
        if let Some(values) = line.strip_prefix(MARKER) {
            return values
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or_else(|| ErrorConfrontation::InputInvalid {
                    detail: format!("unexpected {MARKER} line: {line}"),
                });
        }
    }
    Err(ErrorConfrontation::InputInvalid {
        detail: "VmHWM absent from /proc/self/status (not-Linux platform)".to_string(),
    })
}

/// Generates the deterministic LCG truth table over `n` variables (high bit
/// of the state, same LCG as the M1-8 baseline).
fn table_lcg(number_variables: u32) -> Vec<bool> {
    let mut state = GRAINE_LCG;
    (0..(1usize << number_variables))
        .map(|_| {
            state = state
                .wrapping_mul(MULTIPLICATEUR_LCG)
                .wrapping_add(INCREMENT_LCG);
            (state >> 63) == 1
        })
        .collect()
}

/// Workload `random-lcg`: classic BDD of a deterministic random truth
/// table, built by Shannon expansion. The internal C6 falsification is
/// run: the arena's node count must equal that of the independent mini-BDD
/// witness, and evaluation must follow the table.
fn workload_random_lcg(
    number_variables: u32,
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    if number_variables == 0 || number_variables > VARIABLES_MAXIMALES_ALEA {
        return Err(ErrorConfrontation::ParametersInvalid {
            workload: Workload::RandomLcg.label(),
            detail: format!("n must be in 1..={VARIABLES_MAXIMALES_ALEA}"),
        });
    }
    let keys = BddKeys::new(number_variables);
    let mut pool = Arena::new(BddController::new(number_variables), Box::new(NoOpObserver));
    let start = Instant::now();
    let table = table_lcg(number_variables);
    let satisfies_expected = table.iter().filter(|value| **value).count() as u64;
    let root = compile_bdd(&mut pool, ANCRE_MESURE, &keys, &table);
    let nodes = count_nodes_bdd(&pool, root);
    let duree = start.elapsed();
    // C6 falsification: the arena and the independent mini-BDD witness must
    // count the same number of nodes on the same table.
    let mut witness = crate::witness_bdd::MiniBdd::new();
    let root_witness = witness.compile_table(number_variables, &table);
    let nodes_witness = witness.count_nodes(root_witness);
    if nodes != nodes_witness {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected: nodes_witness,
            obtenu: nodes,
        });
    }
    if witness.evaluer(root_witness, 0) != table[0] {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected: table[0] as u64,
            obtenu: witness.evaluer(root_witness, 0) as u64,
        });
    }
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root,
            value: format!(
                "workload={} nodes={} verified={}",
                Workload::RandomLcg.label(),
                nodes,
                satisfies_expected
            ),
        });
    }
    Ok(MeasureConfrontation {
        variante: Variante::Bdd,
        workload: Workload::RandomLcg,
        parameters: vec![("n".to_string(), number_variables.to_string())],
        encoding: None,
        time_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        nodes,
        verifie: satisfies_expected,
    })
}

/// Extended state of a philosopher: `pc_i` (think/hungry/eat) and derived
/// `left_i` (free/held/preceding), interleaved `pc_0 left_0 pc_1 …`.
type StateEtendu = Vec<u64>;

/// Workload `philosophers`: R_n of the golden thread (docs/golden-thread-example-n-philosophers-end-to-end)
/// read from the extended-state file shared by all tools (first line
/// `n <n>`, then one extended state per line, values separated by spaces).
/// Construction: union of the maps compiled over the interleaved schema;
/// the fold `(ℕ,+,×)` cardinality must equal the number of states in the
/// file.
fn workload_philosophers(
    number_philosophers: u64,
    path: &str,
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    if number_philosophers == 0 || number_philosophers > PHILOSOPHES_MAXIMAUX {
        return Err(ErrorConfrontation::ParametersInvalid {
            workload: Workload::Philosophers.label(),
            detail: format!("n must be in 1..={PHILOSOPHES_MAXIMAUX}"),
        });
    }
    // Init outside the measured block (schema, keys, arenas) — like the
    // manager init at the witnesses. Measured block: input reading +
    // construction + counting.
    let schema = Schema::Map(
        (0..PAS_CLE_INTERCALAGE * number_philosophers)
            .map(Atom)
            .collect(),
        Domain::new(vec![Atom(PENSE), Atom(AFFAME), Atom(EATS)]),
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
    let start = Instant::now();
    let content =
        std::fs::read_to_string(path).map_err(|error| ErrorConfrontation::InputInvalid {
            detail: format!("state file {path} unreadable: {error}"),
        })?;
    let states = read_states(number_philosophers, &content)?;
    let expected = states.len() as u64;
    let mut root = pool.intern(ANCRE_MESURE, Form::Terminal(false));
    for state in &states {
        let map: BTreeMap<Value, Value> = state
            .iter()
            .enumerate()
            .map(|(indice, value)| (Value::Atom(Atom(indice as u64)), Value::Atom(Atom(*value))))
            .collect();
        let membre = compiled(&mut pool, ANCRE_MESURE, &schema, &Value::Map(map))?;
        root = union(&mut pool, &keys, ANCRE_MESURE, root, membre)?;
    }
    let verifie = count_fold(&pool, &mut output, ANCRE_MESURE, &schema, root)?;
    let nodes = count_nodes(&pool, root)?;
    let duree = start.elapsed();
    if verifie != expected {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected,
            obtenu: verifie,
        });
    }
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root,
            value: format!(
                "workload={} nodes={} verified={}",
                Workload::Philosophers.label(),
                nodes,
                verifie
            ),
        });
    }
    Ok(MeasureConfrontation {
        variante: Variante::Mdd,
        workload: Workload::Philosophers,
        parameters: vec![("n".to_string(), number_philosophers.to_string())],
        encoding: None,
        time_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        nodes,
        verifie,
    })
}

/// Reads and validates the extended-state file: first line `n <n>`, then
/// `2n` values per line in `{0, 1, 2}`.
fn read_states(
    number_philosophers: u64,
    content: &str,
) -> Result<Vec<StateEtendu>, ErrorConfrontation> {
    let number_values = PAS_CLE_INTERCALAGE as usize * number_philosophers as usize;
    let mut lines = content.lines();
    let first = lines
        .next()
        .ok_or_else(|| ErrorConfrontation::InputInvalid {
            detail: "empty state file".to_string(),
        })?;
    let fields_first: Vec<&str> = first.split_whitespace().collect();
    if fields_first.len() != 2 || fields_first[0] != "n" {
        return Err(ErrorConfrontation::InputInvalid {
            detail: format!("expected first line \"n <n>\", got: {first}"),
        });
    }
    let n_declared: u64 =
        fields_first[1]
            .parse()
            .map_err(|_| ErrorConfrontation::InputInvalid {
                detail: format!("unreadable n: {}", fields_first[1]),
            })?;
    if n_declared != number_philosophers {
        return Err(ErrorConfrontation::InputInvalid {
            detail: format!("declared n {n_declared} ≠ requested n {number_philosophers}"),
        });
    }
    let mut states = Vec::new();
    for (number, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let state: Result<Vec<u64>, _> = line
            .split_whitespace()
            .map(|value| value.parse::<u64>())
            .collect();
        let state = state.map_err(|_| ErrorConfrontation::InputInvalid {
            detail: format!("line {}: unreadable value", number + 2),
        })?;
        if state.len() != number_values {
            return Err(ErrorConfrontation::InputInvalid {
                detail: format!(
                    "line {}: {} values, {} expected",
                    number + 2,
                    state.len(),
                    number_values
                ),
            });
        }
        if state.iter().any(|value| *value > EATS) {
            return Err(ErrorConfrontation::InputInvalid {
                detail: format!("line {}: value outside the domain {{0,1,2}}", number + 2),
            });
        }
        states.push(state);
    }
    Ok(states)
}

/// Workload `window`: the sliding window of golden thread no. 2 in the
/// requested encoding. Check: root → terminal paths must be
/// `size × count_bases` (one window per base, `size` positions per window),
/// whatever the encoding — only the number of shared nodes differs
/// (absolute: one node per base; relative: a single form, O(1) in
/// `count_bases`).
fn workload_window(
    size: u64,
    count_bases: u64,
    encoding: Encoding,
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    if size == 0 || count_bases == 0 {
        return Err(ErrorConfrontation::ParametersInvalid {
            workload: Workload::Window.label(),
            detail: "size and count_bases must be ≥ 1".to_string(),
        });
    }
    let expected = size.saturating_mul(count_bases);
    let mut pool = Arena::new(
        WorkshopController::new(
            BddController::new(VARIABLES_BDD_FENETRE),
            Reductions::without_collapse(),
        ),
        Box::new(NoOpObserver),
    );
    let start = Instant::now();
    let root = build_window(&mut pool, ANCRE_MESURE, encoding, size, count_bases);
    let verifie = count_paths(&pool, root)?;
    let nodes = count_nodes(&pool, root)?;
    let duree = start.elapsed();
    if verifie != expected {
        return Err(ErrorConfrontation::VerifieEchoue {
            expected,
            obtenu: verifie,
        });
    }
    if let Some(observer) = observer {
        observer.on_event(&TraceEvent::Attribute {
            channel: ChannelAttribute::Synthesize,
            node: root,
            value: format!(
                "workload={} encoding={} nodes={} verified={}",
                Workload::Window.label(),
                label_encoding(&encoding),
                nodes,
                verifie
            ),
        });
    }
    Ok(MeasureConfrontation {
        variante: Variante::Mdd,
        workload: Workload::Window,
        parameters: vec![
            ("size".to_string(), size.to_string()),
            ("count_bases".to_string(), count_bases.to_string()),
        ],
        encoding: Some(encoding),
        time_ms: duree.as_secs_f64() * MILLIS_PAR_SECONDE,
        rss_max_ko: rss_max_ko()?,
        nodes,
        verifie,
    })
}

/// Runs a workload once and returns the measurement. Parameters:
/// `random-lcg <n>`, `philosophers <n> <path>`,
/// `window <size> <count_bases> [absolute|relative]` (absolute by default).
/// `observer`: narration channel (ADR-0100) — an `Attribute`/`Synthesize`
/// event carries the workload summary.
pub fn execute(
    workload: &Workload,
    parameters: &[String],
    observer: Option<&dyn Observer>,
) -> Result<MeasureConfrontation, ErrorConfrontation> {
    match workload {
        Workload::RandomLcg => {
            let only = parameters
                .first()
                .ok_or_else(|| ErrorConfrontation::ParametersInvalid {
                    workload: Workload::RandomLcg.label(),
                    detail: "expected: random-lcg <n>".to_string(),
                })?;
            let number_variables =
                only.parse::<u32>()
                    .map_err(|_| ErrorConfrontation::ParametersInvalid {
                        workload: Workload::RandomLcg.label(),
                        detail: format!("unreadable n: {only}"),
                    })?;
            workload_random_lcg(number_variables, observer)
        }
        Workload::Philosophers => {
            let (first, second) = parameters.first().zip(parameters.get(1)).ok_or_else(|| {
                ErrorConfrontation::ParametersInvalid {
                    workload: Workload::Philosophers.label(),
                    detail: "expected: philosophers <n> <path>".to_string(),
                }
            })?;
            let number_philosophers =
                first
                    .parse::<u64>()
                    .map_err(|_| ErrorConfrontation::ParametersInvalid {
                        workload: Workload::Philosophers.label(),
                        detail: format!("unreadable n: {first}"),
                    })?;
            workload_philosophers(number_philosophers, second, observer)
        }
        Workload::Window => {
            let (first, second) = parameters.first().zip(parameters.get(1)).ok_or_else(|| {
                ErrorConfrontation::ParametersInvalid {
                    workload: Workload::Window.label(),
                    detail: "expected: window <size> <count_bases> [absolute|relative]".to_string(),
                }
            })?;
            let size = first
                .parse::<u64>()
                .map_err(|_| ErrorConfrontation::ParametersInvalid {
                    workload: Workload::Window.label(),
                    detail: format!("unreadable size: {first}"),
                })?;
            let count_bases =
                second
                    .parse::<u64>()
                    .map_err(|_| ErrorConfrontation::ParametersInvalid {
                        workload: Workload::Window.label(),
                        detail: format!("unreadable count_bases: {second}"),
                    })?;
            let encoding = match parameters.get(2).map(String::as_str) {
                None | Some("absolute") => Encoding::Absolute,
                Some("relative") => Encoding::Relative,
                Some(other) => {
                    return Err(ErrorConfrontation::ParametersInvalid {
                        workload: Workload::Window.label(),
                        detail: format!("unknown encoding: {other}"),
                    });
                }
            };
            workload_window(size, count_bases, encoding, observer)
        }
    }
}
