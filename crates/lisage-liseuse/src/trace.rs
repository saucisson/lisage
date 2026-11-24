//! **Versioned JSONL** trace serializer (M1-9 Liseuse v0, M2-6 v1 step by
//! step): one JSON line per event, schema `{"v":3, ...}` (see
//! `docs/reader-visualizer-of-diagram-orders-and-operations.md` § 3).
//! Hand-written serialization: no external dependency (escalation list
//! AGENTS.md).

use std::io::Write;
use std::sync::{Arc, Mutex};

use lisage_core::NodeId;
use lisage_core::observer::{
    ChannelAttr, ClassHistogram, Observer, ReductionRule, SpanKind, StructureChangeKind, TraceEvent,
};

/// Trace schema version (`{"v":7, ...}`): v2 adds the
/// `Attr`/`MemoHit`/`MemoMiss`/`Round` events of the step-by-step view (M2-6);
/// v3 adds the `Histogram` event (MEM-17: degree histogram per class);
/// v4 adds the spans of D1-1 — `SpanOpen`/`SpanClose` events and a `"span"`
/// field on every other event; v5 adds the register events of D1-2 —
/// `OpStart`/`OpEnd`/`VisitEntry`/`VisitExit`/`MemoConsult`/`MemoInsert`/
/// `MemoryWrite`/`StructureChange`, plus the extended `NodeCreated`
/// (`form`), `Gc` (`collected_list`) and `Round` (levels, handles, growth);
/// v6 adds the events of the in-place kernel (ADR-0109): the `Fire`
/// span kind (`"fire"`) and the `LocalFire` event, one per local fire
/// in firing order; v7 adds the memory and S-channel vocabulary of the
/// deferred ADR-0109 tracks: `MemoryRead` (the observed reads of `φ_out`
/// through the probe) and `MemoryState` (one per op, the `(0, 0)` witness
/// of a nil memory `M = ()`). New-in-v6/v7 fields follow the English-only
/// charter (M6): the legacy French field names of v1-v5 stay unchanged,
/// the `LocalFire` fields are `fire`/`level`/`before`/`image`/`merged`,
/// the `MemoryRead` fields are `node`/`level`/`hit`/`value` and the
/// `MemoryState` fields are `version`/`entries`.
pub const TRACE_VERSION: u32 = 7;

/// Minimal JSON escaping of a string (events carry no dynamic
/// ones, but the contract stays correct).
fn escape_json(string: &str) -> String {
    let mut output = String::with_capacity(string.len() + 2);
    for character in string.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if (c as u32) < 0x20 => output.push_str(&format!("\\u{:04x}", c as u32)),
            c => output.push(c),
        }
    }
    output
}

/// JSON name of a reduction rule.
fn name_rule(rule: ReductionRule) -> &'static str {
    match rule {
        ReductionRule::Share => "R-share",
        ReductionRule::Collapse => "R-collapse",
        ReductionRule::Default => "R-default",
    }
}

/// JSON name of a span kind (D1-1).
fn name_span_kind(kind: SpanKind) -> &'static str {
    match kind {
        SpanKind::Op => "op",
        SpanKind::Round => "round",
        SpanKind::Fire => "fire",
        SpanKind::Visit => "visite",
    }
}

/// JSON name of a structure change classification (D1-2).
fn name_structure_change(kind: StructureChangeKind) -> &'static str {
    match kind {
        StructureChangeKind::Identity => "identite",
        StructureChangeKind::Relabeling => "reetiquetage",
        StructureChangeKind::Collapse => "collapse",
        StructureChangeKind::Expansion => "expansion",
        StructureChangeKind::Restructuration => "restructuration",
    }
}

/// JSON rendering of an arc list `(key, target)` (D1-2):
/// `"arcs":[[0,12],[1,7]]`.
fn serialize_arcs(arcs: &[(u64, NodeId)]) -> String {
    let entries: Vec<String> = arcs
        .iter()
        .map(|(key, target)| format!("[{},{}]", key, target.0))
        .collect();
    format!("[{}]", entries.join(","))
}

/// JSON rendering of a node id list (D1-2): `"liste":[3,7,12]`.
fn serialize_list_nodes(list: &[NodeId]) -> String {
    let entries: Vec<String> = list.iter().map(|node| format!("{}", node.0)).collect();
    format!("[{}]", entries.join(","))
}

/// JSON rendering of a u32 list (D1-2): `"niveaux":[0,2]`.
fn serialize_list_levels(list: &[u32]) -> String {
    let entries: Vec<String> = list.iter().map(|level| format!("{level}")).collect();
    format!("[{}]", entries.join(","))
}

/// JSON rendering of an optional span id (D1-1): `"span":3` or `"span":null`.
fn serialize_span(span: Option<lisage_core::observer::SpanId>) -> String {
    match span {
        Some(span) => format!("\"span\":{}", span.0),
        None => "\"span\":null".to_string(),
    }
}

/// JSON name of an attribute channel (M2-6).
fn name_channel(channel: ChannelAttr) -> &'static str {
    match channel {
        ChannelAttr::Inherited => "herite",
        ChannelAttr::Synthesized => "synthetise",
        ChannelAttr::Memory => "memoire",
    }
}

/// Serializes the per-class degree histogram (MEM-17) as a JSON array.
fn serialize_histogramme(classes: &[ClassHistogram]) -> String {
    let entries: Vec<String> = classes
        .iter()
        .map(|c| {
            format!(
                "{{\"class\":{},\"live\":{},\"peak\":{},\"slack\":{}}}",
                c.class, c.live, c.peak, c.slack
            )
        })
        .collect();
    format!("\"classes\":[{}]", entries.join(","))
}

/// Serializes a trace event as one JSONL line (without the trailing `\n`).
/// D1-1 (v4): every event line carries its `"span"` field; `SpanOpen` and
/// `SpanClose` carry the span they define instead.
pub fn serialize_event(event: &TraceEvent) -> String {
    let (type_event, fields) = match event {
        TraceEvent::SpanOpen { span, kind, lane } => {
            return format!(
                "{{\"v\":{TRACE_VERSION},\"type\":\"SpanOpen\",\"span\":{},\"kind\":\"{}\",\"lane\":{}}}",
                span.0,
                name_span_kind(*kind),
                lane
            );
        }
        TraceEvent::SpanClose { span } => {
            return format!(
                "{{\"v\":{TRACE_VERSION},\"type\":\"SpanClose\",\"span\":{}}}",
                span.0
            );
        }
        TraceEvent::NodeCreated {
            node, level, form, ..
        } => (
            "NodeCreated",
            match level {
                Some(level) => format!(
                    "\"noeud\":{},\"level\":{},\"forme\":\"{}\"",
                    node.0,
                    level.0,
                    escape_json(&form.render())
                ),
                None => format!(
                    "\"noeud\":{},\"level\":null,\"forme\":\"{}\"",
                    node.0,
                    escape_json(&form.render())
                ),
            },
        ),
        TraceEvent::NodeShared { node, .. } => ("NodeShared", format!("\"noeud\":{}", node.0)),
        TraceEvent::ReductionApplied { rule, level, .. } => (
            "Reduction",
            format!(
                "\"rule\":\"{}\",\"level\":{}",
                escape_json(name_rule(*rule)),
                level.0
            ),
        ),
        TraceEvent::DiagramBuilt { handle, nodes, .. } => (
            "DiagramBuilt",
            format!(
                "\"controller\":{},\"anchor\":{},\"noeud\":{},\"noeuds\":{}",
                handle.controller.0, handle.anchor.0, handle.node.0, nodes
            ),
        ),
        TraceEvent::Attr {
            channel,
            node,
            value,
            ..
        } => (
            "Attr",
            format!(
                "\"canal\":\"{}\",\"noeud\":{},\"valeur\":\"{}\"",
                name_channel(*channel),
                node.0,
                escape_json(&value.render())
            ),
        ),
        TraceEvent::MemoHit { node, .. } => ("MemoHit", format!("\"noeud\":{}", node.0)),
        TraceEvent::MemoMiss { node, .. } => ("MemoMiss", format!("\"noeud\":{}", node.0)),
        TraceEvent::Gc {
            live,
            collected,
            compaction,
            collected_list,
            ..
        } => (
            "Gc",
            format!(
                "\"vivants\":{live},\"collectes\":{collected},\"compaction\":{compaction},\"liste_collectes\":{}",
                serialize_list_nodes(collected_list)
            ),
        ),
        TraceEvent::Round {
            number,
            active_levels,
            modified_levels,
            before,
            image,
            after,
            growth,
            ..
        } => (
            "Round",
            format!(
                "\"numero\":{number},\"niveaux_actifs\":{},\"niveaux_modifies\":{},\"avant\":{},\"image\":{},\"apres\":{},\"croissance\":{growth}",
                serialize_list_levels(active_levels),
                serialize_list_levels(modified_levels),
                before.0,
                image.0,
                after.0
            ),
        ),
        TraceEvent::Histogram { classes, .. } => ("Histogram", serialize_histogramme(classes)),
        TraceEvent::LocalFire {
            number,
            level,
            before,
            image,
            merged,
            ..
        } => (
            "LocalFire",
            format!(
                "\"fire\":{number},\"level\":{level},\"before\":{},\"image\":{},\"merged\":{}",
                before.0, image.0, merged.0
            ),
        ),
        TraceEvent::OpStart { fingerprint, .. } => {
            ("OpStart", format!("\"empreinte\":{fingerprint}"))
        }
        TraceEvent::OpEnd { .. } => ("OpEnd", String::new()),
        TraceEvent::VisitEntry { node, .. } => ("VisiteEntree", format!("\"noeud\":{}", node.0)),
        TraceEvent::VisitExit { node, result, .. } => (
            "VisiteSortie",
            format!("\"noeud\":{},\"resultat\":{}", node.0, result.0),
        ),
        TraceEvent::MemoConsult {
            fingerprint,
            node,
            h,
            version,
            hit,
            ..
        } => (
            "MemoConsult",
            format!(
                "\"empreinte\":{fingerprint},\"noeud\":{},\"h\":\"{}\",\"version\":{version},\"touche\":{hit}",
                node.0,
                escape_json(&h.render())
            ),
        ),
        TraceEvent::MemoInsert {
            fingerprint,
            node,
            h,
            version,
            result,
            ..
        } => (
            "MemoInsert",
            format!(
                "\"empreinte\":{fingerprint},\"noeud\":{},\"h\":\"{}\",\"version\":{version},\"resultat\":{}",
                node.0,
                escape_json(&h.render()),
                result.0
            ),
        ),
        TraceEvent::MemoryWrite {
            version_before,
            version_after,
            ..
        } => (
            "EcritureMemoire",
            format!("\"version_avant\":{version_before},\"version_apres\":{version_after}"),
        ),
        TraceEvent::MemoryRead {
            node,
            level,
            hit,
            value,
            ..
        } => (
            "MemoryRead",
            format!(
                "\"node\":{},\"level\":{level},\"hit\":{hit},\"value\":\"{}\"",
                node.0,
                escape_json(&value.render())
            ),
        ),
        TraceEvent::MemoryState {
            version,
            entries,
            ..
        } => (
            "MemoryState",
            format!("\"version\":{version},\"entries\":{entries}"),
        ),
        TraceEvent::StructureChange {
            node,
            classification,
            arcs_before,
            arcs_after,
            ..
        } => (
            "StructureChange",
            format!(
                "\"noeud\":{},\"classification\":\"{}\",\"arcs_avant\":{},\"arcs_apres\":{}",
                node.0,
                name_structure_change(*classification),
                serialize_arcs(arcs_before),
                serialize_arcs(arcs_after)
            ),
        ),
    };
    let span = serialize_span(event.span());
    if fields.is_empty() {
        format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_event}\",{span}}}")
    } else {
        format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_event}\",{span},{fields}}}")
    }
}

/// Observer that collects the trace in memory, serialized JSONL.
/// Cloneable: a clone placed in the pool shares the same trace.
/// Event filter of a `JsonlObserver` (D1-3).
pub type FilterTrace = Arc<dyn Fn(&TraceEvent) -> bool + Send + Sync>;

#[derive(Clone)]
pub struct JsonlObserver {
    lines: Arc<Mutex<Vec<String>>>,
    /// Optional event filter (D1-3): only events passing the filter are
    /// serialized — and only then are their lazy `TraceVal` values forced.
    filter: Option<FilterTrace>,
}

impl JsonlObserver {
    /// New observer, empty trace.
    pub fn new() -> Self {
        JsonlObserver {
            lines: Arc::new(Mutex::new(Vec::new())),
            filter: None,
        }
    }

    /// New observer keeping only the events that pass `filter` (D1-3):
    /// the lazy values of the rejected events are never forced.
    pub fn with_filter(filter: FilterTrace) -> Self {
        JsonlObserver {
            lines: Arc::new(Mutex::new(Vec::new())),
            filter: Some(filter),
        }
    }

    /// Collected JSONL lines (in emission order).
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("mutex poisoned").clone()
    }

    /// Writes the full trace (one line per event) to `output`.
    pub fn write<W: Write>(&self, output: &mut W) -> std::io::Result<()> {
        for line in self.lines() {
            writeln!(output, "{line}")?;
        }
        Ok(())
    }
}

impl Default for JsonlObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for JsonlObserver {
    fn on_event(&self, event: &TraceEvent) {
        // D1-3: the filter runs on the cheap fields first; the lazy
        // values are forced (serialized) only if the filter passes.
        if let Some(filter) = &self.filter
            && !filter(event)
        {
            return;
        }
        self.lines
            .lock()
            .expect("mutex poisoned")
            .push(serialize_event(event));
    }
}

// ---------------------------------------------------------------------
// D1-4 (issue #158): deserializer of the JSONL trace.
//
// Version policy: the `"v"` field must be present and equal to
// [`TRACE_VERSION`]; any other version is **rejected explicitly** with a
// typed error naming the version. Decision on v2 traces: they are
// rejected, not silently upgraded — v3/v4/v5/v6/v7 added fields
// (histogram, spans, register events, fires, memory vocabulary) that a
// v2 line does not carry, so an upgrade would fabricate `null` spans and
// empty registers; the correct path is to re-run the scenario and produce
// a current (`TRACE_VERSION`) trace.
// ---------------------------------------------------------------------

use crate::json::{self, Json};
use lisage_core::observer::{SpanId, TraceVal};
use lisage_core::{Anchor, ControllerId, Handle};

/// Deserialization error of a trace: 1-based line number (0 = the whole
/// document) and message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorDeserialisation {
    /// 1-based line number, 0 for whole-document errors.
    pub line: usize,
    /// Error message.
    pub message: String,
}

impl std::fmt::Display for ErrorDeserialisation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for ErrorDeserialisation {}

/// Required integer field.
fn field_num(obj: &Json, key: &str) -> Result<i64, String> {
    obj.get(key)
        .and_then(Json::as_num)
        .ok_or_else(|| format!("missing or invalid integer field '{key}'"))
}

/// Required u64 field.
fn field_u64(obj: &Json, key: &str) -> Result<u64, String> {
    u64::try_from(field_num(obj, key)?).map_err(|_| format!("field '{key}' out of u64 range"))
}

/// Required u32 field.
fn field_u32(obj: &Json, key: &str) -> Result<u32, String> {
    u32::try_from(field_num(obj, key)?).map_err(|_| format!("field '{key}' out of u32 range"))
}

/// Required string field.
fn field_str(obj: &Json, key: &str) -> Result<String, String> {
    obj.get(key)
        .and_then(Json::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing or invalid string field '{key}'"))
}

/// Required boolean field.
fn field_bool(obj: &Json, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("missing or invalid boolean field '{key}'"))
}

/// Required array field.
fn field_arr<'a>(obj: &'a Json, key: &str) -> Result<&'a [Json], String> {
    obj.get(key)
        .and_then(Json::as_arr)
        .ok_or_else(|| format!("missing or invalid array field '{key}'"))
}

/// Optional span field (`"span": N` or `"span": null`).
fn field_span(obj: &Json) -> Result<Option<SpanId>, String> {
    match obj.get("span") {
        None | Some(Json::Null) => Ok(None),
        Some(value) => value
            .as_num()
            .and_then(|n| u64::try_from(n).ok())
            .map(|n| Some(SpanId(n)))
            .ok_or_else(|| "invalid 'span' field".to_string()),
    }
}

/// u32 list field (`"niveaux":[0,2]`).
fn field_list_u32(obj: &Json, key: &str) -> Result<Vec<u32>, String> {
    field_arr(obj, key)?
        .iter()
        .map(|item| {
            item.as_num()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("invalid u32 in '{key}'"))
        })
        .collect()
}

/// Node id list field (`"liste":[3,7,12]`).
fn field_list_nodes(obj: &Json, key: &str) -> Result<Vec<lisage_core::NodeId>, String> {
    field_arr(obj, key)?
        .iter()
        .map(|item| {
            item.as_num()
                .and_then(|n| u32::try_from(n).ok())
                .map(lisage_core::NodeId)
                .ok_or_else(|| format!("invalid node id in '{key}'"))
        })
        .collect()
}

/// Arc list field (`"arcs":[[0,12],[1,7]]`).
fn field_arcs(obj: &Json, key: &str) -> Result<Vec<(u64, lisage_core::NodeId)>, String> {
    field_arr(obj, key)?
        .iter()
        .map(|entry| {
            let pair = entry
                .as_arr()
                .filter(|p| p.len() == 2)
                .ok_or_else(|| format!("invalid arc pair in '{key}'"))?;
            let key = pair[0]
                .as_num()
                .and_then(|n| u64::try_from(n).ok())
                .ok_or_else(|| format!("invalid arc key in '{key}'"))?;
            let target = pair[1]
                .as_num()
                .and_then(|n| u32::try_from(n).ok())
                .map(lisage_core::NodeId)
                .ok_or_else(|| format!("invalid arc target in '{key}'"))?;
            Ok((key, target))
        })
        .collect()
}

/// Parses one JSONL line into a trace event. The `"v"` field must equal
/// [`TRACE_VERSION`]; unknown versions are rejected explicitly.
pub fn deserialize_event(line: &str) -> Result<TraceEvent, String> {
    let obj = json::parse(line).map_err(|e| format!("invalid JSON: {e}"))?;
    let version = field_num(&obj, "v")?;
    if version != TRACE_VERSION as i64 {
        return Err(format!(
            "unsupported trace version {version} (expected {TRACE_VERSION})"
        ));
    }
    let type_event = field_str(&obj, "type")?;
    let span = field_span(&obj)?;
    let node = |key: &str| -> Result<lisage_core::NodeId, String> {
        field_u32(&obj, key).map(lisage_core::NodeId)
    };
    let event = match type_event.as_str() {
        "SpanOpen" => {
            let kind = match field_str(&obj, "kind")?.as_str() {
                "op" => SpanKind::Op,
                "round" => SpanKind::Round,
                "fire" => SpanKind::Fire,
                "visite" => SpanKind::Visit,
                other => return Err(format!("unknown span kind '{other}'")),
            };
            TraceEvent::SpanOpen {
                span: SpanId(field_u64(&obj, "span")?),
                kind,
                lane: field_u32(&obj, "lane")?,
            }
        }
        "SpanClose" => TraceEvent::SpanClose {
            span: SpanId(field_u64(&obj, "span")?),
        },
        "NodeCreated" => {
            let level = match obj.get("level") {
                None | Some(Json::Null) => None,
                Some(value) => Some(lisage_core::Level(
                    value
                        .as_num()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or("invalid 'level' field")?,
                )),
            };
            TraceEvent::NodeCreated {
                node: node("noeud")?,
                level,
                form: TraceVal::Text(field_str(&obj, "forme")?),
                span,
            }
        }
        "NodeShared" => TraceEvent::NodeShared {
            node: node("noeud")?,
            span,
        },
        "Reduction" => {
            let rule = match field_str(&obj, "rule")?.as_str() {
                "R-share" => ReductionRule::Share,
                "R-collapse" => ReductionRule::Collapse,
                "R-default" => ReductionRule::Default,
                other => return Err(format!("unknown reduction rule '{other}'")),
            };
            TraceEvent::ReductionApplied {
                rule,
                level: lisage_core::Level(field_u32(&obj, "level")?),
                span,
            }
        }
        "DiagramBuilt" => TraceEvent::DiagramBuilt {
            handle: Handle {
                controller: ControllerId(field_u64(&obj, "controller")?),
                anchor: Anchor(field_u32(&obj, "anchor")?),
                node: node("noeud")?,
            },
            nodes: usize::try_from(field_num(&obj, "noeuds")?)
                .map_err(|_| "field 'noeuds' out of range".to_string())?,
            span,
        },
        "Attr" => {
            let channel = match field_str(&obj, "canal")?.as_str() {
                "herite" => ChannelAttr::Inherited,
                "synthetise" => ChannelAttr::Synthesized,
                "memoire" => ChannelAttr::Memory,
                other => return Err(format!("unknown attribute channel '{other}'")),
            };
            TraceEvent::Attr {
                channel,
                node: node("noeud")?,
                value: TraceVal::Text(field_str(&obj, "valeur")?),
                span,
            }
        }
        "MemoHit" => TraceEvent::MemoHit {
            node: node("noeud")?,
            span,
        },
        "MemoMiss" => TraceEvent::MemoMiss {
            node: node("noeud")?,
            span,
        },
        "Gc" => TraceEvent::Gc {
            live: field_u64(&obj, "vivants")?,
            collected: field_u64(&obj, "collectes")?,
            compaction: field_bool(&obj, "compaction")?,
            collected_list: field_list_nodes(&obj, "liste_collectes")?,
            span,
        },
        "Round" => TraceEvent::Round {
            number: field_u32(&obj, "numero")?,
            active_levels: field_list_u32(&obj, "niveaux_actifs")?,
            modified_levels: field_list_u32(&obj, "niveaux_modifies")?,
            before: node("avant")?,
            image: node("image")?,
            after: node("apres")?,
            growth: field_num(&obj, "croissance")?,
            span,
        },
        "LocalFire" => {
            let (number, level) = (field_u32(&obj, "fire")?, field_u32(&obj, "level")?);
            TraceEvent::LocalFire {
                number,
                level,
                before: node("before")?,
                image: node("image")?,
                merged: node("merged")?,
                span,
            }
        }
        "Histogram" => TraceEvent::Histogram {
            classes: field_arr(&obj, "classes")?
                .iter()
                .map(|entry| {
                    Ok(ClassHistogram {
                        class: field_u32(entry, "class")?,
                        live: field_u64(entry, "live")?,
                        peak: field_u64(entry, "peak")?,
                        slack: field_u64(entry, "slack")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            span,
        },
        "OpStart" => TraceEvent::OpStart {
            fingerprint: field_u64(&obj, "empreinte")?,
            span,
        },
        "OpEnd" => TraceEvent::OpEnd { span },
        "VisiteEntree" => TraceEvent::VisitEntry {
            node: node("noeud")?,
            span,
        },
        "VisiteSortie" => TraceEvent::VisitExit {
            node: node("noeud")?,
            result: node("resultat")?,
            span,
        },
        "MemoConsult" => TraceEvent::MemoConsult {
            fingerprint: field_u64(&obj, "empreinte")?,
            node: node("noeud")?,
            h: TraceVal::Text(field_str(&obj, "h")?),
            version: field_u64(&obj, "version")?,
            hit: field_bool(&obj, "touche")?,
            span,
        },
        "MemoInsert" => TraceEvent::MemoInsert {
            fingerprint: field_u64(&obj, "empreinte")?,
            node: node("noeud")?,
            h: TraceVal::Text(field_str(&obj, "h")?),
            version: field_u64(&obj, "version")?,
            result: node("resultat")?,
            span,
        },
        "EcritureMemoire" => TraceEvent::MemoryWrite {
            version_before: field_u64(&obj, "version_avant")?,
            version_after: field_u64(&obj, "version_apres")?,
            span,
        },
        "MemoryRead" => TraceEvent::MemoryRead {
            node: node("node")?,
            level: field_u32(&obj, "level")?,
            hit: field_bool(&obj, "hit")?,
            value: TraceVal::Text(field_str(&obj, "value")?),
            span,
        },
        "MemoryState" => TraceEvent::MemoryState {
            version: field_u64(&obj, "version")?,
            entries: usize::try_from(field_num(&obj, "entries")?)
                .map_err(|_| "field 'entries' out of range".to_string())?,
            span,
        },
        "StructureChange" => TraceEvent::StructureChange {
            node: node("noeud")?,
            classification: match field_str(&obj, "classification")?.as_str() {
                "identite" => StructureChangeKind::Identity,
                "reetiquetage" => StructureChangeKind::Relabeling,
                "collapse" => StructureChangeKind::Collapse,
                "expansion" => StructureChangeKind::Expansion,
                "restructuration" => StructureChangeKind::Restructuration,
                other => return Err(format!("unknown structure change '{other}'")),
            },
            arcs_before: field_arcs(&obj, "arcs_avant")?,
            arcs_after: field_arcs(&obj, "arcs_apres")?,
            span,
        },
        other => return Err(format!("unknown event type '{other}'")),
    };
    Ok(event)
}

/// Deserializes a whole JSONL trace (one event per line, empty lines
/// skipped). The first failing line stops the parse with its 1-based
/// line number.
pub fn deserialize_trace(text: &str) -> Result<Vec<TraceEvent>, ErrorDeserialisation> {
    let mut events = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match deserialize_event(line) {
            Ok(event) => events.push(event),
            Err(message) => {
                return Err(ErrorDeserialisation {
                    line: index + 1,
                    message,
                });
            }
        }
    }
    Ok(events)
}
