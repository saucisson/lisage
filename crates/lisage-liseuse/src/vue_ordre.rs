//! Reader — **view 1: the order and the controllers** (M3-4, reader doc § 4):
//! the controller card (the eight fields of `C`), the order `O` rendered as
//! a horizontal linear track (or tree / vtree), the kingdoms side by side,
//! and the arrowed conversions with their cost.
//!
//! The reader does not depend on `lisage-plan` (dependency boundary):
//! it renders a **neutral description** ([`RoyaumeVue`], [`ConversionVue`])
//! produced by the planner; it never interprets the labels.

use crate::svg::echapper_xml;

/// Width of the view in pixels.
const LARGEUR_VUE: u32 = 960;
/// Outer margin in pixels.
const MARGE: u32 = 20;
/// Height of a controller-card line in pixels.
const HAUTEUR_LIGNE_FICHE: u32 = 16;
/// Height of a linear track in pixels.
const HAUTEUR_PISTE: u32 = 56;
/// Height of a tree level (vtree) in pixels.
const HAUTEUR_NIVEAU_ARBRE: u32 = 44;
/// Width of a track box in pixels.
const LARGEUR_BOITE: u32 = 110;
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
const TAILLE_TITRE: u32 = 14;
/// Font size of a kingdom name / card title.
const TAILLE_NOM: u32 = 13;
/// Font size of a card field.
const TAILLE_CHAMP: u32 = 12;
/// Font size of a track / conversion label.
const TAILLE_PISTE: u32 = 11;
/// Standard inner offset (margin inside a panel).
const DECALAGE: u32 = 8;
/// Height of the view title band.
const HAUTEUR_TITRE: u32 = 30;
/// Space after a kingdom's track.
const ESPACE_APRES_PISTE: u32 = 16;
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
const DECALAGE_ETIQUETTE: u32 = 48;
/// Vertical offset of the conversion label below the arrow's middle.
const DECALAGE_ETIQUETTE_Y: u32 = 4;
/// Arrow-marker geometry (size, refX, refY).
const MARQUEUR_TAILLE: u32 = 8;
const MARQUEUR_REF_Y: u32 = 4;
/// Number of controller-card fields (C = eight fields).
const NOMBRE_CHAMPS_FICHE: u32 = 8;
/// Height of a level box in pixels.
const HAUTEUR_BOITE: u32 = 28;
/// Stroke thickness in pixels.
const EPAISSEUR_TRAIT: u32 = 2;
/// Height of a conversion line in pixels.
const HAUTEUR_LIGNE_CONVERSION: u32 = 30;
/// Dashes of the conversion arrows.
const DASH_CONVERSION: &str = "6 3";

/// Height of the controller card in pixels.
const fn hauteur_fiche() -> u32 {
    DECALAGE_LIGNE_FICHE + NOMBRE_CHAMPS_FICHE * HAUTEUR_LIGNE_FICHE
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
    pub decomposition_ordre: String,
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
pub struct NoeudVtree {
    /// Node label (fragment of the decomposition).
    pub libelle: String,
    /// Children (subtrees).
    pub enfants: Vec<NoeudVtree>,
}

/// Order track `O`: linear (levels in sweep order) or
/// structured as a tree (vtree).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PisteOrdre {
    /// Linear O: level labels in order.
    Lineaire(Vec<String>),
    /// Structured O: tree (vtree).
    Arbre(NoeudVtree),
}

/// Controller kingdom: a card + an order track.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RoyaumeVue {
    /// Kingdom name (controller).
    pub nom: String,
    /// Controller card (eight fields).
    pub fiche: FicheControleur,
    /// Order track.
    pub piste: PisteOrdre,
}

/// Arrowed conversion between two kingdoms, with its cost (event
/// `Convert`: source controller, target, cost).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConversionVue {
    /// Source kingdom name.
    pub source: String,
    /// Target kingdom name.
    pub cible: String,
    /// Measured/predicted cost of the conversion.
    pub cout: u64,
}

/// Height in pixels of a tree (vtree).
fn hauteur_arbre(noeud: &NoeudVtree) -> u32 {
    let hauteur_enfants = noeud.enfants.iter().map(hauteur_arbre).max().unwrap_or(0);
    HAUTEUR_NIVEAU_ARBRE + hauteur_enfants
}

/// Height in pixels of a track.
fn hauteur_piste(piste: &PisteOrdre) -> u32 {
    match piste {
        PisteOrdre::Lineaire(_) => HAUTEUR_PISTE,
        PisteOrdre::Arbre(racine) => hauteur_arbre(racine),
    }
}

/// Renders the controller card (eight fields in a panel); returns the
/// rendered height.
fn rendre_fiche(svg: &mut String, fiche: &FicheControleur, x: u32, y: u32, largeur: u32) -> u32 {
    let champs = [
        ("δ", &fiche.delta),
        ("Σ", &fiche.sigma),
        ("decomposition + O", &fiche.decomposition_ordre),
        ("R", &fiche.regles),
        ("defaults", &fiche.defauts),
        ("D^def", &fiche.defaut_global),
        ("encoding", &fiche.encodage),
        ("attribution", &fiche.attribution),
    ];
    debug_assert_eq!(champs.len() as u32, NOMBRE_CHAMPS_FICHE);
    let hauteur = hauteur_fiche();
    svg.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{largeur}\" height=\"{hauteur}\" fill=\"{FOND_FICHE}\" stroke=\"{COULEUR_FICHE}\"/>\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_NOM}\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">controller card</text>\n",
        x + DECALAGE,
        y + DECALAGE_TITRE_FICHE
    ));
    for (index, (champ, valeur)) in champs.iter().enumerate() {
        let ligne_y =
            y + DECALAGE_LIGNE_FICHE + index as u32 * HAUTEUR_LIGNE_FICHE + BASELINE_LIGNE_FICHE;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{ligne_y}\" font-family=\"serif\" font-size=\"{TAILLE_CHAMP}\" fill=\"{COULEUR_TEXTE}\">{champ} = {}</text>\n",
            x + DECALAGE,
            echapper_xml(valeur)
        ));
    }
    hauteur
}

/// Renders a linear track: the levels in sweep order, in
/// boxes linked by arrows.
fn rendre_piste_lineaire(svg: &mut String, niveaux: &[String], x: u32, y: u32, largeur: u32) {
    let y_boite = y + DECALAGE;
    let nombre = niveaux.len().max(1) as u32;
    let pas = largeur / nombre;
    for (index, libelle) in niveaux.iter().enumerate() {
        let boite_x = x + index as u32 * pas + DECALAGE_BOITE;
        let boite_largeur = pas.saturating_sub(DECALAGE).min(LARGEUR_BOITE);
        svg.push_str(&format!(
            "<rect x=\"{boite_x}\" y=\"{y_boite}\" width=\"{boite_largeur}\" height=\"{HAUTEUR_BOITE}\" fill=\"{FOND_BOITE}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_PISTE}\" text-anchor=\"middle\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
            boite_x + boite_largeur / 2,
            y_boite + BASELINE_BOITE,
            echapper_xml(libelle)
        ));
        if index + 1 < niveaux.len() {
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\" marker-end=\"url(#fleche)\"/>\n",
                boite_x + boite_largeur,
                y_boite + MILIEU_BOITE,
                x + (index as u32 + 1) * pas + DECALAGE_BOITE,
                y_boite + MILIEU_BOITE
            ));
        }
    }
}

/// Renders a tree (vtree): the node centered, the children spread in width.
/// Returns the rendered height.
fn rendre_arbre(svg: &mut String, noeud: &NoeudVtree, x: u32, y: u32, largeur: u32) -> u32 {
    let centre_x = x + largeur / 2;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{y}\" width=\"{LARGEUR_BOITE}\" height=\"{HAUTEUR_BOITE}\" fill=\"{FOND_BOITE}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"{EPAISSEUR_TRAIT}\"/>\n",
        centre_x - LARGEUR_BOITE / 2
    ));
    svg.push_str(&format!(
        "<text x=\"{centre_x}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_PISTE}\" text-anchor=\"middle\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
        y + BASELINE_BOITE,
        echapper_xml(&noeud.libelle)
    ));
    if noeud.enfants.is_empty() {
        return HAUTEUR_NIVEAU_ARBRE;
    }
    let nombre = noeud.enfants.len() as u32;
    let pas = largeur / nombre;
    let y_enfants = y + HAUTEUR_NIVEAU_ARBRE;
    let mut hauteur_max = 0;
    for (index, enfant) in noeud.enfants.iter().enumerate() {
        let enfant_x = x + index as u32 * pas;
        let enfant_centre = enfant_x + pas / 2;
        svg.push_str(&format!(
            "<line x1=\"{centre_x}\" y1=\"{}\" x2=\"{enfant_centre}\" y2=\"{y_enfants}\" stroke=\"{COULEUR_PISTE}\"/>\n",
            y + HAUTEUR_BOITE
        ));
        let hauteur = rendre_arbre(svg, enfant, enfant_x, y_enfants, pas);
        hauteur_max = hauteur_max.max(hauteur);
    }
    HAUTEUR_NIVEAU_ARBRE + hauteur_max
}

/// Renders an order track (linear or tree); returns the rendered height.
fn rendre_piste(svg: &mut String, piste: &PisteOrdre, x: u32, y: u32, largeur: u32) -> u32 {
    match piste {
        PisteOrdre::Lineaire(niveaux) => {
            rendre_piste_lineaire(svg, niveaux, x, y, largeur);
            HAUTEUR_PISTE
        }
        PisteOrdre::Arbre(racine) => rendre_arbre(svg, racine, x, y, largeur),
    }
}

/// Position of a kingdom in the view (single source of truth of the layout,
/// used by the rendering: the two passes cannot diverge).
struct PositionRoyaume {
    y_nom: u32,
    y_fiche: u32,
    y_piste: u32,
    hauteur_piste: u32,
}

/// Computes the full layout: total SVG height + position of each
/// kingdom (name, card, track).
fn layout(royaumes: &[RoyaumeVue], nombre_conversions: usize) -> (u32, Vec<PositionRoyaume>) {
    let mut hauteur = MARGE + HAUTEUR_TITRE;
    let mut positions = Vec::new();
    for royaume in royaumes {
        let hauteur_piste = hauteur_piste(&royaume.piste);
        positions.push(PositionRoyaume {
            y_nom: hauteur,
            y_fiche: hauteur + DECALAGE,
            y_piste: hauteur + DECALAGE + hauteur_fiche() + DECALAGE,
            hauteur_piste,
        });
        hauteur += DECALAGE + hauteur_fiche() + DECALAGE + hauteur_piste + ESPACE_APRES_PISTE;
    }
    hauteur += nombre_conversions as u32 * HAUTEUR_LIGNE_CONVERSION + MARGE;
    (hauteur, positions)
}

/// Renders the order view as static SVG: for each kingdom, its
/// controller card then its order track; the arrowed conversions with their
/// cost link the tracks of the source/target kingdoms (anchored at the middle
/// of the track, whatever its height).
pub fn rendre_svg_vue_ordre(royaumes: &[RoyaumeVue], conversions: &[ConversionVue]) -> String {
    let largeur = LARGEUR_VUE - 2 * MARGE;
    let (hauteur, positions) = layout(royaumes, conversions.len());
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{LARGEUR_VUE}\" height=\"{hauteur}\" viewBox=\"0 0 {LARGEUR_VUE} {hauteur}\">\n<defs><marker id=\"fleche\" markerWidth=\"{MARQUEUR_TAILLE}\" markerHeight=\"{MARQUEUR_TAILLE}\" refX=\"{MARQUEUR_TAILLE}\" refY=\"{MARQUEUR_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARQUEUR_TAILLE},{MARQUEUR_REF_Y} L0,{MARQUEUR_TAILLE} z\" fill=\"{COULEUR_PISTE}\"/></marker><marker id=\"fleche-conversion\" markerWidth=\"{MARQUEUR_TAILLE}\" markerHeight=\"{MARQUEUR_TAILLE}\" refX=\"{MARQUEUR_TAILLE}\" refY=\"{MARQUEUR_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARQUEUR_TAILLE},{MARQUEUR_REF_Y} L0,{MARQUEUR_TAILLE} z\" fill=\"{COULEUR_CONVERSION}\"/></marker></defs>\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE}\" y=\"{DECALAGE_TITRE_FICHE}\" font-family=\"serif\" font-size=\"{TAILLE_TITRE}\" fill=\"{COULEUR_TEXTE}\">order view — {} kingdom(s), {} conversion(s)</text>\n",
        royaumes.len(),
        conversions.len()
    ));
    for (royaume, position) in royaumes.iter().zip(&positions) {
        svg.push_str(&format!(
            "<text x=\"{MARGE}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_NOM}\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
            position.y_nom,
            echapper_xml(&royaume.nom)
        ));
        rendre_fiche(&mut svg, &royaume.fiche, MARGE, position.y_fiche, largeur);
        rendre_piste(&mut svg, &royaume.piste, MARGE, position.y_piste, largeur);
    }
    // Arrowed conversions between tracks, with the cost.
    for conversion in conversions {
        let Some(index_source) = royaumes.iter().position(|r| r.nom == conversion.source) else {
            continue;
        };
        let Some(index_cible) = royaumes.iter().position(|r| r.nom == conversion.cible) else {
            continue;
        };
        let y1 = positions[index_source].y_piste + positions[index_source].hauteur_piste / 2;
        let y2 = positions[index_cible].y_piste + positions[index_cible].hauteur_piste / 2;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{y1}\" x2=\"{}\" y2=\"{y2}\" stroke=\"{COULEUR_CONVERSION}\" stroke-width=\"{EPAISSEUR_TRAIT}\" stroke-dasharray=\"{DASH_CONVERSION}\" marker-end=\"url(#fleche-conversion)\"/>\n",
            LARGEUR_VUE - MARGE - DECALAGE_FLECHE,
            LARGEUR_VUE - MARGE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_PISTE}\" text-anchor=\"end\" fill=\"{COULEUR_CONVERSION}\">convert {} → {} (cost {})</text>\n",
            LARGEUR_VUE - MARGE - DECALAGE_ETIQUETTE,
            (y1 + y2) / 2 + DECALAGE_ETIQUETTE_Y,
            echapper_xml(&conversion.source),
            echapper_xml(&conversion.cible),
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
            RoyaumeVue {
                nom: "a".to_string(),
                fiche: FicheControleur {
                    delta: String::new(),
                    sigma: String::new(),
                    decomposition_ordre: String::new(),
                    regles: String::new(),
                    defauts: String::new(),
                    defaut_global: String::new(),
                    encodage: String::new(),
                    attribution: String::new(),
                },
                piste: PisteOrdre::Lineaire(vec!["x".to_string()]),
            },
            RoyaumeVue {
                nom: "b".to_string(),
                fiche: FicheControleur {
                    delta: String::new(),
                    sigma: String::new(),
                    decomposition_ordre: String::new(),
                    regles: String::new(),
                    defauts: String::new(),
                    defaut_global: String::new(),
                    encodage: String::new(),
                    attribution: String::new(),
                },
                piste: PisteOrdre::Arbre(NoeudVtree {
                    libelle: "r".to_string(),
                    enfants: vec![NoeudVtree {
                        libelle: "e".to_string(),
                        enfants: vec![],
                    }],
                }),
            },
        ];
        let (hauteur, positions) = layout(&royaumes, 1);
        assert_eq!(positions.len(), 2);
        for position in &positions {
            assert_eq!(position.y_fiche, position.y_nom + DECALAGE);
            assert_eq!(
                position.y_piste,
                position.y_fiche + hauteur_fiche() + DECALAGE,
                "track placed after the card"
            );
            assert!(
                position.y_piste + position.hauteur_piste <= hauteur,
                "track covered by the total height"
            );
        }
        assert!(positions[1].y_piste > positions[0].y_piste);
    }
}
