//! Reader — **view 1: the order and the controllers** (M3-4, reader doc § 4):
//! the controller card (the eight fields of `C`), the order `O` rendered as
//! a horizontal linear track (or tree / vtree), the kingdoms side by side,
//! and the arrowed conversions with their cost.
//!
//! The reader does not depend on `lisage-plan` (dependency boundary):
//! it renders a **neutral description** ([`RoyaumeView`], [`ConversionView`])
//! produced by the planner; it never interprets the labels.

use crate::svg::echapper_xml;

/// Width of the view in pixels.
const WIDTH_VIEW: u32 = 960;
/// Outer margin in pixels.
const MARGE: u32 = 20;
/// Height of a controller-card line in pixels.
const HEIGHT_LIGNE_FICHE: u32 = 16;
/// Height of a linear track in pixels.
const HEIGHT_PISTE: u32 = 56;
/// Height of a tree level (vtree) in pixels.
const HEIGHT_LEVEL_ARBRE: u32 = 44;
/// Width of a track box in pixels.
const WIDTH_BOITE: u32 = 110;
/// Track color (level borders).
const COULEUR_PISTE: &str = "#2980b9";
/// Controller-card color.
const COULEUR_FICHE: &str = "#555";
/// Arrowed-conversion color.
const COULEUR_CONVERSION: &str = "#e67e22";
/// Text color.
const COULEUR_TEXTE: &str = "#222";
/// Controller-card background.
const FOND_FICHE: &str = "#f7f7f7";
/// Level-box background.
const FOND_BOITE: &str = "#fff";
/// Font size of the view title.
const SIZE_TITRE: u32 = 14;
/// Font size of a kingdom name / card title.
const SIZE_NOM: u32 = 13;
/// Font size of a card field.
const SIZE_CHAMP: u32 = 12;
/// Font size of a track / conversion label.
const SIZE_PISTE: u32 = 11;
/// Standard inner offset (margin inside a panel).
const DECALAGE: u32 = 8;
/// Height of the view title band.
const HEIGHT_TITRE: u32 = 30;
/// Space after a kingdom's track.
const SPACE_APRES_PISTE: u32 = 16;
/// Vertical offset of the card title.
const DECALAGE_TITRE_FICHE: u32 = 16;
/// Vertical offset of the first field line.
const DECALAGE_LIGNE_FICHE: u32 = 24;
/// Vertical offset of a field line's baseline.
const BASELINE_LIGNE_FICHE: u32 = 12;
/// Horizontal offset of a box in the track.
const DECALAGE_BOITE: u32 = 4;
/// Text baseline in a level box.
const BASELINE_BOITE: u32 = 18;
/// Vertical center of a level box.
const MILIEU_BOITE: u32 = 14;
/// Horizontal offset of the conversion arrow.
const DECALAGE_FLECHE: u32 = 40;
/// Horizontal offset of the conversion label.
const DECALAGE_LABEL: u32 = 48;
/// Vertical offset of the conversion label below the arrow's middle.
const DECALAGE_LABEL_Y: u32 = 4;
/// Arrow-marker geometry (size, refX, refY).
const MARKER_SIZE: u32 = 8;
const MARKER_REF_Y: u32 = 4;
/// Number of controller-card fields (C = eight fields).
const NUMBER_CHAMPS_FICHE: u32 = 8;
/// Height of a level box in pixels.
const HEIGHT_BOITE: u32 = 28;
/// Stroke thickness in pixels.
const EPAISSEUR_TRAIT: u32 = 2;
/// Height of a conversion line in pixels.
const HEIGHT_LIGNE_CONVERSION: u32 = 30;
/// Dashes of the conversion arrows.
const DASH_CONVERSION: &str = "6 3";

/// Height of the controller card in pixels.
const fn height_fiche() -> u32 {
    DECALAGE_LIGNE_FICHE + NUMBER_CHAMPS_FICHE * HEIGHT_LIGNE_FICHE
}

/// Controller card: the eight fields of `C = (δ, Σ, decomposition + O, R,
/// defaults, D^def, encoding, attribution)`, in neutral labels.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FicheControleur {
    /// δ: key algebra and per-level defaults.
    pub delta: String,
    /// Σ: type expression unfolded by the order.
    pub sigma: String,
    /// Decomposition + order O.
    pub decomposition_order: String,
    /// R: reduction rules.
    pub regles: String,
    /// Defaults designated per level.
    pub defauts: String,
    /// Global default D^def designated by the controller.
    pub defaut_global: String,
    /// Encoding (kingdom): universe sweep, enumeration…
    pub encodage: String,
    /// Attribution (inherited/synthesized/memory channels).
    pub attribution: String,
}

/// Node of an order tree (vtree): a label and its children.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NodeVtree {
    /// Node label (fragment of the decomposition).
    pub libelle: String,
    /// Children (subtrees).
    pub children: Vec<NodeVtree>,
}

/// Order track `O`: linear (levels in sweep order) or
/// structured as a tree (vtree).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PisteOrder {
    /// Linear O: level labels in order.
    Lineaire(Vec<String>),
    /// Structured O: tree (vtree).
    Arbre(NodeVtree),
}

/// Controller kingdom: a card + an order track.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RoyaumeView {
    /// Kingdom name (controller).
    pub nom: String,
    /// Controller card (eight fields).
    pub fiche: FicheControleur,
    /// Order track.
    pub piste: PisteOrder,
}

/// Arrowed conversion between two kingdoms, with its cost (event
/// `Convert`: source controller, target, cost).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConversionView {
    /// Source kingdom name.
    pub source: String,
    /// Target kingdom name.
    pub target: String,
    /// Measured/predicted cost of the conversion.
    pub cout: u64,
}

/// Height in pixels of a tree (vtree).
fn height_arbre(node: &NodeVtree) -> u32 {
    let height_children = node.children.iter().map(height_arbre).max().unwrap_or(0);
    HEIGHT_LEVEL_ARBRE + height_children
}

/// Height in pixels of a track.
fn height_piste(piste: &PisteOrder) -> u32 {
    match piste {
        PisteOrder::Lineaire(_) => HEIGHT_PISTE,
        PisteOrder::Arbre(root) => height_arbre(root),
    }
}

/// Renders the controller card (eight fields in a panel); returns the
/// rendered height.
fn rendre_fiche(svg: &mut String, fiche: &FicheControleur, x: u32, y: u32, width: u32) -> u32 {
    let champs = [
        ("δ", &fiche.delta),
        ("Σ", &fiche.sigma),
        ("decomposition + O", &fiche.decomposition_order),
        ("R", &fiche.regles),
        ("defaults", &fiche.defauts),
        ("D^def", &fiche.defaut_global),
        ("encoding", &fiche.encodage),
        ("attribution", &fiche.attribution),
    ];
    debug_assert_eq!(champs.len() as u32, NUMBER_CHAMPS_FICHE);
    let height = height_fiche();
    svg.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"{FOND_FICHE}\" stroke=\"{COULEUR_FICHE}\"/>\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_NOM}\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">controller card</text>\n",
        x + DECALAGE,
        y + DECALAGE_TITRE_FICHE
    ));
    for (index, (champ, value)) in champs.iter().enumerate() {
        let ligne_y =
            y + DECALAGE_LIGNE_FICHE + index as u32 * HEIGHT_LIGNE_FICHE + BASELINE_LIGNE_FICHE;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{ligne_y}\" font-family=\"serif\" font-size=\"{SIZE_CHAMP}\" fill=\"{COULEUR_TEXTE}\">{champ} = {}</text>\n",
            x + DECALAGE,
            echapper_xml(value)
        ));
    }
    height
}

/// Renders a linear track: the levels in sweep order, in
/// boxes linked by arrows.
fn rendre_piste_lineaire(svg: &mut String, levels: &[String], x: u32, y: u32, width: u32) {
    let y_boite = y + DECALAGE;
    let number = levels.len().max(1) as u32;
    let step = width / number;
    for (index, libelle) in levels.iter().enumerate() {
        let boite_x = x + index as u32 * step + DECALAGE_BOITE;
        let boite_width = step.saturating_sub(DECALAGE).min(WIDTH_BOITE);
        svg.push_str(&format!(
            "<rect x=\"{boite_x}\" y=\"{y_boite}\" width=\"{boite_width}\" height=\"{HEIGHT_BOITE}\" fill=\"{FOND_BOITE}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_PISTE}\" text-anchor=\"middle\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
            boite_x + boite_width / 2,
            y_boite + BASELINE_BOITE,
            echapper_xml(libelle)
        ));
        if index + 1 < levels.len() {
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\" marker-end=\"url(#fleche)\"/>\n",
                boite_x + boite_width,
                y_boite + MILIEU_BOITE,
                x + (index as u32 + 1) * step + DECALAGE_BOITE,
                y_boite + MILIEU_BOITE
            ));
        }
    }
}

/// Renders a tree (vtree): the node centered, the children spread in width.
/// Returns the rendered height.
fn rendre_arbre(svg: &mut String, node: &NodeVtree, x: u32, y: u32, width: u32) -> u32 {
    let centre_x = x + width / 2;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{y}\" width=\"{WIDTH_BOITE}\" height=\"{HEIGHT_BOITE}\" fill=\"{FOND_BOITE}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\"/>\n",
        centre_x - WIDTH_BOITE / 2
    ));
    svg.push_str(&format!(
        "<text x=\"{centre_x}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_PISTE}\" text-anchor=\"middle\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
        y + BASELINE_BOITE,
        echapper_xml(&node.libelle)
    ));
    if node.children.is_empty() {
        return HEIGHT_LEVEL_ARBRE;
    }
    let number = node.children.len() as u32;
    let step = width / number;
    let y_children = y + HEIGHT_LEVEL_ARBRE;
    let mut height_max = 0;
    for (index, child) in node.children.iter().enumerate() {
        let child_x = x + index as u32 * step;
        let child_centre = child_x + step / 2;
        svg.push_str(&format!(
            "<line x1=\"{centre_x}\" y1=\"{}\" x2=\"{child_centre}\" y2=\"{y_children}\" stroke=\"{COULEUR_PISTE}\"/>\n",
            y + HEIGHT_BOITE
        ));
        let height = rendre_arbre(svg, child, child_x, y_children, step);
        height_max = height_max.max(height);
    }
    HEIGHT_LEVEL_ARBRE + height_max
}

/// Renders an order track (linear or tree); returns the rendered height.
fn rendre_piste(svg: &mut String, piste: &PisteOrder, x: u32, y: u32, width: u32) -> u32 {
    match piste {
        PisteOrder::Lineaire(levels) => {
            rendre_piste_lineaire(svg, levels, x, y, width);
            HEIGHT_PISTE
        }
        PisteOrder::Arbre(root) => rendre_arbre(svg, root, x, y, width),
    }
}

/// Position of a kingdom in the view (single source of truth of the layout,
/// used by the rendering: the two passes cannot diverge).
struct PositionRoyaume {
    y_nom: u32,
    y_fiche: u32,
    y_piste: u32,
    height_piste: u32,
}

/// Computes the full layout: total SVG height + position of each
/// kingdom (name, card, track).
fn layout(royaumes: &[RoyaumeView], number_conversions: usize) -> (u32, Vec<PositionRoyaume>) {
    let mut height = MARGE + HEIGHT_TITRE;
    let mut positions = Vec::new();
    for royaume in royaumes {
        let height_piste = height_piste(&royaume.piste);
        positions.push(PositionRoyaume {
            y_nom: height,
            y_fiche: height + DECALAGE,
            y_piste: height + DECALAGE + height_fiche() + DECALAGE,
            height_piste,
        });
        height += DECALAGE + height_fiche() + DECALAGE + height_piste + SPACE_APRES_PISTE;
    }
    height += number_conversions as u32 * HEIGHT_LIGNE_CONVERSION + MARGE;
    (height, positions)
}

/// Renders the order view as static SVG: for each kingdom, its
/// controller card then its order track; the arrowed conversions with their
/// cost link the tracks of the source/target kingdoms (anchored at the middle
/// of the track, whatever its height).
pub fn rendre_svg_view_order(royaumes: &[RoyaumeView], conversions: &[ConversionView]) -> String {
    let width = WIDTH_VIEW - 2 * MARGE;
    let (height, positions) = layout(royaumes, conversions.len());
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH_VIEW}\" height=\"{height}\" viewBox=\"0 0 {WIDTH_VIEW} {height}\">\n<defs><marker id=\"fleche\" markerWidth=\"{MARKER_SIZE}\" markerHeight=\"{MARKER_SIZE}\" refX=\"{MARKER_SIZE}\" refY=\"{MARKER_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARKER_SIZE},{MARKER_REF_Y} L0,{MARKER_SIZE} z\" fill=\"{COULEUR_PISTE}\"/></marker><marker id=\"fleche-conversion\" markerWidth=\"{MARKER_SIZE}\" markerHeight=\"{MARKER_SIZE}\" refX=\"{MARKER_SIZE}\" refY=\"{MARKER_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARKER_SIZE},{MARKER_REF_Y} L0,{MARKER_SIZE} z\" fill=\"{COULEUR_CONVERSION}\"/></marker></defs>\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE}\" y=\"{DECALAGE_TITRE_FICHE}\" font-family=\"serif\" font-size=\"{SIZE_TITRE}\" fill=\"{COULEUR_TEXTE}\">order view — {} kingdom(s), {} conversion(s)</text>\n",
        royaumes.len(),
        conversions.len()
    ));
    for (royaume, position) in royaumes.iter().zip(&positions) {
        svg.push_str(&format!(
            "<text x=\"{MARGE}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_NOM}\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
            position.y_nom,
            echapper_xml(&royaume.nom)
        ));
        rendre_fiche(&mut svg, &royaume.fiche, MARGE, position.y_fiche, width);
        rendre_piste(&mut svg, &royaume.piste, MARGE, position.y_piste, width);
    }
    // Arrowed conversions between tracks, with the cost.
    for conversion in conversions {
        let Some(index_source) = royaumes.iter().position(|r| r.nom == conversion.source) else {
            continue;
        };
        let Some(index_target) = royaumes.iter().position(|r| r.nom == conversion.target) else {
            continue;
        };
        let y1 = positions[index_source].y_piste + positions[index_source].height_piste / 2;
        let y2 = positions[index_target].y_piste + positions[index_target].height_piste / 2;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{y1}\" x2=\"{}\" y2=\"{y2}\" stroke=\"{COULEUR_CONVERSION}\" stroke-width=\"{EPAISSEUR_TRAIT}\" stroke-dasharray=\"{DASH_CONVERSION}\" marker-end=\"url(#fleche-conversion)\"/>\n",
            WIDTH_VIEW - MARGE - DECALAGE_FLECHE,
            WIDTH_VIEW - MARGE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_PISTE}\" text-anchor=\"end\" fill=\"{COULEUR_CONVERSION}\">convert {} → {} (cost {})</text>\n",
            WIDTH_VIEW - MARGE - DECALAGE_LABEL,
            (y1 + y2) / 2 + DECALAGE_LABEL_Y,
            echapper_xml(&conversion.source),
            echapper_xml(&conversion.target),
            conversion.cout
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout is consistent: each track is placed after its card, and
    /// the total height covers all the tracks (no clipping).
    #[test]
    fn test_layout_coherent() {
        let royaumes = vec![
            RoyaumeView {
                nom: "a".to_string(),
                fiche: FicheControleur {
                    delta: String::new(),
                    sigma: String::new(),
                    decomposition_order: String::new(),
                    regles: String::new(),
                    defauts: String::new(),
                    defaut_global: String::new(),
                    encodage: String::new(),
                    attribution: String::new(),
                },
                piste: PisteOrder::Lineaire(vec!["x".to_string()]),
            },
            RoyaumeView {
                nom: "b".to_string(),
                fiche: FicheControleur {
                    delta: String::new(),
                    sigma: String::new(),
                    decomposition_order: String::new(),
                    regles: String::new(),
                    defauts: String::new(),
                    defaut_global: String::new(),
                    encodage: String::new(),
                    attribution: String::new(),
                },
                piste: PisteOrder::Arbre(NodeVtree {
                    libelle: "r".to_string(),
                    children: vec![NodeVtree {
                        libelle: "e".to_string(),
                        children: vec![],
                    }],
                }),
            },
        ];
        let (height, positions) = layout(&royaumes, 1);
        assert_eq!(positions.len(), 2);
        for position in &positions {
            assert_eq!(position.y_fiche, position.y_nom + DECALAGE);
            assert_eq!(
                position.y_piste,
                position.y_fiche + height_fiche() + DECALAGE,
                "track placed after the card"
            );
            assert!(
                position.y_piste + position.height_piste <= height,
                "track covered by the total height"
            );
        }
        assert!(positions[1].y_piste > positions[0].y_piste);
    }
}
