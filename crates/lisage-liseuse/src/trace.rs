//! Sérialiseur de trace **JSONL versionné** (M1-9, Liseuse v0) : une ligne
//! JSON par événement, schéma `{"v":1, ...}` (voir
//! `docs/liseuse-visualiseur-d-ordres-de-diagrammes-et-d-opérations.md` § 3).
//! Sérialisation écrite à la main : aucune dépendance externe (liste
//! d'escalation AGENTS.md).

use std::io::Write;
use std::sync::{Arc, Mutex};

use lisage_core::observer::{Observer, ReductionRule, TraceEvent};

/// Version du schéma de trace (`{"v":1, ...}`).
pub const TRACE_VERSION: u32 = 1;

/// Échappement JSON minimal d'une chaîne (les événements n'en portent pas
/// de dynamique, mais le contrat reste correct).
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

/// Nom JSON d'une règle de réduction.
fn nom_regle(regle: ReductionRule) -> &'static str {
    match regle {
        ReductionRule::Share => "R-share",
        ReductionRule::Collapse => "R-collapse",
        ReductionRule::Default => "R-default",
    }
}

/// Sérialise un événement de trace en une ligne JSONL (sans le `\n` final).
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
    };
    format!("{{\"v\":{TRACE_VERSION},\"type\":\"{type_evenement}\",{champs}}}")
}

/// Observateur qui collecte la trace en mémoire, sérialisée JSONL.
/// Clonable : un clone posé dans le pool partage la même trace.
#[derive(Clone)]
pub struct JsonlObserver {
    lignes: Arc<Mutex<Vec<String>>>,
}

impl JsonlObserver {
    /// Observateur neuf, trace vide.
    pub fn new() -> Self {
        JsonlObserver {
            lignes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Lignes JSONL collectées (dans l'ordre d'émission).
    pub fn lignes(&self) -> Vec<String> {
        self.lignes.lock().expect("mutex poisoned").clone()
    }

    /// Écrit la trace complète (une ligne par événement) dans `sortie`.
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
