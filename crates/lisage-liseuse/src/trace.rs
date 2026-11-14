//! **Versioned JSONL** trace serializer (M1-9 Liseuse v0, M2-6 v1 step by
//! step): one JSON line per event, schema `{"v":2, ...}` (see
//! `docs/reader-visualizer-of-diagram-orders-and-operations.md` § 3).
//! Hand-written serialization: no external dependency (escalation list
//! AGENTS.md).

use std::io::Write;
use std::sync::{Arc, Mutex};

use lisage_core::observer::{CanalAttr, Observer, ReductionRule, TraceEvent};

/// Trace schema version (`{"v":2, ...}`): v2 adds the
/// `Attr`/`MemoHit`/`MemoMiss`/`Round` events of the step-by-step view (M2-6).
pub const TRACE_VERSION: u32 = 2;

/// Minimal JSON escaping of a string (events carry no dynamic
/// ones, but the contract stays correct).
fn echapper_json(chaine: &str) -> String {
    let mut sortie = String::with_capacity(chaine.len() + 2);
    for caractere in chaine.chars() {
        match caractere {
            '"' => sortie.push_str("\\\""),
            '\\' => sortie.push_str("\\\\"),
            '\n' => sortie.push_str("\\n"),
            '\r' => sortie.push_str("\\r"),
            '\t' => sortie.push_str("\\t"),
            c if (c as u32) < 0x20 => sortie.push_str(&format!("\\u{:04x}", c as u32)),
            c => sortie.push(c),
        }
    }
    sortie
}

/// JSON name of a reduction rule.
fn nom_regle(regle: ReductionRule) -> &'static str {
    match regle {
        ReductionRule::Share => "R-share",
        ReductionRule::Collapse => "R-collapse",
        ReductionRule::Default => "R-default",
    }
}

/// JSON name of an attribute channel (M2-6).
fn nom_canal(canal: CanalAttr) -> &'static str {
    match canal {
        CanalAttr::Herite => "herite",
        CanalAttr::Synthétise => "synthetise",
        CanalAttr::Memoire => "memoire",
    }
}

/// Serializes a trace event as one JSONL line (sans le `\n` final).
pub fn serialiser_evenement(evenement: &TraceEvent) -> String {
    let (type_evenement, champs) = match evenement {
        TraceEvent::NodeCreated { node, level } => (
            "NodeCreated",
            match level {
                Some(niveau) => format!("\"node\":{},\"level\":{}", node.0, niveau.0),
                None => format!("\"node\":{},\"level\":null", node.0),
            },
        ),
        TraceEvent::NodeShared { node } => ("NodeShared", format!("\"node\":{}", node.0)),
        TraceEvent::ReductionApplied { rule, level } => (
            "Reduction",
            format!(
                "\"rule\":\"{}\",\"level\":{}",
                echapper_json(nom_regle(*rule)),
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
        TraceEvent::Attr {
            canal,
            noeud,
            valeur,
        } => (
            "Attr",
            format!(
                "\"canal\":\"{}\",\"noeud\":{},\"valeur\":\"{}\"",
                nom_canal(*canal),
                noeud.0,
                echapper_json(valeur)
            ),
        ),
        TraceEvent::MemoHit { noeud } => ("MemoHit", format!("\"noeud\":{}", noeud.0)),
        TraceEvent::MemoMiss { noeud } => ("MemoMiss", format!("\"noeud\":{}", noeud.0)),
        TraceEvent::Gc {
            vivants,
            collectes,
            compaction,
        } => (
            "Gc",
            format!("\"vivants\":{vivants},\"collectes\":{collectes},\"compaction\":{compaction}"),
        ),
        TraceEvent::Round { numero } => ("Round", format!("\"numero\":{numero}")),
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

    /// Writes the full trace (one line per event) to `sortie`.
    pub fn ecrire<W: Write>(&self, sortie: &mut W) -> std::io::Result<()> {
        for ligne in self.lignes() {
            writeln!(sortie, "{ligne}")?;
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
