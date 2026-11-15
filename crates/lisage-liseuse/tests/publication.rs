//! Publication mode tests (M5-5): standalone SVG in the serif theme with
//! legend auto-generated from the channel legend, and PGF/TikZ export.

use std::collections::BTreeSet;

use lisage_core::arena::Arena;
use lisage_core::observer::CanalAttr;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiler};
use lisage_core::{Anchor, NodeId};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::publication::{
    LEGENDE_CANAUX, Theme, rendre_legende_svg, rendre_pgf, rendre_svg_publication,
};

type PoolRef = Arena<ReferenceController>;

/// Test schema: `Set({0,1})`.
fn schema_test() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1)]))
}

/// Builds the diagram of the set `{0}` in a new pool.
fn diagramme_test() -> (PoolRef, NodeId) {
    let schema = schema_test();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let value = Value::Set(BTreeSet::from([Value::Atom(Atom(0))]));
    let root = compiler(&mut pool, Anchor(0), &schema, &value).unwrap();
    (pool, root)
}

/// The publication SVG is standalone, in the serif theme, with the escaped title.
#[test]
fn test_svg_publication_theme_serif_et_titre() {
    let (pool, root) = diagramme_test();
    let theme = Theme::publication();
    let svg = rendre_svg_publication(&pool, root, "figure <A> & B", &theme, false);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("font-family=\"serif\""));
    assert!(svg.contains("figure &lt;A&gt; &amp; B"));
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.ends_with("</svg>\n"));
}

/// The legend is auto-generated from the channel legend: one swatch and one
/// label per channel, with the declared color.
#[test]
fn test_legende_auto_depuis_legende_des_canaux() {
    let theme = Theme::publication();
    let legende = rendre_legende_svg(&theme, 10, 20);
    assert_eq!(LEGENDE_CANAUX.len(), 3);
    for (canal, etiquette, couleur) in LEGENDE_CANAUX {
        assert!(
            legende.contains(&format!("fill=\"{couleur}\"")),
            "color of {canal:?}"
        );
        assert!(legende.contains(etiquette), "label of {canal:?}");
    }
    // The publication SVG with legend embeds the three channels.
    let (pool, root) = diagramme_test();
    let svg = rendre_svg_publication(&pool, root, "fig", &theme, true);
    for (_canal, etiquette, _couleur) in LEGENDE_CANAUX {
        assert!(svg.contains(etiquette));
    }
    // Without legend: no channel swatch.
    let svg_sans = rendre_svg_publication(&pool, root, "fig", &theme, false);
    assert!(!svg_sans.contains("inherited (φ_disp)"));
}

/// The publication rendering is deterministic (two identical calls).
#[test]
fn test_svg_publication_deterministe() {
    let (pool, root) = diagramme_test();
    let theme = Theme::publication();
    let first = rendre_svg_publication(&pool, root, "fig", &theme, true);
    let second = rendre_svg_publication(&pool, root, "fig", &theme, true);
    assert_eq!(first, second);
}

/// The PGF is a standalone TikZ document: diagram nodes and arcs, title.
#[test]
fn test_pgf_autonome_avec_nodes_et_arcs() {
    let (pool, root) = diagramme_test();
    let pgf = rendre_pgf(&pool, root, "figure A");
    assert!(pgf.contains("\\documentclass[tikz,border=5pt]{standalone}"));
    assert!(pgf.contains("\\begin{tikzpicture}"));
    assert!(pgf.contains("\\end{tikzpicture}"));
    assert!(pgf.contains("\\end{document}"));
    assert!(pgf.contains("figure A"));
    // The diagram {0} over Set({0,1}) has 1 level node + the terminal ⊤
    // (the terminal ⊥ is collapsed by R-default), and 1 arc.
    let nombre_nodes = pgf.matches("\\node[draw, rectangle").count();
    assert_eq!(nombre_nodes, 2);
    assert_eq!(pgf.matches("\\draw[->]").count(), 1);
    // LaTeX escaping of the title.
    let pgf_echappe = rendre_pgf(&pool, root, "100% _special_");
    assert!(pgf_echappe.contains("100\\% \\_special\\_"));
    // Regression: the backslash must be escaped last, otherwise the
    // braces of `\textbackslash{}` are re-escaped.
    let pgf_backslash = rendre_pgf(&pool, root, "a\\b");
    assert!(pgf_backslash.contains("a\\textbackslash{}b"));
}

/// The `CanalAttr` channel covered by the legend is the observer's one.
#[test]
fn test_legende_couvre_les_trois_canaux() {
    let canaux: Vec<CanalAttr> = LEGENDE_CANAUX.iter().map(|(canal, _, _)| *canal).collect();
    assert_eq!(
        canaux,
        vec![
            CanalAttr::Inherited,
            CanalAttr::Synthesized,
            CanalAttr::Memory
        ]
    );
}
