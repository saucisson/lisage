//! Liseuse v1 — **step by step** view (M2-6, liseuse doc § 6): scrubber on the
//! trace, colored channels (inherited blue / synthesized orange / memory purple),
//! `MemoHit`/`MemoMiss`, saturation rounds, and before/after differential
//! with **ghosts** (absorbed nodes rendered as watermark).
//!
//! The scrubber is a position `k` on the trace; the state at `k` is the
//! replay of events `0..k`. The static SVG rendering at a given position
//! embeds the event timeline — the interactive JS version is out of scope
//! (doc § 7: the static SVG output remains complete).

use std::collections::{BTreeMap, BTreeSet};

use lisage_core::observer::{CanalAttr, TraceEvent};
use lisage_core::{Form, NodeId, Pool};

use crate::svg::{
    DEMI_LARGEUR, HAUTEUR_NIVEAU, LARGEUR_NOEUD, MARGE_GAUCHE, MARGE_HAUT, Placement, echapper_xml,
    placer,
};

/// Color of the inherited channel (blue).
const COULEUR_HERITE: &str = "#2980b9";
/// Color of the synthesized channel (orange).
const COULEUR_SYNTHETISE: &str = "#e67e22";
/// Color of the level memory (purple).
const COULEUR_MEMOIRE: &str = "#8e44ad";
/// Color of ghosts (gray watermark).
const COULEUR_FANTOME: &str = "#bbb";
/// Default color of nodes without attribute (M2-6).
const COULEUR_DEFAUT: &str = "#333";
/// Color of a `MemoHit` on the timeline (green).
const COULEUR_MEMO_HIT: &str = "#27ae60";
/// Color of a `MemoMiss` on the timeline (red).
const COULEUR_MEMO_MISS: &str = "#c0392b";
/// Color of the other events on the timeline (neutral gray).
const COULEUR_AUTRE: &str = "#999";
/// Color of the diagram arcs.
const COULEUR_ARC: &str = "#555";
/// Color of the arc labels.
const COULEUR_ETIQUETTE: &str = "#777";
/// Color of the position cursor on the timeline (black).
const COULEUR_CURSEUR: &str = "#000";
/// Background of terminals.
const COULEUR_FOND_TERMINAL: &str = "#eee";
/// Background of internal nodes.
const COULEUR_FOND_NOEUD: &str = "#fff";
/// Height of the timeline in pixels.
const HAUTEUR_FRISE: u32 = 46;

/// State of the trace at a scrubber position: replay of events
/// `0..k`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EtatTrace {
    /// Nodes seen (created or shared) up to the position.
    pub noeuds: BTreeSet<NodeId>,
    /// Attributes seen `(channel, node, value)`, in emission order.
    pub attributs: Vec<(CanalAttr, NodeId, String)>,
    /// Number of `MemoHit` up to the position.
    pub memo_hits: usize,
    /// Number of `MemoMiss` up to the position.
    pub memo_misses: usize,
    /// Number of the last saturation round seen (0 = none).
    pub rounds: u32,
}

/// Before/after differential between two trace states: `fantomes` =
/// nodes present before but not after (rendered as watermark).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffTrace {
    /// Nodes present after but not before.
    pub ajoutes: BTreeSet<NodeId>,
    /// Nodes present before but not after (ghosts).
    pub fantomes: BTreeSet<NodeId>,
}

/// Differential between two states: additions and ghosts.
pub fn diff(avant: &EtatTrace, apres: &EtatTrace) -> DiffTrace {
    DiffTrace {
        ajoutes: apres.noeuds.difference(&avant.noeuds).copied().collect(),
        fantomes: avant.noeuds.difference(&apres.noeuds).copied().collect(),
    }
}

/// Scrubber: position on an event trace + state replay.
pub struct Scrubber {
    evenements: Vec<TraceEvent>,
    position: usize,
}

impl Scrubber {
    /// New scrubber on the given trace, positioned at the start.
    pub fn new(evenements: Vec<TraceEvent>) -> Self {
        Scrubber {
            evenements,
            position: 0,
        }
    }

    /// Number of events in the trace.
    pub fn longueur(&self) -> usize {
        self.evenements.len()
    }

    /// Current position of the scrubber.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Moves the scrubber to `position` (clamped to the trace length).
    pub fn aller_a(&mut self, position: usize) {
        self.position = position.min(self.evenements.len());
    }

    /// Replay of events `0..position`: state of the trace at the current
    /// position.
    pub fn etat(&self) -> EtatTrace {
        let mut etat = EtatTrace::default();
        for evenement in &self.evenements[..self.position] {
            match evenement {
                TraceEvent::NodeCreated { node, .. } | TraceEvent::NodeShared { node } => {
                    etat.noeuds.insert(*node);
                }
                TraceEvent::Attr {
                    canal,
                    noeud,
                    valeur,
                } => {
                    etat.attributs.push((*canal, *noeud, valeur.clone()));
                }
                TraceEvent::MemoHit { .. } => etat.memo_hits += 1,
                TraceEvent::MemoMiss { .. } => etat.memo_misses += 1,
                TraceEvent::Round { numero } => etat.rounds = etat.rounds.max(*numero),
                TraceEvent::ReductionApplied { .. }
                | TraceEvent::DiagramBuilt { .. }
                | TraceEvent::Gc { .. } => {}
            }
        }
        etat
    }

    /// Differential between the current position and position `autre`.
    pub fn diff_vers(&self, autre: usize) -> DiffTrace {
        let mut autre_scrubber = Scrubber::new(self.evenements.clone());
        autre_scrubber.aller_a(autre);
        diff(&self.etat(), &autre_scrubber.etat())
    }
}

/// Color of an attribute channel.
fn couleur_canal(canal: CanalAttr) -> &'static str {
    match canal {
        CanalAttr::Herite => COULEUR_HERITE,
        CanalAttr::Synthétise => COULEUR_SYNTHETISE,
        CanalAttr::Memoire => COULEUR_MEMOIRE,
    }
}

/// Border color of a node according to the **last** attribute channel seen
/// on this node (memory > synthesized > inherited > default).
fn couleur_noeud(etat: &EtatTrace, noeud: NodeId) -> &'static str {
    let mut couleur = COULEUR_DEFAUT;
    for (canal, n, _) in &etat.attributs {
        if *n == noeud {
            couleur = couleur_canal(*canal);
        }
    }
    couleur
}

/// Height and color of a timeline tick according to the event type.
fn tiret_frise(evenement: &TraceEvent) -> (u32, &'static str) {
    match evenement {
        TraceEvent::Round { .. } => (HAUTEUR_FRISE - 16, COULEUR_MEMOIRE),
        TraceEvent::MemoHit { .. } => (10, COULEUR_MEMO_HIT),
        TraceEvent::MemoMiss { .. } => (10, COULEUR_MEMO_MISS),
        TraceEvent::Attr { canal, .. } => (10, couleur_canal(*canal)),
        _ => (6, COULEUR_AUTRE),
    }
}

/// Opens the SVG document and renders the title (position, rounds, memo counters).
fn ouvrir_svg(svg: &mut String, largeur: u32, hauteur: u32, scrubber: &Scrubber, etat: &EtatTrace) {
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"20\" font-family=\"serif\" font-size=\"14\">step by step — position {}/{}, rounds {}, memo {}/{}</text>\n",
        scrubber.position(),
        scrubber.longueur(),
        etat.rounds,
        etat.memo_hits,
        etat.memo_misses
    ));
}

/// Renders the timeline: one tick per event, and the cursor of the
/// current position highlighted.
fn rendre_frise(svg: &mut String, scrubber: &Scrubber, largeur: u32) {
    let y_frise = MARGE_HAUT + 8;
    let largeur_frise = largeur - 2 * MARGE_GAUCHE;
    let pas = if scrubber.longueur() == 0 {
        0
    } else {
        largeur_frise / scrubber.longueur() as u32
    };
    for (index, evenement) in scrubber.evenements.iter().enumerate() {
        let x = MARGE_GAUCHE + index as u32 * pas;
        let (hauteur_tiret, couleur) = tiret_frise(evenement);
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{couleur}\" stroke-width=\"2\"/>\n",
            y_frise + HAUTEUR_FRISE - 16 - hauteur_tiret,
            y_frise + HAUTEUR_FRISE - 16
        ));
    }
    if scrubber.position() > 0 && scrubber.position() <= scrubber.longueur() {
        let x = MARGE_GAUCHE + scrubber.position() as u32 * pas;
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{COULEUR_CURSEUR}\" stroke-width=\"2\"/>\n",
            y_frise,
            y_frise + HAUTEUR_FRISE - 12
        ));
    }
}

/// Renders the diagram arcs (under the nodes) with their labels.
fn rendre_arcs<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    racine: NodeId,
    placement: &BTreeMap<NodeId, Placement>,
    y_decalage: u32,
) {
    let mut pile = vec![racine];
    let mut deja_vu = BTreeSet::new();
    while let Some(noeud) = pile.pop() {
        if !deja_vu.insert(noeud) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(noeud) {
            let depart = &placement[&noeud];
            for (cle, cible) in arcs {
                let arrivee = &placement[cible];
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COULEUR_ARC}\"/>\n",
                    depart.x,
                    depart.y + y_decalage + 14,
                    arrivee.x,
                    arrivee.y + y_decalage - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"{COULEUR_ETIQUETTE}\">{}</text>\n",
                    (depart.x + arrivee.x) / 2,
                    (depart.y + arrivee.y) / 2 + y_decalage,
                    echapper_xml(&cle.to_string())
                ));
                pile.push(*cible);
            }
        }
    }
}

/// Renders a node (terminal or internal); as a ghost (dashed) if requested.
fn rendre_noeud<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    noeud: NodeId,
    position: &Placement,
    fantome: bool,
    etat: &EtatTrace,
    y_decalage: u32,
) {
    let couleur = if fantome {
        COULEUR_FANTOME
    } else {
        couleur_noeud(etat, noeud)
    };
    let style = if fantome {
        format!("stroke=\"{couleur}\" stroke-dasharray=\"4 3\"")
    } else {
        format!("stroke=\"{couleur}\" stroke-width=\"2\"")
    };
    let x = position.x - DEMI_LARGEUR;
    let y = position.y + y_decalage - 14;
    match pool.resolve(noeud) {
        Ok(Form::Terminal(payload)) => {
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"{COULEUR_FOND_TERMINAL}\" {style}/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">{}</text>\n",
                position.x,
                position.y + y_decalage + 4,
                if *payload { "⊤" } else { "⊥" }
            ));
        }
        Ok(Form::Node { level, .. }) => {
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"{COULEUR_FOND_NOEUD}\" {style}/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                position.x,
                position.y + y_decalage + 4,
                level.0,
                noeud.0
            ));
        }
        Err(_) => {
            // Unreachable: the placement is derived from the same pool via
            // `placer`, so `resolve` already succeeded on this node.
            debug_assert!(false, "resolve failed on a node of the placement");
        }
    }
}

/// Renders all the nodes of the placement, ghosts as watermark.
fn rendre_noeuds<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    placement: &BTreeMap<NodeId, Placement>,
    diff: &DiffTrace,
    etat: &EtatTrace,
    y_decalage: u32,
) {
    for (noeud, position) in placement {
        rendre_noeud(
            svg,
            pool,
            *noeud,
            position,
            diff.fantomes.contains(noeud),
            etat,
            y_decalage,
        );
    }
}

/// Renders in static SVG the step-by-step view at the scrubber position:
/// event timeline (rounds as vertical bars, current position
/// highlighted), diagram with colored channels, ghosts as watermark, and
/// dashboard (hits/misses/rounds).
pub fn rendre_svg_pas_a_pas<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    scrubber: &Scrubber,
    diff: &DiffTrace,
) -> String {
    let etat = scrubber.etat();
    let (placement, nombre_niveaux) = placer(pool, racine);
    let largeur =
        MARGE_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_LARGEUR;
    let hauteur = MARGE_HAUT + HAUTEUR_FRISE + (nombre_niveaux + 1) * HAUTEUR_NIVEAU + 20;
    let mut svg = String::new();
    ouvrir_svg(&mut svg, largeur, hauteur, scrubber, &etat);
    rendre_frise(&mut svg, scrubber, largeur);
    let y_decalage = HAUTEUR_FRISE + 8;
    rendre_arcs(&mut svg, pool, racine, &placement, y_decalage);
    rendre_noeuds(&mut svg, pool, &placement, diff, &etat, y_decalage);
    svg.push_str("</svg>\n");
    svg
}

/// Legend of the channels (colors) for the step-by-step rendering.
pub fn legende_canaux() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("inherited", COULEUR_HERITE),
        ("synthesized", COULEUR_SYNTHETISE),
        ("memory", COULEUR_MEMOIRE),
        ("ghost", COULEUR_FANTOME),
    ])
}
