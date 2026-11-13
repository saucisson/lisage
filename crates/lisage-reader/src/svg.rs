//! Reader v0 — **static SVG** rendering of the diagram (M1-9) : levels as
//! horizontal lines (level = key of the order), nodes laid out in x by
//! deterministic traversal order, terminals at the bottom. No external
//! dependency; minimal XML escaping.

use std::collections::BTreeMap;

use lisage_core::{Form, Level, NodeId, Pool};

/// Width of a node in pixels.
pub(crate) const WIDTH_NODE: u32 = 56;
/// Height of a level in pixels.
pub(crate) const HEIGHT_LEVEL: u32 = 90;
/// Top margin (title) in pixels.
pub(crate) const MARGIN_HAUT: u32 = 40;
/// Left margin in pixels.
pub(crate) const MARGIN_GAUCHE: u32 = 40;
/// Half-width of a node.
pub(crate) const DEMI_WIDTH: u32 = WIDTH_NODE / 2;

/// Minimal XML escaping.
pub(crate) fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Position of a node: `(x, y)` of its center.
pub(crate) struct Placement {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

/// Collects the nodes reachable from `root`, grouped by level, and
/// computes a deterministic placement (depth-first traversal, x increasing
/// in order of first visit within each level).
pub(crate) fn placer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
) -> (BTreeMap<NodeId, Placement>, u32) {
    let mut levels: BTreeMap<u32, Vec<NodeId>> = BTreeMap::new();
    let mut visits: Vec<NodeId> = Vec::new();
    let mut stack = vec![root];
    let mut already_seen = std::collections::BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !already_seen.insert(node) {
            continue;
        }
        visits.push(node);
        if let Ok(Form::Node { level, edges }) = pool.resolve(node) {
            levels.entry(level.0).or_default().push(node);
            // Push in reverse order to visit edges in increasing order.
            for target in edges.values().rev() {
                stack.push(*target);
            }
        }
    }
    let number_levels = levels.len() as u32;
    let mut placement = BTreeMap::new();
    for (index_level, (_level, nodes)) in levels.iter().enumerate() {
        let y = MARGIN_HAUT + index_level as u32 * HEIGHT_LEVEL + HEIGHT_LEVEL / 2;
        for (index, node) in nodes.iter().enumerate() {
            let x = MARGIN_GAUCHE + index as u32 * (WIDTH_NODE + 24) + DEMI_WIDTH;
            placement.insert(*node, Placement { x, y });
        }
    }
    // Terminals: bottom line.
    let y_terminal = MARGIN_HAUT + number_levels * HEIGHT_LEVEL + HEIGHT_LEVEL / 2;
    let mut index_terminal = 0u32;
    for node in &visits {
        if !placement.contains_key(node) {
            let x = MARGIN_GAUCHE + index_terminal * (WIDTH_NODE + 24) + DEMI_WIDTH;
            placement.insert(*node, Placement { x, y: y_terminal });
            index_terminal += 1;
        }
    }
    (placement, number_levels)
}

/// Color palette of the SVG rendering.
pub(crate) struct Colors {
    pub(crate) ink: &'static str,
    pub(crate) edge: &'static str,
    pub(crate) label: &'static str,
    pub(crate) background_node: &'static str,
    pub(crate) background_terminal: &'static str,
}

/// Default colors of the debug rendering.
pub(crate) const COLORS_DEBUG: Colors = Colors {
    ink: "#333",
    edge: "#555",
    label: "#777",
    background_node: "#fff",
    background_terminal: "#eee",
};

/// Draws the edges then the nodes of the sub-diagram reachable from
/// `root`, with the given placement and palette.
pub(crate) fn draw_body<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    placement: &BTreeMap<NodeId, Placement>,
    colors: &Colors,
) -> String {
    let mut svg = String::new();
    // Edges first (under the nodes).
    let mut stack = vec![root];
    let mut already_seen = std::collections::BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !already_seen.insert(node) {
            continue;
        }
        if let Ok(Form::Node { edges, .. }) = pool.resolve(node) {
            let start = &placement[&node];
            for (key, target) in edges {
                let end = &placement[target];
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\"/>\n",
                    start.x,
                    start.y + 14,
                    end.x,
                    end.y - 14,
                    colors.edge
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"{}\">{}</text>\n",
                    (start.x + end.x) / 2,
                    (start.y + end.y) / 2,
                    colors.label,
                    escape_xml(&key.to_string())
                ));
                stack.push(*target);
            }
        }
    }
    // Nodes afterwards.
    for (node, position) in placement {
        match pool.resolve(*node) {
            Ok(Form::Terminal(payload)) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{}\" stroke=\"{}\"/>\n",
                    position.x - DEMI_WIDTH,
                    position.y - 14,
                    colors.background_terminal,
                    colors.ink
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
                    colors.background_node,
                    colors.ink
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
pub fn render_svg<P: Pool<Key = u64, Payload = bool>>(pool: &P, root: NodeId) -> String {
    let (placement, number_levels) = placer(pool, root);
    let width = MARGIN_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_WIDTH;
    let height = MARGIN_HAUT + (number_levels + 1) * HEIGHT_LEVEL + 20;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGIN_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"14\">diagram (levels = keys)</text>\n"
    ));
    svg.push_str(&draw_body(pool, root, &placement, &COLORS_DEBUG));
    svg.push_str("</svg>\n");
    svg
}

/// Displayed level rule (for tests and the legend).
pub fn label_level(level: Level) -> String {
    format!("ℓ{}", level.0)
}
