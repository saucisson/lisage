//! Tests of M3-4 (view 1 of the Reader, reader doc § 4): controller card
//! (eight fields), order as a linear track or tree (vtree), kingdoms side by
//! side, arrowed conversions with cost — deterministic static SVG rendering.

use lisage_reader::view_order::{
    CardController, ConversionView, KingdomView, NodeVtree, TrackOrder, render_svg_view_order,
};

/// Extracts a numeric attribute from an SVG element (e.g. `y="42"` → 42).
fn extract_attribute(element: &str, attribute: &str) -> u32 {
    // Leading space: avoids matching the end of `font-family="..."`.
    let position_attribute = element
        .find(&format!(" {attribute}=\""))
        .unwrap_or_else(|| panic!("attribute {attribute} present"));
    let after = &element[position_attribute + attribute.len() + 3..];
    let end = after.find('"').expect("end of attribute");
    after[..end].parse().expect("integer attribute")
}

/// `y` coordinate of the text element carrying `marker` (e.g. `>record<`).
fn y_text(svg: &str, marker: &str) -> u32 {
    let position = svg.find(marker).expect("marker present");
    let before = &svg[..position];
    let y_pos = before
        .rfind(" y=\"")
        .expect("y attribute before the marker");
    extract_attribute(&before[y_pos..], "y")
}

/// `x` coordinate of the text element carrying `marker`.
fn x_text(svg: &str, marker: &str) -> u32 {
    let position = svg.find(marker).expect("marker present");
    let before = &svg[..position];
    let x_pos = before
        .rfind(" x=\"")
        .expect("x attribute before the marker");
    extract_attribute(&before[x_pos..], "x")
}

/// `y1` of the conversion arrow (dashed line).
fn y1_arrow_conversion(svg: &str) -> u32 {
    let pos = svg
        .find("stroke-dasharray=\"6 3\"")
        .expect("conversion arrow");
    let start = svg[..pos].rfind("<line").expect("line element");
    extract_attribute(&svg[start..pos], "y1")
}

/// Controller card of the reference instance (eight fields filled).
fn card_reference() -> CardController {
    CardController {
        delta: "presence {absent, present}".to_string(),
        sigma: "Set(Atom(D))".to_string(),
        decomposition_order: "universe sweep, canonical O".to_string(),
        regles: "R-share, R-collapse, R-default".to_string(),
        defauts: "δ(ℓ) = absent".to_string(),
        default_global: "D_δ = terminal(false)".to_string(),
        encoding: "universe sweep".to_string(),
        assignment: "inherited / synthesized / memory".to_string(),
    }
}

/// Card of the classical BDD witness.
fn card_bdd() -> CardController {
    CardController {
        encoding: "classical BDD".to_string(),
        ..card_reference()
    }
}

/// M3-4: the order view renders the controller card (eight fields), the
/// linear track of the levels in sweep order, and the kingdoms side by
/// side with the arrowed conversion and its cost.
#[test]
fn test_view_order_card_track_linear_kingdoms_and_conversion() {
    let kingdoms = vec![
        KingdomView {
            name: "reference".to_string(),
            card: card_reference(),
            track: TrackOrder::Linear(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "<a>".to_string(),
            ]),
        },
        KingdomView {
            name: "bdd-witness".to_string(),
            card: card_bdd(),
            track: TrackOrder::Linear(vec!["x0".to_string(), "x1".to_string()]),
        },
    ];
    let conversions = vec![ConversionView {
        source: "reference".to_string(),
        target: "bdd-witness".to_string(),
        cost: 3,
    }];
    let svg = render_svg_view_order(&kingdoms, &conversions);

    // Controller card: the eight fields.
    for field in [
        "δ =",
        "Σ =",
        "decomposition + O =",
        "R =",
        "defaults =",
        "D^def =",
        "encoding =",
        "assignment =",
    ] {
        assert!(svg.contains(field), "card field present: {field}");
    }
    // Two kingdoms side by side.
    assert!(svg.contains("reference"));
    assert!(svg.contains("bdd-witness"));
    // Linear track: the levels in order (increasing x).
    let x_a = x_text(&svg, ">a<");
    let x_b = x_text(&svg, ">b<");
    let x_c = x_text(&svg, ">c<");
    assert!(x_a < x_b && x_b < x_c, "levels in sweep order");
    // Arrowed conversion with cost.
    assert!(svg.contains("convert reference → bdd-witness (cost 3)"));
    assert!(svg.contains("stroke-dasharray=\"6 3\""), "dashed arrow");
    // XML escaping of labels: "<a>" rendered as &lt;a&gt;, never as a tag.
    assert!(svg.contains("&lt;a&gt;"), "label escaped as &lt;a&gt;");
    assert!(!svg.contains("><a><"), "no injected tag");
}

/// M3-4: a structured order renders as a tree (vtree), no special case.
#[test]
fn test_view_order_track_tree_vtree() {
    let tree = NodeVtree {
        libelle: "record".to_string(),
        children: vec![
            NodeVtree {
                libelle: "machine".to_string(),
                children: vec![
                    NodeVtree {
                        libelle: "cell".to_string(),
                        children: vec![],
                    },
                    NodeVtree {
                        libelle: "state".to_string(),
                        children: vec![],
                    },
                ],
            },
            NodeVtree {
                libelle: "conveyor".to_string(),
                children: vec![],
            },
        ],
    };
    let kingdoms = vec![KingdomView {
        name: "sdD".to_string(),
        card: card_reference(),
        track: TrackOrder::Tree(tree),
    }];
    let svg = render_svg_view_order(&kingdoms, &[]);
    for libelle in ["record", "machine", "cell", "state", "conveyor"] {
        assert!(
            svg.contains(&format!(">{libelle}<")),
            "vtree node: {libelle}"
        );
    }
    // The parent is above its children (parsed y, not offsets).
    let y_record = y_text(&svg, ">record<");
    let y_cell = y_text(&svg, ">cell<");
    assert!(y_record < y_cell, "parent above the child");
}

/// M3-4: the rendering is deterministic and a conversion toward an
/// unknown kingdom is ignored (no arrow).
#[test]
fn test_view_order_deterministic_and_conversion_unknown_ignored() {
    let kingdoms = vec![KingdomView {
        name: "r".to_string(),
        card: card_reference(),
        track: TrackOrder::Linear(vec!["n0".to_string()]),
    }];
    let conversions = vec![ConversionView {
        source: "r".to_string(),
        target: "unknown".to_string(),
        cost: 1,
    }];
    let svg = render_svg_view_order(&kingdoms, &conversions);
    assert!(!svg.contains("convert r → unknown"));
    let svg2 = render_svg_view_order(&kingdoms, &conversions);
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// M3-4: an arrowed conversion anchors at the middle of the track **whatever
/// its height** — including a tree track (vtree), not only a linear track.
#[test]
fn test_conversion_anchored_in_track_tree() {
    let tree = NodeVtree {
        libelle: "record".to_string(),
        children: vec![
            NodeVtree {
                libelle: "left".to_string(),
                children: vec![NodeVtree {
                    libelle: "leaf".to_string(),
                    children: vec![],
                }],
            },
            NodeVtree {
                libelle: "right".to_string(),
                children: vec![],
            },
        ],
    };
    let kingdoms = vec![
        KingdomView {
            name: "tree".to_string(),
            card: card_reference(),
            track: TrackOrder::Tree(tree),
        },
        KingdomView {
            name: "linear".to_string(),
            card: card_bdd(),
            track: TrackOrder::Linear(vec!["x0".to_string()]),
        },
    ];
    let conversions = vec![ConversionView {
        source: "tree".to_string(),
        target: "linear".to_string(),
        cost: 7,
    }];
    let svg = render_svg_view_order(&kingdoms, &conversions);
    assert!(svg.contains("convert tree → linear (cost 7)"));
    // The arrow starts from the middle of the tree track (3 levels of 44 px,
    // i.e. y_track + 66): parsed y1, strictly below the root and its first
    // level — an anchoring on the root (y_track + 28) would fail here.
    let y_root = y_text(&svg, ">record<");
    let y1 = y1_arrow_conversion(&svg);
    assert!(
        y1 > y_root + 44,
        "arrow anchored at the middle of the track (y1={y1}, root={y_root})"
    );
}
