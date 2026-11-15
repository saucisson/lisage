//! M3-4 tests (Liseuse view 1, liseuse doc § 4): controller card
//! (eight fields), order as a linear track or a tree (vtree), kingdoms side by
//! side, arrowed conversions with cost — deterministic static SVG rendering.

use lisage_liseuse::view_order::{
    CardControleur, ConversionView, KingdomView, NodeVtree, TrackOrder, rendre_svg_view_order,
};

/// Extracts a numeric attribute from an SVG element (ex. `y="42"` → 42).
fn extraire_attribute(element: &str, attribute: &str) -> u32 {
    // Leading space: avoids matching the end of `font-family="..."`.
    let position_attribute = element
        .find(&format!(" {attribute}=\""))
        .unwrap_or_else(|| panic!("attribute {attribute} present"));
    let apres = &element[position_attribute + attribute.len() + 3..];
    let fin = apres.find('"').expect("end of the attribute");
    apres[..fin].parse().expect("integer attribute")
}

/// `y` coordinate of the text element carrying `marker` (e.g. `>record<`).
fn y_texte(svg: &str, marker: &str) -> u32 {
    let position = svg.find(marker).expect("marker present");
    let avant = &svg[..position];
    let y_pos = avant.rfind(" y=\"").expect("attribute y before the marker");
    extraire_attribute(&avant[y_pos..], "y")
}

/// `x` coordinate of the text element carrying `marker`.
fn x_texte(svg: &str, marker: &str) -> u32 {
    let position = svg.find(marker).expect("marker present");
    let avant = &svg[..position];
    let x_pos = avant.rfind(" x=\"").expect("attribute x before the marker");
    extraire_attribute(&avant[x_pos..], "x")
}

/// `y1` of the conversion arrow (dashed line).
fn y1_fleche_conversion(svg: &str) -> u32 {
    let pos = svg
        .find("stroke-dasharray=\"6 3\"")
        .expect("conversion arrow");
    let debut = svg[..pos].rfind("<line").expect("line element");
    extraire_attribute(&svg[debut..pos], "y1")
}

/// Controller card of the reference instance (eight fields filled).
fn card_reference() -> CardControleur {
    CardControleur {
        delta: "presence {absent, present}".to_string(),
        sigma: "Set(Atom(D))".to_string(),
        decomposition_order: "universe sweep, canonical O".to_string(),
        regles: "R-share, R-collapse, R-default".to_string(),
        defauts: "δ(ℓ) = absent".to_string(),
        defaut_global: "D_δ = terminal(false)".to_string(),
        encodage: "universe sweep".to_string(),
        attribution: "inherited / synthesized / memory".to_string(),
    }
}

/// Card of the classic BDD witness.
fn card_bdd() -> CardControleur {
    CardControleur {
        encodage: "classic BDD".to_string(),
        ..card_reference()
    }
}

/// M3-4: the order view renders the controller card (eight fields), the
/// linear track of the levels in sweep order, and the kingdoms side by
/// side with the arrowed conversion and its cost.
#[test]
fn test_view_order_card_track_lineaire_kingdoms_et_conversion() {
    let kingdoms = vec![
        KingdomView {
            name: "reference".to_string(),
            card: card_reference(),
            track: TrackOrder::Lineaire(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "<a>".to_string(),
            ]),
        },
        KingdomView {
            name: "temoin-bdd".to_string(),
            card: card_bdd(),
            track: TrackOrder::Lineaire(vec!["x0".to_string(), "x1".to_string()]),
        },
    ];
    let conversions = vec![ConversionView {
        source: "reference".to_string(),
        target: "temoin-bdd".to_string(),
        cout: 3,
    }];
    let svg = rendre_svg_view_order(&kingdoms, &conversions);

    // Controller card: the eight fields.
    for champ in [
        "δ =",
        "Σ =",
        "decomposition + O =",
        "R =",
        "defaults =",
        "D^def =",
        "encoding =",
        "attribution =",
    ] {
        assert!(svg.contains(champ), "card field present: {champ}");
    }
    // Two kingdoms side by side.
    assert!(svg.contains("reference"));
    assert!(svg.contains("temoin-bdd"));
    // Linear track: the levels in order (increasing x).
    let x_a = x_texte(&svg, ">a<");
    let x_b = x_texte(&svg, ">b<");
    let x_c = x_texte(&svg, ">c<");
    assert!(x_a < x_b && x_b < x_c, "levels in sweep order");
    // Arrowed conversion with cost.
    assert!(svg.contains("convert reference → temoin-bdd (cost 3)"));
    assert!(svg.contains("stroke-dasharray=\"6 3\""), "dashed arrow");
    // XML escaping of labels: "<a>" rendered as &lt;a&gt;, never as a tag.
    assert!(svg.contains("&lt;a&gt;"), "label escaped as &lt;a&gt;");
    assert!(!svg.contains("><a><"), "no injected tag");
}

/// M3-4: a structured order renders as a tree (vtree), no special case.
#[test]
fn test_view_order_track_arbre_vtree() {
    let arbre = NodeVtree {
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
        track: TrackOrder::Arbre(arbre),
    }];
    let svg = rendre_svg_view_order(&kingdoms, &[]);
    for libelle in ["record", "machine", "cell", "state", "conveyor"] {
        assert!(
            svg.contains(&format!(">{libelle}<")),
            "vtree node: {libelle}"
        );
    }
    // The parent is above its children (parsed y, not offsets).
    let y_record = y_texte(&svg, ">record<");
    let y_cell = y_texte(&svg, ">cell<");
    assert!(y_record < y_cell, "parent above the child");
}

/// M3-4: the rendering is deterministic and a conversion to an
/// unknown kingdom is ignored (no arrow).
#[test]
fn test_view_order_deterministe_et_conversion_inconnue_ignoree() {
    let kingdoms = vec![KingdomView {
        name: "r".to_string(),
        card: card_reference(),
        track: TrackOrder::Lineaire(vec!["n0".to_string()]),
    }];
    let conversions = vec![ConversionView {
        source: "r".to_string(),
        target: "inconnu".to_string(),
        cout: 1,
    }];
    let svg = rendre_svg_view_order(&kingdoms, &conversions);
    assert!(!svg.contains("convert r → inconnu"));
    let svg2 = rendre_svg_view_order(&kingdoms, &conversions);
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// M3-4: an arrowed conversion anchors at the middle of the track **whatever
/// its height** — including a tree track (vtree), not only a
/// linear track.
#[test]
fn test_conversion_anchored_dans_track_arbre() {
    let arbre = NodeVtree {
        libelle: "record".to_string(),
        children: vec![
            NodeVtree {
                libelle: "gauche".to_string(),
                children: vec![NodeVtree {
                    libelle: "feuille".to_string(),
                    children: vec![],
                }],
            },
            NodeVtree {
                libelle: "droite".to_string(),
                children: vec![],
            },
        ],
    };
    let kingdoms = vec![
        KingdomView {
            name: "arbre".to_string(),
            card: card_reference(),
            track: TrackOrder::Arbre(arbre),
        },
        KingdomView {
            name: "lineaire".to_string(),
            card: card_bdd(),
            track: TrackOrder::Lineaire(vec!["x0".to_string()]),
        },
    ];
    let conversions = vec![ConversionView {
        source: "arbre".to_string(),
        target: "lineaire".to_string(),
        cout: 7,
    }];
    let svg = rendre_svg_view_order(&kingdoms, &conversions);
    assert!(svg.contains("convert arbre → lineaire (cost 7)"));
    // The arrow starts from the middle of the tree track (3 levels of 44 px,
    // i.e. y_track + 66): parsed y1, strictly under the root and its first
    // level — an anchoring on the root (y_track + 28) would fail here.
    let y_root = y_texte(&svg, ">record<");
    let y1 = y1_fleche_conversion(&svg);
    assert!(
        y1 > y_root + 44,
        "arrow anchored at the middle of the track (y1={y1}, root={y_root})"
    );
}
