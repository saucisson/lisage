//! Tests du mode publication (M5-5) : SVG autonome au thème serif avec
//! légende auto-générée depuis la légende des canaux, et export PGF/TikZ.

use std::collections::BTreeSet;

use lisage_core::arena::Arena;
use lisage_core::observer::CanalAttr;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_core::{Anchor, NodeId};
use lisage_lang::value::{Atom, Value};
use lisage_liseuse::publication::{
    LEGENDE_CANAUX, Theme, rendre_legende_svg, rendre_pgf, rendre_svg_publication,
};

type PoolRef = Arena<ReferenceController>;

/// Schéma de test : `Set({0,1})`.
fn schema_test() -> Schema {
    Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]))
}

/// Construit le diagramme de l'ensemble `{0}` dans un pool neuf.
fn diagramme_test() -> (PoolRef, NodeId) {
    let schema = schema_test();
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let valeur = Value::Set(BTreeSet::from([Value::Atom(Atom(0))]));
    let racine = compiler(&mut pool, Anchor(0), &schema, &valeur).unwrap();
    (pool, racine)
}

/// Le SVG de publication est autonome, au thème serif, avec le titre échappé.
#[test]
fn test_svg_publication_theme_serif_et_titre() {
    let (pool, racine) = diagramme_test();
    let theme = Theme::publication();
    let svg = rendre_svg_publication(&pool, racine, "figure <A> & B", &theme, false);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("font-family=\"serif\""));
    assert!(svg.contains("figure &lt;A&gt; &amp; B"));
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.ends_with("</svg>\n"));
}

/// La légende est auto-générée depuis la légende des canaux : un pavé et une
/// étiquette par canal, avec la couleur déclarée.
#[test]
fn test_legende_auto_depuis_legende_des_canaux() {
    let theme = Theme::publication();
    let legende = rendre_legende_svg(&theme, 10, 20);
    assert_eq!(LEGENDE_CANAUX.len(), 3);
    for (canal, etiquette, couleur) in LEGENDE_CANAUX {
        assert!(
            legende.contains(&format!("fill=\"{couleur}\"")),
            "couleur de {canal:?}"
        );
        assert!(legende.contains(etiquette), "étiquette de {canal:?}");
    }
    // Le SVG de publication avec légende embarque les trois canaux.
    let (pool, racine) = diagramme_test();
    let svg = rendre_svg_publication(&pool, racine, "fig", &theme, true);
    for (_canal, etiquette, _couleur) in LEGENDE_CANAUX {
        assert!(svg.contains(etiquette));
    }
    // Sans légende : aucun pavé de canal.
    let svg_sans = rendre_svg_publication(&pool, racine, "fig", &theme, false);
    assert!(!svg_sans.contains("hérité (φ_disp)"));
}

/// Le rendu de publication est déterministe (deux appels identiques).
#[test]
fn test_svg_publication_deterministe() {
    let (pool, racine) = diagramme_test();
    let theme = Theme::publication();
    let premier = rendre_svg_publication(&pool, racine, "fig", &theme, true);
    let second = rendre_svg_publication(&pool, racine, "fig", &theme, true);
    assert_eq!(premier, second);
}

/// Le PGF est un document TikZ autonome : nœuds et arcs du diagramme, titre.
#[test]
fn test_pgf_autonome_avec_noeuds_et_arcs() {
    let (pool, racine) = diagramme_test();
    let pgf = rendre_pgf(&pool, racine, "figure A");
    assert!(pgf.contains("\\documentclass[tikz,border=5pt]{standalone}"));
    assert!(pgf.contains("\\begin{tikzpicture}"));
    assert!(pgf.contains("\\end{tikzpicture}"));
    assert!(pgf.contains("\\end{document}"));
    assert!(pgf.contains("figure A"));
    // Le diagramme {0} sur Set({0,1}) a 1 nœud de niveau + le terminal ⊤
    // (le terminal ⊥ est collapsé par R-default), et 1 arc.
    let nombre_noeuds = pgf.matches("\\node[draw, rectangle").count();
    assert_eq!(nombre_noeuds, 2);
    assert_eq!(pgf.matches("\\draw[->]").count(), 1);
    // Échappement LaTeX du titre.
    let pgf_echappe = rendre_pgf(&pool, racine, "100% _special_");
    assert!(pgf_echappe.contains("100\\% \\_special\\_"));
}

/// Le canal `CanalAttr` couvert par la légende est bien celui de l'observateur.
#[test]
fn test_legende_couvre_les_trois_canaux() {
    let canaux: Vec<CanalAttr> = LEGENDE_CANAUX.iter().map(|(canal, _, _)| *canal).collect();
    assert_eq!(
        canaux,
        vec![CanalAttr::Herite, CanalAttr::Synthétise, CanalAttr::Memoire]
    );
}
