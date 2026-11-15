//! Publication mode (M5-5): export of figures ready for article
//! layout — standalone SVG or PGF/TikZ, serif theme, **legend
//! auto-generated from the channel legend** (`CanalAttr`): the same
//! file used for debugging is used for publication (liseuse doc § 5).

use lisage_core::observer::CanalAttr;
use lisage_core::{NodeId, Pool};

use crate::svg::{
    Couleurs, DEMI_LARGEUR, HAUTEUR_LEVEL, MARGE_GAUCHE, MARGE_HAUT, dessiner_corps, echapper_xml,
    placer,
};

/// Publication theme: serif, dark ink on white background, channel colors
/// aligned with the `CanalAttr` legend (inherited = blue, synthesized =
/// orange, memory = purple).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Theme {
    /// Font (serif for article layout).
    pub police: &'static str,
    /// Title size in pixels.
    pub size_titre: u32,
    /// Node text size in pixels.
    pub size_texte: u32,
    /// Legend size in pixels.
    pub size_legende: u32,
    /// Ink color (outlines, text).
    pub encre: &'static str,
    /// Arc color.
    pub arete: &'static str,
    /// Arc label color.
    pub label: &'static str,
    /// Figure background.
    pub fond: &'static str,
    /// Node background.
    pub fond_node: &'static str,
    /// Terminal background.
    pub fond_terminal: &'static str,
}

impl Theme {
    /// Default publication theme (serif, white background, dark ink).
    pub fn publication() -> Self {
        Theme {
            police: "serif",
            size_titre: 16,
            size_texte: 12,
            size_legende: 11,
            encre: "#1a1a1a",
            arete: "#444444",
            label: "#666666",
            fond: "#ffffff",
            fond_node: "#ffffff",
            fond_terminal: "#f4f4f4",
        }
    }

    /// Diagram rendering palette for this theme.
    fn couleurs(&self) -> Couleurs {
        Couleurs {
            encre: self.encre,
            arete: self.arete,
            label: self.label,
            fond_node: self.fond_node,
            fond_terminal: self.fond_terminal,
        }
    }
}

/// Channel legend: single source of truth (channel, label, color),
/// rendered as-is in the auto-generated legend.
pub const LEGENDE_CANAUX: &[(CanalAttr, &str, &str)] = &[
    (CanalAttr::Inherited, "inherited (φ_disp)", "#1f6feb"),
    (CanalAttr::Synthesized, "synthesized (φ_aggr)", "#d29922"),
    (CanalAttr::Memory, "level memory", "#8250df"),
];

/// Height reserved for the legend in pixels (one line per channel).
const HAUTEUR_LEGENDE: u32 = 18;
/// Width of the legend column in pixels.
const LARGEUR_LEGENDE: u32 = 220;
/// Size of a channel color swatch in pixels.
const SIZE_PAVE_LEGENDE: u32 = 12;
/// Horizontal offset between the swatch and its label in pixels.
const DECALAGE_LABEL_LEGENDE: u32 = 18;
/// Vertical offset of the swatch relative to the text line in pixels.
const DECALAGE_PAVE_LEGENDE: u32 = 10;
/// Margin between the diagram and the legend column in pixels.
const MARGE_LEGENDE: u32 = 20;
/// Title baseline in pixels.
const BASELINE_TITRE: u32 = 26;
/// Bottom margin of the figure in pixels.
const MARGE_BAS: u32 = 20;

/// Renders the channel legend as an SVG fragment (one colored swatch + one label
/// per channel, auto-generated from [`LEGENDE_CANAUX`]).
pub fn rendre_legende_svg(theme: &Theme, x: u32, y: u32) -> String {
    let mut svg = String::new();
    for (index, (_canal, label, couleur)) in LEGENDE_CANAUX.iter().enumerate() {
        let ligne_y = y + index as u32 * HAUTEUR_LEGENDE;
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{}\" width=\"{SIZE_PAVE_LEGENDE}\" height=\"{SIZE_PAVE_LEGENDE}\" fill=\"{couleur}\"/>\n",
            ligne_y - DECALAGE_PAVE_LEGENDE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
            x + DECALAGE_LABEL_LEGENDE,
            ligne_y,
            theme.police,
            theme.size_legende,
            theme.encre,
            echapper_xml(label)
        ));
    }
    svg
}

/// Renders the sub-diagram reachable from `root` as a **standalone
/// publication SVG**: white background, serif title, diagram in the given
/// theme, and auto-generated channel legend if `with_legende`.
pub fn rendre_svg_publication<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    titre: &str,
    theme: &Theme,
    with_legende: bool,
) -> String {
    let (placement, number_levels) = placer(pool, root);
    let largeur_diagramme =
        MARGE_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_LARGEUR;
    let largeur_legende = if with_legende { LARGEUR_LEGENDE } else { 0 };
    let largeur = largeur_diagramme + largeur_legende;
    let hauteur_legende = LEGENDE_CANAUX.len() as u32 * HAUTEUR_LEGENDE;
    let hauteur = (MARGE_HAUT + (number_levels + 1) * HAUTEUR_LEVEL + MARGE_BAS)
        .max(hauteur_legende + MARGE_HAUT);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{largeur}\" height=\"{hauteur}\" fill=\"{}\"/>\n",
        theme.fond
    ));
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{BASELINE_TITRE}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
        theme.police,
        theme.size_titre,
        theme.encre,
        echapper_xml(titre)
    ));
    svg.push_str(&dessiner_corps(pool, root, &placement, &theme.couleurs()));
    if with_legende {
        svg.push_str(&rendre_legende_svg(
            theme,
            largeur_diagramme + MARGE_LEGENDE,
            MARGE_HAUT + DECALAGE_PAVE_LEGENDE,
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

/// Pixel → centimeter conversion scale for PGF (1 px = 0.05 cm).
const ECHELLE_PGF_CM_PAR_PIXEL: f64 = 0.05;

/// Renders the sub-diagram reachable from `root` as a **standalone
/// PGF/TikZ** (document `standalone`), for inclusion in a LaTeX article.
pub fn rendre_pgf<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    titre: &str,
) -> String {
    let (placement, _number_levels) = placer(pool, root);
    let mut pgf = String::new();
    pgf.push_str(
        "\\documentclass[tikz,border=5pt]{standalone}\n\\usepackage{tikz}\n\\begin{document}\n",
    );
    pgf.push_str("\\begin{tikzpicture}[every node/.style={font=\\ttfamily\\small}]\n");
    pgf.push_str(&format!(
        "\\node[font=\\rmfamily\\bfseries] at (0,0) {{{}}};\n",
        echapper_pgf(titre)
    ));
    // Nodes.
    for (node, position) in &placement {
        let x = position.x as f64 * ECHELLE_PGF_CM_PAR_PIXEL;
        let y = -(position.y as f64 * ECHELLE_PGF_CM_PAR_PIXEL);
        match pool.resolve(*node) {
            Ok(lisage_core::Form::Terminal(payload)) => {
                let label = if *payload { "$\\top$" } else { "$\\bot$" };
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
    let mut pile = vec![root];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(node) = pile.pop() {
        if !deja_vu.insert(node) {
            continue;
        }
        if let Ok(lisage_core::Form::Node { arcs, .. }) = pool.resolve(node) {
            for (key, cible) in arcs {
                pgf.push_str(&format!(
                    "\\draw[->] (n{}) -- (n{}) node[midway, above, font=\\ttfamily\\scriptsize] {{{}}};\n",
                    node.0,
                    cible.0,
                    echapper_pgf(&key.to_string())
                ));
                pile.push(*cible);
            }
        }
    }
    pgf.push_str("\\end{tikzpicture}\n\\end{document}\n");
    pgf
}

/// Minimal escaping for TikZ (LaTeX special characters).
fn echapper_pgf(texte: &str) -> String {
    // The backslash goes through a placeholder: escaped first it would
    // poorly protect its own braces, escaped last it would re-escape
    // the backslashes inserted by the other rules.
    const PLACEHOLDER_BASCUL: char = '\u{0}';
    texte
        .replace('\\', PLACEHOLDER_BASCUL.to_string().as_str())
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
