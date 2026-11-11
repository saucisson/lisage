//! Liseuse v1 — vue **pas à pas** (M2-6, doc liseuse § 6) : scrubber sur la
//! trace, canaux colorés (hérité bleu / synthétisé orange / mémoire violet),
//! `MemoHit`/`MemoMiss`, rounds de saturation, et différentiel avant/après
//! avec **fantômes** (nœuds absorbés rendus en filigrane).
//!
//! Le scrubber est une position `k` sur la trace ; l'état à `k` est le
//! replay des événements `0..k`. Le rendu SVG statique à une position
//! donnée embarque la frise (timeline) des événements — la version JS
//! interactive est hors périmètre (doc § 7 : la sortie SVG statique reste
//! complète).

use std::collections::{BTreeMap, BTreeSet};

use lisage_core::observer::{CanalAttr, TraceEvent};
use lisage_core::{Form, NodeId, Pool};

use crate::svg::{
    DEMI_LARGEUR, HAUTEUR_NIVEAU, LARGEUR_NOEUD, MARGE_GAUCHE, MARGE_HAUT, echapper_xml, placer,
};

/// Couleur du canal hérité (bleu).
const COULEUR_HERITE: &str = "#2980b9";
/// Couleur du canal synthétisé (orange).
const COULEUR_SYNTHETISE: &str = "#e67e22";
/// Couleur de la mémoire de niveau (violet).
const COULEUR_MEMOIRE: &str = "#8e44ad";
/// Couleur des fantômes (filigrane gris).
const COULEUR_FANTOME: &str = "#bbb";
/// Couleur par défaut des nœuds sans attribut (M2-6).
const COULEUR_DEFAUT: &str = "#333";
/// Hauteur de la frise (timeline) en pixels.
const HAUTEUR_FRISE: u32 = 46;

/// État de la trace à une position de scrubber : replay des événements
/// `0..k`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EtatTrace {
    /// Nœuds vus (créés ou partagés) jusqu'à la position.
    pub noeuds: BTreeSet<NodeId>,
    /// Attributs vus `(canal, nœud, valeur)`, dans l'ordre d'émission.
    pub attributs: Vec<(CanalAttr, NodeId, String)>,
    /// Nombre de `MemoHit` jusqu'à la position.
    pub memo_hits: usize,
    /// Nombre de `MemoMiss` jusqu'à la position.
    pub memo_misses: usize,
    /// Numéro du dernier round de saturation vu (0 = aucun).
    pub rounds: u32,
}

/// Différentiel avant/après entre deux états de trace : `fantomes` =
/// nœuds présents avant mais plus après (rendus en filigrane).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffTrace {
    /// Nœuds présents après mais pas avant.
    pub ajoutes: BTreeSet<NodeId>,
    /// Nœuds présents avant mais plus après (fantômes).
    pub fantomes: BTreeSet<NodeId>,
}

/// Différentiel entre deux états : ajouts et fantômes.
pub fn diff(avant: &EtatTrace, apres: &EtatTrace) -> DiffTrace {
    DiffTrace {
        ajoutes: apres.noeuds.difference(&avant.noeuds).copied().collect(),
        fantomes: avant.noeuds.difference(&apres.noeuds).copied().collect(),
    }
}

/// Scrubber : position sur une trace d'événements + replay de l'état.
pub struct Scrubber {
    evenements: Vec<TraceEvent>,
    position: usize,
}

impl Scrubber {
    /// Scrubber neuf sur la trace donnée, positionné au début.
    pub fn new(evenements: Vec<TraceEvent>) -> Self {
        Scrubber {
            evenements,
            position: 0,
        }
    }

    /// Nombre d'événements de la trace.
    pub fn longueur(&self) -> usize {
        self.evenements.len()
    }

    /// Position courante du scrubber.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Déplace le scrubber à `position` (bornée à la longueur de la trace).
    pub fn aller_a(&mut self, position: usize) {
        self.position = position.min(self.evenements.len());
    }

    /// Replay des événements `0..position` : état de la trace à la position
    /// courante.
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
                TraceEvent::ReductionApplied { .. } | TraceEvent::DiagramBuilt { .. } => {}
            }
        }
        etat
    }

    /// Différentiel entre la position courante et la position `autre`.
    pub fn diff_vers(&self, autre: usize) -> DiffTrace {
        let mut autre_scrubber = Scrubber::new(self.evenements.clone());
        autre_scrubber.aller_a(autre);
        diff(&self.etat(), &autre_scrubber.etat())
    }
}

/// Couleur de bordure d'un nœud selon le **dernier** canal d'attribut vu
/// sur ce nœud (mémoire > synthétisé > hérité > défaut).
fn couleur_noeud(etat: &EtatTrace, noeud: NodeId) -> &'static str {
    let mut couleur = COULEUR_DEFAUT;
    for (canal, n, _) in &etat.attributs {
        if *n == noeud {
            couleur = match canal {
                CanalAttr::Herite => COULEUR_HERITE,
                CanalAttr::Synthétise => COULEUR_SYNTHETISE,
                CanalAttr::Memoire => COULEUR_MEMOIRE,
            };
        }
    }
    couleur
}

/// Rend en SVG statique la vue pas à pas à la position du scrubber :
/// frise des événements (rounds en barres verticales, position courante
/// surlignée), diagramme avec canaux colorés, fantômes en filigrane, et
/// tableau de bord (hits/misses/rounds).
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
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"20\" font-family=\"serif\" font-size=\"14\">pas à pas — position {}/{}, rounds {}, memo {}/{}</text>\n",
        scrubber.position(),
        scrubber.longueur(),
        etat.rounds,
        etat.memo_hits,
        etat.memo_misses
    ));
    // Frise (timeline) : un tiret par événement, barre verticale par round,
    // position courante surlignée.
    let y_frise = MARGE_HAUT + 8;
    let largeur_frise = largeur - 2 * MARGE_GAUCHE;
    let pas = if scrubber.longueur() == 0 {
        0
    } else {
        largeur_frise / scrubber.longueur() as u32
    };
    for (index, evenement) in scrubber.evenements.iter().enumerate() {
        let x = MARGE_GAUCHE + index as u32 * pas;
        let (hauteur_tiret, couleur) = match evenement {
            TraceEvent::Round { .. } => (HAUTEUR_FRISE - 16, COULEUR_MEMOIRE),
            TraceEvent::MemoHit { .. } => (10, "#27ae60"),
            TraceEvent::MemoMiss { .. } => (10, "#c0392b"),
            TraceEvent::Attr { canal, .. } => (
                10,
                match canal {
                    CanalAttr::Herite => COULEUR_HERITE,
                    CanalAttr::Synthétise => COULEUR_SYNTHETISE,
                    CanalAttr::Memoire => COULEUR_MEMOIRE,
                },
            ),
            _ => (6, "#999"),
        };
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{couleur}\" stroke-width=\"2\"/>\n",
            y_frise + HAUTEUR_FRISE - 16 - hauteur_tiret,
            y_frise + HAUTEUR_FRISE - 16
        ));
    }
    if scrubber.position() > 0 && scrubber.position() <= scrubber.longueur() {
        let x = MARGE_GAUCHE + scrubber.position() as u32 * pas;
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"#000\" stroke-width=\"2\"/>\n",
            y_frise,
            y_frise + HAUTEUR_FRISE - 12
        ));
    }
    // Arcs d'abord (sous les nœuds).
    let y_decalage = HAUTEUR_FRISE + 8;
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
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#555\"/>\n",
                    depart.x,
                    depart.y + y_decalage + 14,
                    arrivee.x,
                    arrivee.y + y_decalage - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"#777\">{}</text>\n",
                    (depart.x + arrivee.x) / 2,
                    (depart.y + arrivee.y) / 2 + y_decalage,
                    echapper_xml(&cle.to_string())
                ));
                pile.push(*cible);
            }
        }
    }
    // Nœuds : couleur du canal, fantômes en pointillés.
    for (noeud, position) in &placement {
        let fantome = diff.fantomes.contains(noeud);
        let couleur = if fantome {
            COULEUR_FANTOME
        } else {
            couleur_noeud(&etat, *noeud)
        };
        let style = if fantome {
            format!("stroke=\"{couleur}\" stroke-dasharray=\"4 3\"")
        } else {
            format!("stroke=\"{couleur}\" stroke-width=\"2\"")
        };
        match pool.resolve(*noeud) {
            Ok(Form::Terminal(payload)) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"#eee\" {style}/>\n",
                    position.x - DEMI_LARGEUR,
                    position.y + y_decalage - 14
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
                    "<rect x=\"{}\" y=\"{}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"#fff\" {style}/>\n",
                    position.x - DEMI_LARGEUR,
                    position.y + y_decalage - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                    position.x,
                    position.y + y_decalage + 4,
                    level.0,
                    noeud.0
                ));
            }
            Err(_) => {}
        }
    }
    svg.push_str("</svg>\n");
    svg
}

/// Légende des canaux (couleurs) pour le rendu pas à pas.
pub fn legende_canaux() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("herite", COULEUR_HERITE),
        ("synthetise", COULEUR_SYNTHETISE),
        ("memoire", COULEUR_MEMOIRE),
        ("fantome", COULEUR_FANTOME),
    ])
}
