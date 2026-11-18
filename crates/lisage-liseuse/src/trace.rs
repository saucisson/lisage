//! **Versioned JSONL** trace serializer (M1-9 Liseuse v0, M2-6 v1 step by
//! step): one JSON line per event, schema `{"v":3, ...}` (see
//! `docs/reader-visualizer-of-diagram-orders-and-operations.md` § 3).
//! Hand-written serialization: no external dependency (escalation list
//! AGENTS.md).

use std::io::Write;
use std::sync::{Arc, Mutex};

use lisage_core::observer::{ChannelAttr, ClassHistogram, Observer, ReductionRule, TraceEvent};

/// Trace schema version (`{"v":3, ...}`): v2 adds the
/// `Attr`/`MemoHit`/`MemoMiss`/`Round` events of the step-by-step view (M2-6);
/// v3 adds the `Histogram` event (MEM-17: degree histogram per class).
pub const TRACE_VERSION: u32 = 3;

/// Minimal JSON escaping of a string (events carry no dynamic
/// ones, but the contract stays correct).
fn echapper_json(chaine: &str) -> String {
    let mut output = String::with_capacity(chaine.len() + 2);
    for caractere in chaine.chars() {
        match caractere {
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
fn name_regle(regle: ReductionRule) -> &'static str {
    match regle {
        ReductionRule::Share => "R-share",
        ReductionRule::Collapse => "R-collapse",
        ReductionRule::Default => "R-default",
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
fn serialiser_histogramme(classes: &[ClassHistogram]) -> String {
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
pub fn serialiser_evenement(evenement: &TraceEvent) -> String {
    let (type_evenement, champs) = match evenement {
        TraceEvent::NodeCreated { node, level } => (
            "NodeCreated",
            match level {
                Some(level) => format!("\"noeud\":{},\"level\":{}", node.0, level.0),
                None => format!("\"noeud\":{},\"level\":null", node.0),
            },
        ),
        TraceEvent::NodeShared { node } => ("NodeShared", format!("\"noeud\":{}", node.0)),
        TraceEvent::ReductionApplied { rule, level } => (
            "Reduction",
            format!(
                "\"rule\":\"{}\",\"level\":{}",
                echapper_json(name_regle(*rule)),
                level.0
            ),
        ),
        TraceEvent::DiagramBuilt { handle, nodes } => (
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
        } => (
            "Attr",
            format!(
                "\"canal\":\"{}\",\"noeud\":{},\"valeur\":\"{}\"",
                name_channel(*channel),
                node.0,
                echapper_json(value)
            ),
        ),
        TraceEvent::MemoHit { node } => ("MemoHit", format!("\"noeud\":{}", node.0)),
        TraceEvent::MemoMiss { node } => ("MemoMiss", format!("\"noeud\":{}", node.0)),
        TraceEvent::Gc {
            live,
            collected,
            compaction,
        } => (
            "Gc",
            format!("\"vivants\":{live},\"collectes\":{collected},\"compaction\":{compaction}"),
        ),
        TraceEvent::Round { numero } => ("Round", format!("\"numero\":{numero}")),
        TraceEvent::Histogram { classes } => ("Histogram", serialiser_histogramme(classes)),
    };
    format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_evenement}\",{champs}}}")
}

/// Observer that collects the trace in memory, serialized JSONL.
/// Cloneable: a clone placed in the pool shares the same trace.
#[derive(Clone)]
pub struct JsonlObserver {
    lignes: Arc<Mutex<Vec<String>>>,
}

impl JsonlObserver {
    /// New observer, empty trace.
    pub fn new() -> Self {
        JsonlObserver {
            lignes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Collected JSONL lines (in emission order).
    pub fn lignes(&self) -> Vec<String> {
        self.lignes.lock().expect("mutex poisoned").clone()
    }

    /// Writes the full trace (one line per event) to `output`.
    pub fn write<W: Write>(&self, output: &mut W) -> std::io::Result<()> {
        for ligne in self.lignes() {
            writeln!(output, "{ligne}")?;
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
        self.lignes
            .lock()
            .expect("mutex poisoned")
            .push(serialiser_evenement(event));
    }
}
