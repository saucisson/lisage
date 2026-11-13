//! Reader — **view 1: the order and the controllers** (M3-4, reader doc § 4):
//! the controller card (the eight fields of `C`), the order `O` rendered as
//! a horizontal linear track (or tree / vtree), the kingdoms side by side,
//! and the arrowed conversions with their cost.
//!
//! The reader does not depend on `lisage-plan` (dependency boundary):
//! it renders a **neutral description** ([`KingdomView`], [`ConversionView`])
//! produced by the plan; it never interprets the labels.

use crate::svg::escape_xml;

/// Width of the view in pixels.
const WIDTH_VUE: u32 = 960;
/// Outer margin in pixels.
const MARGIN: u32 = 20;
/// Height of a controller card line in pixels.
const HEIGHT_LINE_FICHE: u32 = 16;
/// Height of a linear track in pixels.
const HEIGHT_PISTE: u32 = 56;
/// Height of a tree level (vtree) in pixels.
const HEIGHT_LEVEL_ARBRE: u32 = 44;
/// Width of a track box in pixels.
const WIDTH_BOX: u32 = 110;
/// Color of the track (level borders).
const COLOR_PISTE: &str = "#2980b9";
/// Color of the controller card.
const COLOR_FICHE: &str = "#555";
/// Color of the arrowed conversions.
const COLOR_CONVERSION: &str = "#e67e22";
/// Color of the text.
const COLOR_TEXTE: &str = "#222";
/// Background of the controller card.
const BACKGROUND_FICHE: &str = "#f7f7f7";
/// Background of the level boxes.
const BACKGROUND_BOX: &str = "#fff";
/// Title font size of the view.
const SIZE_TITRE: u32 = 14;
/// Font size of a kingdom name / card title.
const SIZE_NAME: u32 = 13;
/// Font size of a card field.
const SIZE_FIELD: u32 = 12;
/// Font size of a track / conversion label.
const SIZE_PISTE: u32 = 11;
/// Standard inner offset (margin inside a panel).
const OFFSET: u32 = 8;
/// Height of the view title banner.
const HEIGHT_TITRE: u32 = 30;
/// Space after a kingdom's track.
const ESPACE_APRES_PISTE: u32 = 16;
/// Vertical offset of the card title.
const DECALAGE_TITRE_FICHE: u32 = 16;
/// Vertical offset of the first field line.
const DECALAGE_LINE_FICHE: u32 = 24;
/// Vertical offset of the baseline of a field line.
const BASELINE_LINE_FICHE: u32 = 12;
/// Horizontal offset of a box in the track.
const DECALAGE_BOX: u32 = 4;
/// Text baseline in a level box.
const BASELINE_BOX: u32 = 18;
/// Vertical center of a level box.
const MIDDLE_BOX: u32 = 14;
/// Horizontal offset of the conversion arrow.
const DECALAGE_FLECHE: u32 = 40;
/// Horizontal offset of the conversion label.
const DECALAGE_ETIQUETTE: u32 = 48;
/// Vertical offset of the conversion label below the middle of the arrow.
const DECALAGE_ETIQUETTE_Y: u32 = 4;
/// Geometry of the arrow markers (size, refX, refY).
const MARQUEUR_SIZE: u32 = 8;
const MARQUEUR_REF_Y: u32 = 4;
/// Number of fields of the controller card (C = eight fields).
const NUMBER_FIELDS_FICHE: u32 = 8;
/// Height of a level box in pixels.
const HEIGHT_BOX: u32 = 28;
/// Stroke thickness in pixels.
const EPAISSEUR_TRAIT: u32 = 2;
/// Height of a conversion line in pixels.
const HEIGHT_LINE_CONVERSION: u32 = 30;
/// Dash pattern of the conversion arrows.
const DASH_CONVERSION: &str = "6 3";

/// Height of the controller card in pixels.
const fn height_card() -> u32 {
    DECALAGE_LINE_FICHE + NUMBER_FIELDS_FICHE * HEIGHT_LINE_FICHE
}

/// Controller card: the eight fields of `C = (δ, Σ, decomposition + O, R,
/// defaults, D^def, encoding, assignment)`, as neutral labels.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CardController {
    /// δ : algebra of keys and defaults per level.
    pub delta: String,
    /// Σ : type expression unfolded by the order.
    pub sigma: String,
    /// Decomposition + order O.
    pub decomposition_order: String,
    /// R : reduction rules.
    pub regles: String,
    /// Defaults designated per level.
    pub defauts: String,
    /// Global default D^def designated by the controller.
    pub default_global: String,
    /// Encoding (kingdom): universe sweep, enumeration…
    pub encoding: String,
    /// Assignment (inherited/synthesized/memory channels).
    pub assignment: String,
}

/// Node of an order tree (vtree): a label and its children.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NodeVtree {
    /// Label of the node (fragment of the decomposition).
    pub libelle: String,
    /// Children (subtrees).
    pub children: Vec<NodeVtree>,
}

/// Order track `O`: linear (levels in sweep order) or structured as a
/// tree (vtree).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TrackOrder {
    /// Linear O: level labels in order.
    Linear(Vec<String>),
    /// Structured O: tree (vtree).
    Tree(NodeVtree),
}

/// Controller kingdom: a card + an order track.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KingdomView {
    /// Name of the kingdom (controller).
    pub name: String,
    /// Controller card (eight fields).
    pub card: CardController,
    /// Order track.
    pub track: TrackOrder,
}

/// Arrowed conversion between two kingdoms, with its cost (event
/// `Convert`: source controller, target, cost).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConversionView {
    /// Name of the source kingdom.
    pub source: String,
    /// Name of the target kingdom.
    pub target: String,
    /// Measured/predicted cost of the conversion.
    pub cost: u64,
}

/// Height in pixels of a tree (vtree).
fn height_tree(node: &NodeVtree) -> u32 {
    let height_children = node.children.iter().map(height_tree).max().unwrap_or(0);
    HEIGHT_LEVEL_ARBRE + height_children
}

/// Height in pixels of a track.
fn height_track(track: &TrackOrder) -> u32 {
    match track {
        TrackOrder::Linear(_) => HEIGHT_PISTE,
        TrackOrder::Tree(root) => height_tree(root),
    }
}

/// Renders the controller card (eight fields in a panel); returns the
/// rendered height.
fn render_card(svg: &mut String, card: &CardController, x: u32, y: u32, width: u32) -> u32 {
    let fields = [
        ("δ", &card.delta),
        ("Σ", &card.sigma),
        ("decomposition + O", &card.decomposition_order),
        ("R", &card.regles),
        ("defaults", &card.defauts),
        ("D^def", &card.default_global),
        ("encoding", &card.encoding),
        ("assignment", &card.assignment),
    ];
    debug_assert_eq!(fields.len() as u32, NUMBER_FIELDS_FICHE);
    let height = height_card();
    svg.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"{BACKGROUND_FICHE}\" stroke=\"{COLOR_FICHE}\"/>\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_NAME}\" font-weight=\"bold\" fill=\"{COLOR_TEXTE}\">controller card</text>\n",
        x + OFFSET,
        y + DECALAGE_TITRE_FICHE
    ));
    for (index, (field, value)) in fields.iter().enumerate() {
        let line_y =
            y + DECALAGE_LINE_FICHE + index as u32 * HEIGHT_LINE_FICHE + BASELINE_LINE_FICHE;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{line_y}\" font-family=\"serif\" font-size=\"{SIZE_FIELD}\" fill=\"{COLOR_TEXTE}\">{field} = {}</text>\n",
            x + OFFSET,
            escape_xml(value)
        ));
    }
    height
}

/// Renders a linear track: the levels in sweep order, in boxes linked by
/// arrows.
fn render_track_linear(svg: &mut String, levels: &[String], x: u32, y: u32, width: u32) {
    let y_box = y + OFFSET;
    let number = levels.len().max(1) as u32;
    let not_ = width / number;
    for (index, libelle) in levels.iter().enumerate() {
        let box_x = x + index as u32 * not_ + DECALAGE_BOX;
        let box_width = not_.saturating_sub(OFFSET).min(WIDTH_BOX);
        svg.push_str(&format!(
            "<rect x=\"{box_x}\" y=\"{y_box}\" width=\"{box_width}\" height=\"{HEIGHT_BOX}\" fill=\"{BACKGROUND_BOX}\" stroke=\"{COLOR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_PISTE}\" text-anchor=\"middle\" fill=\"{COLOR_TEXTE}\">{}</text>\n",
            box_x + box_width / 2,
            y_box + BASELINE_BOX,
            escape_xml(libelle)
        ));
        if index + 1 < levels.len() {
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COLOR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\" marker-end=\"url(#arrow)\"/>\n",
                box_x + box_width,
                y_box + MIDDLE_BOX,
                x + (index as u32 + 1) * not_ + DECALAGE_BOX,
                y_box + MIDDLE_BOX
            ));
        }
    }
}

/// Renders a tree (vtree): the node centered, the children spread in width.
/// Returns the rendered height.
fn render_tree(svg: &mut String, node: &NodeVtree, x: u32, y: u32, width: u32) -> u32 {
    let center_x = x + width / 2;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{y}\" width=\"{WIDTH_BOX}\" height=\"{HEIGHT_BOX}\" fill=\"{BACKGROUND_BOX}\" stroke=\"{COLOR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\"/>\n",
        center_x - WIDTH_BOX / 2
    ));
    svg.push_str(&format!(
        "<text x=\"{center_x}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_PISTE}\" text-anchor=\"middle\" fill=\"{COLOR_TEXTE}\">{}</text>\n",
        y + BASELINE_BOX,
        escape_xml(&node.libelle)
    ));
    if node.children.is_empty() {
        return HEIGHT_LEVEL_ARBRE;
    }
    let number = node.children.len() as u32;
    let not_ = width / number;
    let y_children = y + HEIGHT_LEVEL_ARBRE;
    let mut height_max = 0;
    for (index, child) in node.children.iter().enumerate() {
        let child_x = x + index as u32 * not_;
        let child_center = child_x + not_ / 2;
        svg.push_str(&format!(
            "<line x1=\"{center_x}\" y1=\"{}\" x2=\"{child_center}\" y2=\"{y_children}\" stroke=\"{COLOR_PISTE}\"/>\n",
            y + HEIGHT_BOX
        ));
        let height = render_tree(svg, child, child_x, y_children, not_);
        height_max = height_max.max(height);
    }
    HEIGHT_LEVEL_ARBRE + height_max
}

/// Renders an order track (linear or tree); returns the rendered height.
fn render_track(svg: &mut String, track: &TrackOrder, x: u32, y: u32, width: u32) -> u32 {
    match track {
        TrackOrder::Linear(levels) => {
            render_track_linear(svg, levels, x, y, width);
            HEIGHT_PISTE
        }
        TrackOrder::Tree(root) => render_tree(svg, root, x, y, width),
    }
}

/// Position of a kingdom in the view (single source of truth of the layout,
/// used by the rendering: the two passes cannot diverge).
struct PositionKingdom {
    y_name: u32,
    y_card: u32,
    y_track: u32,
    height_track: u32,
}

/// Computes the full layout: total SVG height + position of each
/// kingdom (name, card, track).
fn layout(kingdoms: &[KingdomView], number_conversions: usize) -> (u32, Vec<PositionKingdom>) {
    let mut height = MARGIN + HEIGHT_TITRE;
    let mut positions = Vec::new();
    for kingdom in kingdoms {
        let height_track = height_track(&kingdom.track);
        positions.push(PositionKingdom {
            y_name: height,
            y_card: height + OFFSET,
            y_track: height + OFFSET + height_card() + OFFSET,
            height_track,
        });
        height += OFFSET + height_card() + OFFSET + height_track + ESPACE_APRES_PISTE;
    }
    height += number_conversions as u32 * HEIGHT_LINE_CONVERSION + MARGIN;
    (height, positions)
}

/// Renders the order view as static SVG: for each kingdom, its controller
/// card then its order track; the arrowed conversions with their cost link
/// the tracks of the source/target kingdoms (anchored at the middle of the
/// track, whatever its height).
pub fn render_svg_view_order(kingdoms: &[KingdomView], conversions: &[ConversionView]) -> String {
    let width = WIDTH_VUE - 2 * MARGIN;
    let (height, positions) = layout(kingdoms, conversions.len());
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH_VUE}\" height=\"{height}\" viewBox=\"0 0 {WIDTH_VUE} {height}\">\n<defs><marker id=\"arrow\" markerWidth=\"{MARQUEUR_SIZE}\" markerHeight=\"{MARQUEUR_SIZE}\" refX=\"{MARQUEUR_SIZE}\" refY=\"{MARQUEUR_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARQUEUR_SIZE},{MARQUEUR_REF_Y} L0,{MARQUEUR_SIZE} z\" fill=\"{COLOR_PISTE}\"/></marker><marker id=\"arrow-conversion\" markerWidth=\"{MARQUEUR_SIZE}\" markerHeight=\"{MARQUEUR_SIZE}\" refX=\"{MARQUEUR_SIZE}\" refY=\"{MARQUEUR_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARQUEUR_SIZE},{MARQUEUR_REF_Y} L0,{MARQUEUR_SIZE} z\" fill=\"{COLOR_CONVERSION}\"/></marker></defs>\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGIN}\" y=\"{DECALAGE_TITRE_FICHE}\" font-family=\"serif\" font-size=\"{SIZE_TITRE}\" fill=\"{COLOR_TEXTE}\">order view — {} kingdom(s), {} conversion(s)</text>\n",
        kingdoms.len(),
        conversions.len()
    ));
    for (kingdom, position) in kingdoms.iter().zip(&positions) {
        svg.push_str(&format!(
            "<text x=\"{MARGIN}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_NAME}\" font-weight=\"bold\" fill=\"{COLOR_TEXTE}\">{}</text>\n",
            position.y_name,
            escape_xml(&kingdom.name)
        ));
        render_card(&mut svg, &kingdom.card, MARGIN, position.y_card, width);
        render_track(&mut svg, &kingdom.track, MARGIN, position.y_track, width);
    }
    // Arrowed conversions between tracks, with the cost.
    for conversion in conversions {
        let Some(index_source) = kingdoms.iter().position(|r| r.name == conversion.source) else {
            continue;
        };
        let Some(index_target) = kingdoms.iter().position(|r| r.name == conversion.target) else {
            continue;
        };
        let y1 = positions[index_source].y_track + positions[index_source].height_track / 2;
        let y2 = positions[index_target].y_track + positions[index_target].height_track / 2;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{y1}\" x2=\"{}\" y2=\"{y2}\" stroke=\"{COLOR_CONVERSION}\" stroke-width=\"{EPAISSEUR_TRAIT}\" stroke-dasharray=\"{DASH_CONVERSION}\" marker-end=\"url(#arrow-conversion)\"/>\n",
            WIDTH_VUE - MARGIN - DECALAGE_FLECHE,
            WIDTH_VUE - MARGIN
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{SIZE_PISTE}\" text-anchor=\"end\" fill=\"{COLOR_CONVERSION}\">convert {} → {} (cost {})</text>\n",
            WIDTH_VUE - MARGIN - DECALAGE_ETIQUETTE,
            (y1 + y2) / 2 + DECALAGE_ETIQUETTE_Y,
            escape_xml(&conversion.source),
            escape_xml(&conversion.target),
            conversion.cost
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout is consistent: each track is placed after its card, and
    /// the total height covers all tracks (no clipping).
    #[test]
    fn test_layout_consistent() {
        let kingdoms = vec![
            KingdomView {
                name: "a".to_string(),
                card: CardController {
                    delta: String::new(),
                    sigma: String::new(),
                    decomposition_order: String::new(),
                    regles: String::new(),
                    defauts: String::new(),
                    default_global: String::new(),
                    encoding: String::new(),
                    assignment: String::new(),
                },
                track: TrackOrder::Linear(vec!["x".to_string()]),
            },
            KingdomView {
                name: "b".to_string(),
                card: CardController {
                    delta: String::new(),
                    sigma: String::new(),
                    decomposition_order: String::new(),
                    regles: String::new(),
                    defauts: String::new(),
                    default_global: String::new(),
                    encoding: String::new(),
                    assignment: String::new(),
                },
                track: TrackOrder::Tree(NodeVtree {
                    libelle: "r".to_string(),
                    children: vec![NodeVtree {
                        libelle: "e".to_string(),
                        children: vec![],
                    }],
                }),
            },
        ];
        let (height, positions) = layout(&kingdoms, 1);
        assert_eq!(positions.len(), 2);
        for position in &positions {
            assert_eq!(position.y_card, position.y_name + OFFSET);
            assert_eq!(
                position.y_track,
                position.y_card + height_card() + OFFSET,
                "track placed after the card"
            );
            assert!(
                position.y_track + position.height_track <= height,
                "track covered by the total height"
            );
        }
        assert!(positions[1].y_track > positions[0].y_track);
    }
}
