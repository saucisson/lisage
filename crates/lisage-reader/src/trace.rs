//! **Versioned JSONL** trace serializer (M1-9 Reader v0, M2-6 v1 step by
//! step) : one JSON line per event, schema `{"v":2, ...}` (see
//! `docs/reader-visualizer-of-diagram-orders-and-operations.md` § 3).
//! Hand-written serialization: no external dependency (AGENTS.md
//! escalation list).

use std::io::Write;
use std::sync::{Arc, Mutex};

use lisage_core::observer::{ChannelAttribute, Observer, ReductionRule, TraceEvent};

/// Trace schema version (`{"v":2, ...}`) : v2 adds the
/// `Attribute`/`MemoHit`/`MemoMiss`/`Round` events of the step-by-step view
/// (M2-6).
pub const TRACE_VERSION: u32 = 2;

/// Minimal JSON escaping of a string (events carry no dynamic strings, but
/// the contract stays correct).
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

/// JSON name of an attribute channel (M2-6).
fn name_channel(channel: ChannelAttribute) -> &'static str {
    match channel {
        ChannelAttribute::Inherited => "inherited",
        ChannelAttribute::Synthesize => "synthesized",
        ChannelAttribute::Memory => "memory",
    }
}

/// Serializes a trace event as one JSONL line (without the trailing `\n`).
pub fn serialize_event(event: &TraceEvent) -> String {
    let (type_event, fields) = match event {
        TraceEvent::NodeCreated { node, level } => (
            "NodeCreated",
            match level {
                Some(level) => format!("\"node\":{},\"level\":{}", node.0, level.0),
                None => format!("\"node\":{},\"level\":null", node.0),
            },
        ),
        TraceEvent::NodeShared { node } => ("NodeShared", format!("\"node\":{}", node.0)),
        TraceEvent::ReductionApplied { rule, level } => (
            "Reduction",
            format!(
                "\"rule\":\"{}\",\"level\":{}",
                escape_json(name_rule(*rule)),
                level.0
            ),
        ),
        TraceEvent::DiagramBuilt { handle, nodes } => (
            "DiagramBuilt",
            format!(
                "\"controller\":{},\"anchor\":{},\"node\":{},\"nodes\":{}",
                handle.controller.0, handle.anchor.0, handle.node.0, nodes
            ),
        ),
        TraceEvent::Attribute {
            channel,
            node,
            value,
        } => (
            "Attribute",
            format!(
                "\"channel\":\"{}\",\"node\":{},\"value\":\"{}\"",
                name_channel(*channel),
                node.0,
                escape_json(value)
            ),
        ),
        TraceEvent::MemoHit { node } => ("MemoHit", format!("\"node\":{}", node.0)),
        TraceEvent::MemoMiss { node } => ("MemoMiss", format!("\"node\":{}", node.0)),
        TraceEvent::Gc {
            live,
            collected,
            compaction,
        } => (
            "Gc",
            format!("\"live\":{live},\"collected\":{collected},\"compaction\":{compaction}"),
        ),
        TraceEvent::Round { number } => ("Round", format!("\"number\":{number}")),
    };
    format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_event}\",{fields}}}")
}

/// Observer that collects the trace in memory, serialized as JSONL.
/// Cloneable: a clone installed in the pool shares the same trace.
#[derive(Clone)]
pub struct JsonlObserver {
    lines: Arc<Mutex<Vec<String>>>,
}

impl JsonlObserver {
    /// New observer, empty trace.
    pub fn new() -> Self {
        JsonlObserver {
            lines: Arc::new(Mutex::new(Vec::new())),
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
        self.lines
            .lock()
            .expect("mutex poisoned")
            .push(serialize_event(event));
    }
}
