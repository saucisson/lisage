//! Liseuse v0 — **static SVG** rendering of the diagram (M1-9): levels in
//! horizontal lines (level = key of the order), nodes arranged in x by
//! deterministic traversal order, terminals at the bottom. No external
//! dependency; minimal XML escaping.

use std::collections::BTreeMap;

use lisage_core::{Form, Level, NodeId, Pool};

/// Node width in pixels.
pub(crate) const WIDTH_NODE: u32 = 56;
/// Level height in pixels.
pub(crate) const HEIGHT_LEVEL: u32 = 90;
/// Top margin (title) in pixels.
pub(crate) const MARGE_HAUT: u32 = 40;
/// Left margin in pixels.
pub(crate) const MARGE_GAUCHE: u32 = 40;
/// Half node width.
pub(crate) const DEMI_WIDTH: u32 = WIDTH_NODE / 2;

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

/// Collects the nodes reachable from `root`, grouped by level, and
/// computes a deterministic placement (depth-first traversal, x increasing
/// by order of first visit in each level).
pub(crate) fn placer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
) -> (BTreeMap<NodeId, Placement>, u32) {
    let mut levels: BTreeMap<u32, Vec<NodeId>> = BTreeMap::new();
    let mut visites: Vec<NodeId> = Vec::new();
    let mut stack = vec![root];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !deja_vu.insert(node) {
            continue;
        }
        visites.push(node);
        if let Ok(Form::Node { level, arcs }) = pool.resolve(node) {
            levels.entry(level.0).or_default().push(node);
            // Push in reverse order to visit the arcs in increasing order.
            for cible in arcs.values().rev() {
                stack.push(*cible);
            }
        }
    }
    let number_levels = levels.len() as u32;
    let mut placement = BTreeMap::new();
    for (index_level, (_level, nodes)) in levels.iter().enumerate() {
        let y = MARGE_HAUT + index_level as u32 * HEIGHT_LEVEL + HEIGHT_LEVEL / 2;
        for (index, node) in nodes.iter().enumerate() {
            let x = MARGE_GAUCHE + index as u32 * (WIDTH_NODE + 24) + DEMI_WIDTH;
            placement.insert(*node, Placement { x, y });
        }
    }
    // Terminals: bottom line.
    let y_terminal = MARGE_HAUT + number_levels * HEIGHT_LEVEL + HEIGHT_LEVEL / 2;
    let mut index_terminal = 0u32;
    for node in &visites {
        if !placement.contains_key(node) {
            let x = MARGE_GAUCHE + index_terminal * (WIDTH_NODE + 24) + DEMI_WIDTH;
            placement.insert(*node, Placement { x, y: y_terminal });
            index_terminal += 1;
        }
    }
    (placement, number_levels)
}

/// Color palette of the SVG rendering.
pub(crate) struct Couleurs {
    pub(crate) encre: &'static str,
    pub(crate) arete: &'static str,
    pub(crate) label: &'static str,
    pub(crate) fond_node: &'static str,
    pub(crate) fond_terminal: &'static str,
}

/// Default colors of the debug rendering.
pub(crate) const COULEURS_DEBUG: Couleurs = Couleurs {
    encre: "#333",
    arete: "#555",
    label: "#777",
    fond_node: "#fff",
    fond_terminal: "#eee",
};

/// Draws the arcs then the nodes of the sub-diagram reachable from
/// `root`, with the given placement and palette.
pub(crate) fn dessiner_corps<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    placement: &BTreeMap<NodeId, Placement>,
    couleurs: &Couleurs,
) -> String {
    let mut svg = String::new();
    // Arcs first (under the nodes).
    let mut stack = vec![root];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !deja_vu.insert(node) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(node) {
            let depart = &placement[&node];
            for (key, cible) in arcs {
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
                    couleurs.label,
                    echapper_xml(&key.to_string())
                ));
                stack.push(*cible);
            }
        }
    }
    // Nodes next.
    for (node, position) in placement {
        match pool.resolve(*node) {
            Ok(Form::Terminal(payload)) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{}\" stroke=\"{}\"/>\n",
                    position.x - DEMI_WIDTH,
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
                    "<rect x=\"{}\" y=\"{}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{}\" stroke=\"{}\"/>\n",
                    position.x - DEMI_WIDTH,
                    position.y - 14,
                    couleurs.fond_node,
                    couleurs.encre
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                    position.x,
                    position.y + 4,
                    level.0,
                    node.0
                ));
            }
            Err(_) => {}
        }
    }
    svg
}

/// Renders the sub-diagram reachable from `root` as static SVG.
pub fn rendre_svg<P: Pool<Key = u64, Payload = bool>>(pool: &P, root: NodeId) -> String {
    let (placement, number_levels) = placer(pool, root);
    let width = MARGE_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_WIDTH;
    let height = MARGE_HAUT + (number_levels + 1) * HEIGHT_LEVEL + 20;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"14\">diagram (levels = keys)</text>\n"
    ));
    svg.push_str(&dessiner_corps(pool, root, &placement, &COULEURS_DEBUG));
    svg.push_str("</svg>\n");
    svg
}

/// Displayed level rule (for tests and the legend).
pub fn label_level(level: Level) -> String {
    format!("ℓ{}", level.0)
}
