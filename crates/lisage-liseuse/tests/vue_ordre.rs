//! Tests de M3-4 (vue 1 de la Liseuse, doc liseuse § 4) : fiche contrôleur
//! (huit champs), ordre en piste linéaire ou arbre (vtree), royaumes côte à
//! côte, conversions fléchées avec coût — rendu SVG statique déterministe.

use lisage_liseuse::vue_ordre::{
    ConversionVue, FicheControleur, NoeudVtree, PisteOrdre, RoyaumeVue, rendre_svg_vue_ordre,
};

/// Extrait un attribut numérique d'un élément SVG (ex. `y="42"` → 42).
fn extraire_attribut(element: &str, attribut: &str) -> u32 {
    // Espace initial : évite de matcher la fin de `font-family="..."`.
    let pos = element
        .find(&format!(" {attribut}=\""))
        .unwrap_or_else(|| panic!("attribut {attribut} présent"));
    let apres = &element[pos + attribut.len() + 3..];
    let fin = apres.find('"').expect("fin de l'attribut");
    apres[..fin].parse().expect("attribut entier")
}

/// Coordonnée `y` de l'élément texte portant `marqueur` (ex. `>record<`).
fn y_texte(svg: &str, marqueur: &str) -> u32 {
    let position = svg.find(marqueur).expect("marqueur présent");
    let avant = &svg[..position];
    let y_pos = avant.rfind(" y=\"").expect("attribut y avant le marqueur");
    extraire_attribut(&avant[y_pos..], "y")
}

/// Coordonnée `x` de l'élément texte portant `marqueur`.
fn x_texte(svg: &str, marqueur: &str) -> u32 {
    let position = svg.find(marqueur).expect("marqueur présent");
    let avant = &svg[..position];
    let x_pos = avant.rfind(" x=\"").expect("attribut x avant le marqueur");
    extraire_attribut(&avant[x_pos..], "x")
}

/// `y1` de la flèche de conversion (ligne en pointillés).
fn y1_fleche_conversion(svg: &str) -> u32 {
    let pos = svg
        .find("stroke-dasharray=\"6 3\"")
        .expect("flèche de conversion");
    let debut = svg[..pos].rfind("<line").expect("élément line");
    extraire_attribut(&svg[debut..pos], "y1")
}

/// Fiche contrôleur de l'instance de référence (huit champs remplis).
fn fiche_reference() -> FicheControleur {
    FicheControleur {
        delta: "présence {absent, présent}".to_string(),
        sigma: "Set(Atom(D))".to_string(),
        decomposition_ordre: "balayage d'univers, O canonique".to_string(),
        regles: "R-share, R-collapse, R-default".to_string(),
        defauts: "δ(ℓ) = absent".to_string(),
        defaut_global: "D_δ = terminal(false)".to_string(),
        encodage: "balayage d'univers".to_string(),
        attribution: "hérité / synthétisé / mémoire".to_string(),
    }
}

/// Fiche du témoin BDD classique.
fn fiche_bdd() -> FicheControleur {
    FicheControleur {
        encodage: "BDD classique".to_string(),
        ..fiche_reference()
    }
}

/// M3-4 : la vue ordre rend la fiche contrôleur (huit champs), la piste
/// linéaire des niveaux dans l'ordre de balayage, et les royaumes côte à
/// côte avec la conversion fléchée et son coût.
#[test]
fn test_vue_ordre_fiche_piste_lineaire_royaumes_et_conversion() {
    let royaumes = vec![
        RoyaumeVue {
            nom: "reference".to_string(),
            fiche: fiche_reference(),
            piste: PisteOrdre::Lineaire(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
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

    // Fiche contrôleur : les huit champs.
    for champ in [
        "δ =",
        "Σ =",
        "décomposition + O =",
        "R =",
        "défauts =",
        "D^déf =",
        "encodage =",
        "attribution =",
    ] {
        assert!(svg.contains(champ), "champ de fiche présent: {champ}");
    }
    // Deux royaumes côte à côte.
    assert!(svg.contains("reference"));
    assert!(svg.contains("temoin-bdd"));
    // Piste linéaire : les niveaux dans l'ordre (x croissants).
    let x_a = x_texte(&svg, ">a<");
    let x_b = x_texte(&svg, ">b<");
    let x_c = x_texte(&svg, ">c<");
    assert!(x_a < x_b && x_b < x_c, "niveaux dans l'ordre de balayage");
    // Conversion fléchée avec coût.
    assert!(svg.contains("convert reference → temoin-bdd (coût 3)"));
    assert!(
        svg.contains("stroke-dasharray=\"6 3\""),
        "flèche en pointillés"
    );
    // Échappement XML des libellés.
    assert!(!svg.contains("<a>"), "pas de balise injectée");
}

/// M3-4 : un ordre structuré se rend en arbre (vtree), sans cas particulier.
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
                        libelle: "état".to_string(),
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
    for libelle in ["record", "machine", "cellule", "état", "convoyeur"] {
        assert!(
            svg.contains(&format!(">{libelle}<")),
            "nœud vtree: {libelle}"
        );
    }
    // Le parent est au-dessus de ses enfants (y parsés, pas des offsets).
    let y_record = y_texte(&svg, ">record<");
    let y_cellule = y_texte(&svg, ">cellule<");
    assert!(y_record < y_cellule, "parent au-dessus de l'enfant");
}

/// M3-4 : le rendu est déterministe et une conversion vers un royaume
/// inconnu est ignorée (pas de flèche).
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
    assert_eq!(svg, svg2, "rendu déterministe");
}

/// M3-4 : une conversion fléchée s'ancre au milieu de la piste **quelle que
/// soit sa hauteur** — y compris une piste en arbre (vtree), pas seulement
/// une piste linéaire.
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
    assert!(svg.contains("convert arbre → lineaire (coût 7)"));
    // La flèche part du milieu de la piste arbre (3 niveaux de 44 px, soit
    // y_piste + 66) : y1 parsé, strictement sous la racine et son premier
    // niveau — un ancrage sur la racine (y_piste + 28) échouerait ici.
    let y_racine = y_texte(&svg, ">record<");
    let y1 = y1_fleche_conversion(&svg);
    assert!(
        y1 > y_racine + 44,
        "flèche ancrée au milieu de la piste (y1={y1}, racine={y_racine})"
    );
}
