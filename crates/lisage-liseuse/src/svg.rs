//! Liseuse v0 — **static SVG** rendering of the diagram (M1-9): levels in
//! horizontal lines (level = key of the order), nodes arranged in x by
//! deterministic traversal order, terminals at the bottom. No external
//! dependency; minimal XML escaping.

use std::collections::BTreeMap;

use lisage_core::{Form, Level, NodeId, Pool};

/// Node width in pixels.
pub(crate) const LARGEUR_NOEUD: u32 = 56;
/// Level height in pixels.
pub(crate) const HAUTEUR_NIVEAU: u32 = 90;
/// Top margin (title) in pixels.
pub(crate) const MARGE_HAUT: u32 = 40;
/// Left margin in pixels.
pub(crate) const MARGE_GAUCHE: u32 = 40;
/// Half node width.
pub(crate) const DEMI_LARGEUR: u32 = LARGEUR_NOEUD / 2;

/// Minimal XML escaping.
pub(crate) fn echapper_xml(texte: &str) -> String {
    texte
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Position of a node: `(x, y)` of the center.
pub(crate) struct Placement {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

/// Collects the nodes reachable from `racine`, grouped by level, and
/// computes a deterministic placement (depth-first traversal, x increasing
/// by order of first visit in each level).
pub(crate) fn placer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
) -> (BTreeMap<NodeId, Placement>, u32) {
    let mut niveaux: BTreeMap<u32, Vec<NodeId>> = BTreeMap::new();
    let mut visites: Vec<NodeId> = Vec::new();
    let mut pile = vec![racine];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(noeud) = pile.pop() {
        if !deja_vu.insert(noeud) {
            continue;
        }
        visites.push(noeud);
        if let Ok(Form::Node { level, arcs }) = pool.resolve(noeud) {
            niveaux.entry(level.0).or_default().push(noeud);
            // Push in reverse order to visit the arcs in increasing order.
            for cible in arcs.values().rev() {
                pile.push(*cible);
            }
        }
    }
    let nombre_niveaux = niveaux.len() as u32;
    let mut placement = BTreeMap::new();
    for (index_niveau, (_niveau, noeuds)) in niveaux.iter().enumerate() {
        let y = MARGE_HAUT + index_niveau as u32 * HAUTEUR_NIVEAU + HAUTEUR_NIVEAU / 2;
        for (index, noeud) in noeuds.iter().enumerate() {
            let x = MARGE_GAUCHE + index as u32 * (LARGEUR_NOEUD + 24) + DEMI_LARGEUR;
            placement.insert(*noeud, Placement { x, y });
        }
    }
    // Terminals: bottom line.
    let y_terminal = MARGE_HAUT + nombre_niveaux * HAUTEUR_NIVEAU + HAUTEUR_NIVEAU / 2;
    let mut index_terminal = 0u32;
    for noeud in &visites {
        if !placement.contains_key(noeud) {
            let x = MARGE_GAUCHE + index_terminal * (LARGEUR_NOEUD + 24) + DEMI_LARGEUR;
            placement.insert(*noeud, Placement { x, y: y_terminal });
            index_terminal += 1;
        }
    }
    (placement, nombre_niveaux)
}

/// Color palette of the SVG rendering.
pub(crate) struct Couleurs {
    pub(crate) encre: &'static str,
    pub(crate) arete: &'static str,
    pub(crate) etiquette: &'static str,
    pub(crate) fond_noeud: &'static str,
    pub(crate) fond_terminal: &'static str,
}

/// Default colors of the debug rendering.
pub(crate) const COULEURS_DEBUG: Couleurs = Couleurs {
    encre: "#333",
    arete: "#555",
    etiquette: "#777",
    fond_noeud: "#fff",
    fond_terminal: "#eee",
};

/// Draws the arcs then the nodes of the sub-diagram reachable from
/// `racine`, with the given placement and palette.
pub(crate) fn dessiner_corps<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    placement: &BTreeMap<NodeId, Placement>,
    couleurs: &Couleurs,
) -> String {
    let mut svg = String::new();
    // Arcs first (under the nodes).
    let mut pile = vec![racine];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(noeud) = pile.pop() {
        if !deja_vu.insert(noeud) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(noeud) {
            let depart = &placement[&noeud];
            for (cle, cible) in arcs {
                let arrivee = &placement[cible];
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\"/>\n",
                    depart.x,
                    depart.y + 14,
                    arrivee.x,
                    arrivee.y - 14,
                    couleurs.arete
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"{}\">{}</text>\n",
                    (depart.x + arrivee.x) / 2,
                    (depart.y + arrivee.y) / 2,
                    couleurs.etiquette,
                    echapper_xml(&cle.to_string())
                ));
                pile.push(*cible);
            }
        }
    }
    // Nodes next.
    for (noeud, position) in placement {
        match pool.resolve(*noeud) {
            Ok(Form::Terminal(payload)) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"{}\" stroke=\"{}\"/>\n",
                    position.x - DEMI_LARGEUR,
                    position.y - 14,
                    couleurs.fond_terminal,
                    couleurs.encre
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">{}</text>\n",
                    position.x,
                    position.y + 4,
                    if *payload { "⊤" } else { "⊥" }
                ));
            }
            Ok(Form::Node { level, .. }) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"{}\" stroke=\"{}\"/>\n",
                    position.x - DEMI_LARGEUR,
                    position.y - 14,
                    couleurs.fond_noeud,
                    couleurs.encre
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                    position.x,
                    position.y + 4,
                    level.0,
                    noeud.0
                ));
            }
            Err(_) => {}
        }
    }
    svg
}

/// Renders the sub-diagram reachable from `racine` as static SVG.
pub fn rendre_svg<P: Pool<Key = u64, Payload = bool>>(pool: &P, racine: NodeId) -> String {
    let (placement, nombre_niveaux) = placer(pool, racine);
    let largeur =
        MARGE_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_LARGEUR;
    let hauteur = MARGE_HAUT + (nombre_niveaux + 1) * HAUTEUR_NIVEAU + 20;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"14\">diagram (levels = keys)</text>\n"
    ));
    svg.push_str(&dessiner_corps(pool, racine, &placement, &COULEURS_DEBUG));
    svg.push_str("</svg>\n");
    svg
}

/// Displayed level rule (for tests and the legend).
pub fn etiquette_niveau(niveau: Level) -> String {
    format!("ℓ{}", niveau.0)
}
