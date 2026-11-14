//! M3-4 tests (Liseuse view 1, liseuse doc § 4): controller card
//! (eight fields), order as a linear track or a tree (vtree), kingdoms side by
//! side, arrowed conversions with cost — deterministic static SVG rendering.

use lisage_liseuse::vue_ordre::{
    ConversionVue, FicheControleur, NoeudVtree, PisteOrdre, RoyaumeVue, rendre_svg_vue_ordre,
};

/// Extracts a numeric attribute from an SVG element (ex. `y="42"` → 42).
fn extraire_attribut(element: &str, attribut: &str) -> u32 {
    // Leading space: avoids matching the end of `font-family="..."`.
    let position_attribut = element
        .find(&format!(" {attribut}=\""))
        .unwrap_or_else(|| panic!("attribute {attribut} present"));
    let apres = &element[position_attribut + attribut.len() + 3..];
    let fin = apres.find('"').expect("end of the attribute");
    apres[..fin].parse().expect("integer attribute")
}

/// `y` coordinate of the text element carrying `marqueur` (e.g. `>record<`).
fn y_texte(svg: &str, marqueur: &str) -> u32 {
    let position = svg.find(marqueur).expect("marker present");
    let avant = &svg[..position];
    let y_pos = avant.rfind(" y=\"").expect("attribut y avant le marqueur");
    extraire_attribut(&avant[y_pos..], "y")
}

/// `x` coordinate of the text element carrying `marqueur`.
fn x_texte(svg: &str, marqueur: &str) -> u32 {
    let position = svg.find(marqueur).expect("marker present");
    let avant = &svg[..position];
    let x_pos = avant.rfind(" x=\"").expect("attribut x avant le marqueur");
    extraire_attribut(&avant[x_pos..], "x")
}

/// `y1` of the conversion arrow (dashed line).
fn y1_fleche_conversion(svg: &str) -> u32 {
    let pos = svg
        .find("stroke-dasharray=\"6 3\"")
        .expect("conversion arrow");
    let debut = svg[..pos].rfind("<line").expect("line element");
    extraire_attribut(&svg[debut..pos], "y1")
}

/// Controller card of the reference instance (eight fields filled).
fn fiche_reference() -> FicheControleur {
    FicheControleur {
        delta: "presence {absent, present}".to_string(),
        sigma: "Set(Atom(D))".to_string(),
        decomposition_ordre: "universe sweep, canonical O".to_string(),
        regles: "R-share, R-collapse, R-default".to_string(),
        defauts: "δ(ℓ) = absent".to_string(),
        defaut_global: "D_δ = terminal(false)".to_string(),
        encodage: "universe sweep".to_string(),
        attribution: "inherited / synthesized / memory".to_string(),
    }
}

/// Card of the classic BDD witness.
fn fiche_bdd() -> FicheControleur {
    FicheControleur {
        encodage: "classic BDD".to_string(),
        ..fiche_reference()
    }
}

/// M3-4: the order view renders the controller card (eight fields), the
/// linear track of the levels in sweep order, and the kingdoms side by
/// side with the arrowed conversion and its cost.
#[test]
fn test_vue_ordre_fiche_piste_lineaire_royaumes_et_conversion() {
    let royaumes = vec![
        RoyaumeVue {
            nom: "reference".to_string(),
            fiche: fiche_reference(),
            piste: PisteOrdre::Lineaire(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "<a>".to_string(),
            ]),
        },
        RoyaumeVue {
            nom: "temoin-bdd".to_string(),
            fiche: fiche_bdd(),
            piste: PisteOrdre::Lineaire(vec!["x0".to_string(), "x1".to_string()]),
        },
    ];
    let conversions = vec![ConversionVue {
        source: "reference".to_string(),
        cible: "temoin-bdd".to_string(),
        cout: 3,
    }];
    let svg = rendre_svg_vue_ordre(&royaumes, &conversions);

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
fn test_vue_ordre_piste_arbre_vtree() {
    let arbre = NoeudVtree {
        libelle: "record".to_string(),
        enfants: vec![
            NoeudVtree {
                libelle: "machine".to_string(),
                enfants: vec![
                    NoeudVtree {
                        libelle: "cellule".to_string(),
                        enfants: vec![],
                    },
                    NoeudVtree {
                        libelle: "state".to_string(),
                        enfants: vec![],
                    },
                ],
            },
            NoeudVtree {
                libelle: "convoyeur".to_string(),
                enfants: vec![],
            },
        ],
    };
    let royaumes = vec![RoyaumeVue {
        nom: "sdD".to_string(),
        fiche: fiche_reference(),
        piste: PisteOrdre::Arbre(arbre),
    }];
    let svg = rendre_svg_vue_ordre(&royaumes, &[]);
    for libelle in ["record", "machine", "cellule", "state", "convoyeur"] {
        assert!(
            svg.contains(&format!(">{libelle}<")),
            "vtree node: {libelle}"
        );
    }
    // The parent is above its children (parsed y, not offsets).
    let y_record = y_texte(&svg, ">record<");
    let y_cellule = y_texte(&svg, ">cellule<");
    assert!(y_record < y_cellule, "parent above the child");
}

/// M3-4: the rendering is deterministic and a conversion to an
/// unknown kingdom is ignored (no arrow).
#[test]
fn test_vue_ordre_deterministe_et_conversion_inconnue_ignoree() {
    let royaumes = vec![RoyaumeVue {
        nom: "r".to_string(),
        fiche: fiche_reference(),
        piste: PisteOrdre::Lineaire(vec!["n0".to_string()]),
    }];
    let conversions = vec![ConversionVue {
        source: "r".to_string(),
        cible: "inconnu".to_string(),
        cout: 1,
    }];
    let svg = rendre_svg_vue_ordre(&royaumes, &conversions);
    assert!(!svg.contains("convert r → inconnu"));
    let svg2 = rendre_svg_vue_ordre(&royaumes, &conversions);
    assert_eq!(svg, svg2, "deterministic rendering");
}

/// M3-4: an arrowed conversion anchors at the middle of the track **whatever
/// its height** — including a tree track (vtree), not only a
/// linear track.
#[test]
fn test_conversion_ancree_dans_piste_arbre() {
    let arbre = NoeudVtree {
        libelle: "record".to_string(),
        enfants: vec![
            NoeudVtree {
                libelle: "gauche".to_string(),
                enfants: vec![NoeudVtree {
                    libelle: "feuille".to_string(),
                    enfants: vec![],
                }],
            },
            NoeudVtree {
                libelle: "droite".to_string(),
                enfants: vec![],
            },
        ],
    };
    let royaumes = vec![
        RoyaumeVue {
            nom: "arbre".to_string(),
            fiche: fiche_reference(),
            piste: PisteOrdre::Arbre(arbre),
        },
        RoyaumeVue {
            nom: "lineaire".to_string(),
            fiche: fiche_bdd(),
            piste: PisteOrdre::Lineaire(vec!["x0".to_string()]),
        },
    ];
    let conversions = vec![ConversionVue {
        source: "arbre".to_string(),
        cible: "lineaire".to_string(),
        cout: 7,
    }];
    let svg = rendre_svg_vue_ordre(&royaumes, &conversions);
    assert!(svg.contains("convert arbre → lineaire (cost 7)"));
    // The arrow starts from the middle of the tree track (3 levels of 44 px,
    // i.e. y_track + 66): parsed y1, strictly under the root and its first
    // level — an anchoring on the root (y_track + 28) would fail here.
    let y_racine = y_texte(&svg, ">record<");
    let y1 = y1_fleche_conversion(&svg);
    assert!(
        y1 > y_racine + 44,
        "arrow anchored at the middle of the track (y1={y1}, root={y_racine})"
    );
}
