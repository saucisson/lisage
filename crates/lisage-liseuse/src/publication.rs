//! Publication mode (M5-5): export of figures ready for article
//! layout — standalone SVG or PGF/TikZ, serif theme, **legend
//! auto-generated from the channel legend** (`ChannelAttr`): the same
//! file used for debugging is used for publication (liseuse doc § 5).

use lisage_core::observer::ChannelAttr;
use lisage_core::{NodeId, Pool};

use crate::svg::{
    Colors, HALF_WIDTH, HEIGHT_LEVEL, MARGIN_HAUT, MARGIN_LEFT, draw_body, escape_xml, placer,
};

/// Publication theme: serif, dark ink on white background, channel colors
/// aligned with the `ChannelAttr` legend (inherited = blue, synthesized =
/// orange, memory = purple).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Theme {
    /// Font (serif for article layout).
    pub police: &'static str,
    /// Title size in pixels.
    pub size_title: u32,
    /// Node text size in pixels.
    pub size_text: u32,
    /// Legend size in pixels.
    pub size_legend: u32,
    /// Ink color (outlines, text).
    pub ink: &'static str,
    /// Arc color.
    pub arete: &'static str,
    /// Arc label color.
    pub label: &'static str,
    /// Figure background.
    pub background: &'static str,
    /// Node background.
    pub background_node: &'static str,
    /// Terminal background.
    pub background_terminal: &'static str,
}

impl Theme {
    /// Default publication theme (serif, white background, dark ink).
    pub fn publication() -> Self {
        Theme {
            police: "serif",
            size_title: 16,
            size_text: 12,
            size_legend: 11,
            ink: "#1a1a1a",
            arete: "#444444",
            label: "#666666",
            background: "#ffffff",
            background_node: "#ffffff",
            background_terminal: "#f4f4f4",
        }
    }

    /// Diagram rendering palette for this theme.
    fn colors(&self) -> Colors {
        Colors {
            ink: self.ink,
            arete: self.arete,
            label: self.label,
            background_node: self.background_node,
            background_terminal: self.background_terminal,
        }
    }
}

/// Channel legend: single source of truth (channel, label, color),
/// rendered as-is in the auto-generated legend.
pub const LEGEND_CHANNELS: &[(ChannelAttr, &str, &str)] = &[
    (ChannelAttr::Inherited, "inherited (φ_disp)", "#1f6feb"),
    (ChannelAttr::Synthesized, "synthesized (φ_aggr)", "#d29922"),
    (ChannelAttr::Memory, "level memory", "#8250df"),
];

/// Height reserved for the legend in pixels (one line per channel).
const HEIGHT_LEGEND: u32 = 18;
/// Width of the legend column in pixels.
const WIDTH_LEGEND: u32 = 220;
/// Size of a channel color swatch in pixels.
const SIZE_TILE_LEGEND: u32 = 12;
/// Horizontal offset between the swatch and its label in pixels.
const SHIFT_LABEL_LEGEND: u32 = 18;
/// Vertical offset of the swatch relative to the text line in pixels.
const SHIFT_TILE_LEGEND: u32 = 10;
/// Margin between the diagram and the legend column in pixels.
const MARGIN_LEGEND: u32 = 20;
/// Title baseline in pixels.
const BASELINE_TITLE: u32 = 26;
/// Bottom margin of the figure in pixels.
const MARGIN_BAS: u32 = 20;

/// Renders the channel legend as an SVG fragment (one colored swatch + one label
/// per channel, auto-generated from [`LEGEND_CHANNELS`]).
pub fn render_legend_svg(theme: &Theme, x: u32, y: u32) -> String {
    let mut svg = String::new();
    for (index, (_channel, label, color)) in LEGEND_CHANNELS.iter().enumerate() {
        let line_y = y + index as u32 * HEIGHT_LEGEND;
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{}\" width=\"{SIZE_TILE_LEGEND}\" height=\"{SIZE_TILE_LEGEND}\" fill=\"{color}\"/>\n",
            line_y - SHIFT_TILE_LEGEND
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
            x + SHIFT_LABEL_LEGEND,
            line_y,
            theme.police,
            theme.size_legend,
            theme.ink,
            escape_xml(label)
        ));
    }
    svg
}

/// Renders the sub-diagram reachable from `root` as a **standalone
/// publication SVG**: white background, serif title, diagram in the given
/// theme, and auto-generated channel legend if `with_legend`.
pub fn render_svg_publication<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    title: &str,
    theme: &Theme,
    with_legend: bool,
) -> String {
    let (placement, number_levels) = placer(pool, root);
    let width_diagram =
        MARGIN_LEFT * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + HALF_WIDTH;
    let width_legend = if with_legend { WIDTH_LEGEND } else { 0 };
    let width = width_diagram + width_legend;
    let height_legend = LEGEND_CHANNELS.len() as u32 * HEIGHT_LEGEND;
    let height = (MARGIN_HAUT + (number_levels + 1) * HEIGHT_LEVEL + MARGIN_BAS)
        .max(height_legend + MARGIN_HAUT);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"{}\"/>\n",
        theme.background
    ));
    svg.push_str(&format!(
        "<text x=\"{MARGIN_LEFT}\" y=\"{BASELINE_TITLE}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
        theme.police,
        theme.size_title,
        theme.ink,
        escape_xml(title)
    ));
    svg.push_str(&draw_body(pool, root, &placement, &theme.colors()));
    if with_legend {
        svg.push_str(&render_legend_svg(
            theme,
            width_diagram + MARGIN_LEGEND,
            MARGIN_HAUT + SHIFT_TILE_LEGEND,
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

/// Pixel → centimeter conversion scale for PGF (1 px = 0.05 cm).
const SCALE_PGF_CM_PER_PIXEL: f64 = 0.05;

/// Renders the sub-diagram reachable from `root` as a **standalone
/// PGF/TikZ** (document `standalone`), for inclusion in a LaTeX article.
pub fn render_pgf<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    title: &str,
) -> String {
    let (placement, _number_levels) = placer(pool, root);
    let mut pgf = String::new();
    pgf.push_str(
        "\\documentclass[tikz,border=5pt]{standalone}\n\\usepackage{tikz}\n\\begin{document}\n",
    );
    pgf.push_str("\\begin{tikzpicture}[every node/.style={font=\\ttfamily\\small}]\n");
    pgf.push_str(&format!(
        "\\node[font=\\rmfamily\\bfseries] at (0,0) {{{}}};\n",
        escape_pgf(title)
    ));
    // Nodes.
    for (node, position) in &placement {
        let x = position.x as f64 * SCALE_PGF_CM_PER_PIXEL;
        let y = -(position.y as f64 * SCALE_PGF_CM_PER_PIXEL);
        match pool.resolve(*node) {
            Ok(lisage_core::Form::Terminal(payload)) => {
                let label = if payload { "$\\top$" } else { "$\\bot$" };
                pgf.push_str(&format!(
                    "\\node[draw, rectangle, minimum width=1.2cm] (n{}) at ({x:.2},{y:.2}) {{{label}}};\n",
                    node.0
                ));
            }
            Ok(lisage_core::Form::Node { level, .. }) => {
                pgf.push_str(&format!(
                    "\\node[draw, rectangle, minimum width=1.2cm] (n{}) at ({x:.2},{y:.2}) {{$\\ell{}$}};\n",
                    node.0,
                    level.0
                ));
            }
            Err(_) => {}
        }
    }
    // Arcs.
    let mut stack = vec![root];
    let mut already_seen = std::collections::BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !already_seen.insert(node) {
            continue;
        }
        if let Ok(lisage_core::Form::Node { arcs, .. }) = pool.resolve(node) {
            for (key, target) in arcs {
                pgf.push_str(&format!(
                    "\\draw[->] (n{}) -- (n{}) node[midway, above, font=\\ttfamily\\scriptsize] {{{}}};\n",
                    node.0,
                    target.0,
                    escape_pgf(&key.to_string())
                ));
                stack.push(target);
            }
        }
    }
    pgf.push_str("\\end{tikzpicture}\n\\end{document}\n");
    pgf
}

/// Minimal escaping for TikZ (LaTeX special characters).
fn escape_pgf(text: &str) -> String {
    // The backslash goes through a placeholder: escaped first it would
    // poorly protect its own braces, escaped last it would re-escape
    // the backslashes inserted by the other rules.
    const PLACEHOLDER_BASCUL: char = '\u{0}';
    text.replace('\\', PLACEHOLDER_BASCUL.to_string().as_str())
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('$', "\\$")
        .replace('&', "\\&")
        .replace('#', "\\#")
        .replace('%', "\\%")
        .replace('_', "\\_")
        .replace('^', "\\textasciicircum{}")
        .replace(PLACEHOLDER_BASCUL, "\\textbackslash{}")
}
