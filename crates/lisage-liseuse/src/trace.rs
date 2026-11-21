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

/// Trace schema version (`{"v":5, ...}`): v2 adds the
/// `Attr`/`MemoHit`/`MemoMiss`/`Round` events of the step-by-step view (M2-6);
/// v3 adds the `Histogram` event (MEM-17: degree histogram per class);
/// v4 adds the spans of D1-1 — `SpanOpen`/`SpanClose` events and a `"span"`
/// field on every other event; v5 adds the register events of D1-2 —
/// `OpStart`/`OpEnd`/`VisiteEntree`/`VisiteSortie`/`MemoConsult`/`MemoInsert`/
/// `EcritureMemoire`/`StructureChange`, plus the extended `NodeCreated`
/// (`forme`), `Gc` (`liste_collectes`) and `Round` (levels, handles, growth).
pub const TRACE_VERSION: u32 = 5;

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

/// JSON name of a span kind (D1-1).
fn name_span_kind(kind: SpanKind) -> &'static str {
    match kind {
        SpanKind::Op => "op",
        SpanKind::Round => "round",
        SpanKind::Visite => "visite",
    }
}

/// JSON name of a structure change classification (D1-2).
fn name_structure_change(kind: StructureChangeKind) -> &'static str {
    match kind {
        StructureChangeKind::Identite => "identite",
        StructureChangeKind::Reetiquetage => "reetiquetage",
        StructureChangeKind::Collapse => "collapse",
        StructureChangeKind::Expansion => "expansion",
        StructureChangeKind::Restructuration => "restructuration",
    }
}

/// JSON rendering of an arc list `(key, target)` (D1-2):
/// `"arcs":[[0,12],[1,7]]`.
fn serialiser_arcs(arcs: &[(u64, NodeId)]) -> String {
    let entries: Vec<String> = arcs
        .iter()
        .map(|(key, target)| format!("[{},{}]", key, target.0))
        .collect();
    format!("[{}]", entries.join(","))
}

/// JSON rendering of a node id list (D1-2): `"liste":[3,7,12]`.
fn serialiser_liste_noeuds(liste: &[NodeId]) -> String {
    let entries: Vec<String> = liste.iter().map(|node| format!("{}", node.0)).collect();
    format!("[{}]", entries.join(","))
}

/// JSON rendering of a u32 list (D1-2): `"niveaux":[0,2]`.
fn serialiser_liste_niveaux(liste: &[u32]) -> String {
    let entries: Vec<String> = liste.iter().map(|niveau| format!("{niveau}")).collect();
    format!("[{}]", entries.join(","))
}

/// JSON rendering of an optional span id (D1-1): `"span":3` or `"span":null`.
fn serialiser_span(span: Option<lisage_core::observer::SpanId>) -> String {
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
/// D1-1 (v4): every event line carries its `"span"` field; `SpanOpen` and
/// `SpanClose` carry the span they define instead.
pub fn serialiser_evenement(evenement: &TraceEvent) -> String {
    let (type_evenement, champs) = match evenement {
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
            node, level, forme, ..
        } => (
            "NodeCreated",
            match level {
                Some(level) => format!(
                    "\"noeud\":{},\"level\":{},\"forme\":\"{}\"",
                    node.0,
                    level.0,
                    echapper_json(&forme.rendre())
                ),
                None => format!(
                    "\"noeud\":{},\"level\":null,\"forme\":\"{}\"",
                    node.0,
                    echapper_json(&forme.rendre())
                ),
            },
        ),
        TraceEvent::NodeShared { node, .. } => ("NodeShared", format!("\"noeud\":{}", node.0)),
        TraceEvent::ReductionApplied { rule, level, .. } => (
            "Reduction",
            format!(
                "\"rule\":\"{}\",\"level\":{}",
                echapper_json(name_regle(*rule)),
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
                echapper_json(&value.rendre())
            ),
        ),
        TraceEvent::MemoHit { node, .. } => ("MemoHit", format!("\"noeud\":{}", node.0)),
        TraceEvent::MemoMiss { node, .. } => ("MemoMiss", format!("\"noeud\":{}", node.0)),
        TraceEvent::Gc {
            live,
            collected,
            compaction,
            liste_collectes,
            ..
        } => (
            "Gc",
            format!(
                "\"vivants\":{live},\"collectes\":{collected},\"compaction\":{compaction},\"liste_collectes\":{}",
                serialiser_liste_noeuds(liste_collectes)
            ),
        ),
        TraceEvent::Round {
            numero,
            niveaux_actifs,
            niveaux_modifies,
            avant,
            image,
            apres,
            croissance,
            ..
        } => (
            "Round",
            format!(
                "\"numero\":{numero},\"niveaux_actifs\":{},\"niveaux_modifies\":{},\"avant\":{},\"image\":{},\"apres\":{},\"croissance\":{croissance}",
                serialiser_liste_niveaux(niveaux_actifs),
                serialiser_liste_niveaux(niveaux_modifies),
                avant.0,
                image.0,
                apres.0
            ),
        ),
        TraceEvent::Histogram { classes, .. } => ("Histogram", serialiser_histogramme(classes)),
        TraceEvent::OpStart { fingerprint, .. } => {
            ("OpStart", format!("\"empreinte\":{fingerprint}"))
        }
        TraceEvent::OpEnd { .. } => ("OpEnd", String::new()),
        TraceEvent::VisiteEntree { node, .. } => ("VisiteEntree", format!("\"noeud\":{}", node.0)),
        TraceEvent::VisiteSortie { node, resultat, .. } => (
            "VisiteSortie",
            format!("\"noeud\":{},\"resultat\":{}", node.0, resultat.0),
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
                echapper_json(&h.rendre())
            ),
        ),
        TraceEvent::MemoInsert {
            fingerprint,
            node,
            h,
            version,
            resultat,
            ..
        } => (
            "MemoInsert",
            format!(
                "\"empreinte\":{fingerprint},\"noeud\":{},\"h\":\"{}\",\"version\":{version},\"resultat\":{}",
                node.0,
                echapper_json(&h.rendre()),
                resultat.0
            ),
        ),
        TraceEvent::EcritureMemoire {
            version_avant,
            version_apres,
            ..
        } => (
            "EcritureMemoire",
            format!("\"version_avant\":{version_avant},\"version_apres\":{version_apres}"),
        ),
        TraceEvent::StructureChange {
            node,
            classification,
            arcs_avant,
            arcs_apres,
            ..
        } => (
            "StructureChange",
            format!(
                "\"noeud\":{},\"classification\":\"{}\",\"arcs_avant\":{},\"arcs_apres\":{}",
                node.0,
                name_structure_change(*classification),
                serialiser_arcs(arcs_avant),
                serialiser_arcs(arcs_apres)
            ),
        ),
    };
    let span = serialiser_span(evenement.span());
    if champs.is_empty() {
        format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_evenement}\",{span}}}")
    } else {
        format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_evenement}\",{span},{champs}}}")
    }
}

/// Observer that collects the trace in memory, serialized JSONL.
/// Cloneable: a clone placed in the pool shares the same trace.
/// Event filter of a `JsonlObserver` (D1-3).
pub type FiltreTrace = Arc<dyn Fn(&TraceEvent) -> bool + Send + Sync>;

#[derive(Clone)]
pub struct JsonlObserver {
    lignes: Arc<Mutex<Vec<String>>>,
    /// Optional event filter (D1-3): only events passing the filter are
    /// serialized — and only then are their lazy `TraceVal` values forced.
    filtre: Option<FiltreTrace>,
}

impl JsonlObserver {
    /// New observer, empty trace.
    pub fn new() -> Self {
        JsonlObserver {
            lignes: Arc::new(Mutex::new(Vec::new())),
            filtre: None,
        }
    }

    /// New observer keeping only the events that pass `filtre` (D1-3):
    /// the lazy values of the rejected events are never forced.
    pub fn avec_filtre(filtre: FiltreTrace) -> Self {
        JsonlObserver {
            lignes: Arc::new(Mutex::new(Vec::new())),
            filtre: Some(filtre),
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
        // D1-3: the filter runs on the cheap fields first; the lazy
        // values are forced (serialized) only if the filter passes.
        if let Some(filtre) = &self.filtre
            && !filtre(event)
        {
            return;
        }
        self.lignes
            .lock()
            .expect("mutex poisoned")
            .push(serialiser_evenement(event));
    }
}
