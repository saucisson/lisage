//! Sérialiseur de trace **JSONL versionné** (M1-9 Liseuse v0, M2-6 v1 pas à
//! pas) : une ligne JSON par événement, schéma `{"v":2, ...}` (voir
//! `docs/reader-visualizer-of-diagram-orders-and-operations.md` § 3).
//! Sérialisation écrite à la main : aucune dépendance externe (liste
//! d'escalation AGENTS.md).

use std::io::Write;
use std::sync::{Arc, Mutex};

use lisage_core::observer::{CanalAttr, Observer, ReductionRule, TraceEvent};

/// Version du schéma de trace (`{"v":2, ...}`) : v2 ajoute les événements
/// `Attr`/`MemoHit`/`MemoMiss`/`Round` de la vue pas à pas (M2-6).
pub const TRACE_VERSION: u32 = 2;

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

/// Nom JSON d'un canal d'attribut (M2-6).
fn nom_canal(canal: CanalAttr) -> &'static str {
    match canal {
        CanalAttr::Herite => "herite",
        CanalAttr::Synthétise => "synthetise",
        CanalAttr::Memoire => "memoire",
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
