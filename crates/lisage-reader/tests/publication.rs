//! Tests of the publication mode (M5-5): standalone SVG in the serif theme
//! with legend auto-generated from the channel legend, and PGF/TikZ export.

use std::collections::BTreeSet;

use lisage_core::arena::Arena;
use lisage_core::observer::ChannelAttribute;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, compiled};
use lisage_core::{Anchor, NodeId};
use lisage_lang::value::{Atom, Value};
use lisage_reader::publication::{
    LEGEND_CANAUX, Theme, render_legend_svg, render_pgf, render_svg_publication,
};

type PoolRef = Arena<ReferenceController>;

/// Test schema: `Set({0,1})`.
fn schema_test() -> Schema {
    Schema::Set(Domain::new(vec![Atom(0), Atom(1)]))
}

/// Builds the diagram of the set `{0}` in a new pool.
fn diagram_test() -> (PoolRef, NodeId) {
    let schema = schema_test();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let value = Value::Set(BTreeSet::from([Value::Atom(Atom(0))]));
    let root = compiled(&mut pool, Anchor(0), &schema, &value).unwrap();
    (pool, root)
}

/// The publication SVG is standalone, in the serif theme, with the escaped title.
#[test]
fn test_svg_publication_theme_serif_and_title() {
    let (pool, root) = diagram_test();
    let theme = Theme::publication();
    let svg = render_svg_publication(&pool, root, "figure <A> & B", &theme, false);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("font-family=\"serif\""));
    assert!(svg.contains("figure &lt;A&gt; &amp; B"));
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.ends_with("</svg>\n"));
}

/// The legend is auto-generated from the channel legend: one swatch and one
/// label per channel, with the declared color.
#[test]
fn test_legend_auto_since_legend_of_channels() {
    let theme = Theme::publication();
    let legend = render_legend_svg(&theme, 10, 20);
    assert_eq!(LEGEND_CANAUX.len(), 3);
    for (channel, label, color) in LEGEND_CANAUX {
        assert!(
            legend.contains(&format!("fill=\"{color}\"")),
            "color of {channel:?}"
        );
        assert!(legend.contains(label), "label of {channel:?}");
    }
    // The publication SVG with legend embeds the three channels.
    let (pool, root) = diagram_test();
    let svg = render_svg_publication(&pool, root, "fig", &theme, true);
    for (_canal, label, _color) in LEGEND_CANAUX {
        assert!(svg.contains(label));
    }
    // Without legend: no channel swatch.
    let svg_without = render_svg_publication(&pool, root, "fig", &theme, false);
    assert!(!svg_without.contains("inherited (φ_disp)"));
}

/// The publication rendering is deterministic (two identical calls).
#[test]
fn test_svg_publication_deterministic() {
    let (pool, root) = diagram_test();
    let theme = Theme::publication();
    let first = render_svg_publication(&pool, root, "fig", &theme, true);
    let second = render_svg_publication(&pool, root, "fig", &theme, true);
    assert_eq!(first, second);
}

/// The PGF is a standalone TikZ document: nodes and edges of the diagram, title.
#[test]
fn test_pgf_autonomous_with_nodes_and_edges() {
    let (pool, root) = diagram_test();
    let pgf = render_pgf(&pool, root, "figure A");
    assert!(pgf.contains("\\documentclass[tikz,border=5pt]{standalone}"));
    assert!(pgf.contains("\\begin{tikzpicture}"));
    assert!(pgf.contains("\\end{tikzpicture}"));
    assert!(pgf.contains("\\end{document}"));
    assert!(pgf.contains("figure A"));
    // The diagram {0} over Set({0,1}) has 1 level node + the terminal ⊤
    // (the terminal ⊥ is collapsed by R-default), and 1 edge.
    let number_nodes = pgf.matches("\\node[draw, rectangle").count();
    assert_eq!(number_nodes, 2);
    assert_eq!(pgf.matches("\\draw[->]").count(), 1);
    // LaTeX escaping of the title.
    let pgf_escaped = render_pgf(&pool, root, "100% _special_");
    assert!(pgf_escaped.contains("100\\% \\_special\\_"));
    // Regression: the backslash must be escaped last, otherwise the
    // braces of `\textbackslash{}` are re-escaped.
    let pgf_backslash = render_pgf(&pool, root, "a\\b");
    assert!(pgf_backslash.contains("a\\textbackslash{}b"));
}

/// The `ChannelAttribute` channel covered by the legend is indeed the observer's one.
#[test]
fn test_legend_covers_the_three_channels() {
    let channels: Vec<ChannelAttribute> = LEGEND_CANAUX
        .iter()
        .map(|(channel, _, _)| *channel)
        .collect();
    assert_eq!(
        channels,
        vec![
            ChannelAttribute::Inherited,
            ChannelAttribute::Synthesize,
            ChannelAttribute::Memory
        ]
    );
}
