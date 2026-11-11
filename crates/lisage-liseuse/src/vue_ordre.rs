//! Liseuse — **vue 1 : l'ordre et les contrôleurs** (M3-4, doc liseuse § 4) :
//! la fiche contrôleur (les huit champs de `C`), l'ordre `O` rendu comme
//! piste linéaire horizontale (ou arbre / vtree), les royaumes côte à côte,
//! et les conversions fléchées avec leur coût.
//!
//! La liseuse ne dépend pas de `lisage-plan` (frontière de dépendances) :
//! elle rend une **description neutre** ([`RoyaumeVue`], [`ConversionVue`])
//! produite par le plan ; elle n'interprète jamais les libellés.

use crate::svg::echapper_xml;

/// Largeur de la vue en pixels.
const LARGEUR_VUE: u32 = 960;
/// Marge extérieure en pixels.
const MARGE: u32 = 20;
/// Hauteur d'une ligne de la fiche contrôleur en pixels.
const HAUTEUR_LIGNE_FICHE: u32 = 16;
/// Hauteur d'une piste linéaire en pixels.
const HAUTEUR_PISTE: u32 = 56;
/// Hauteur d'un niveau d'arbre (vtree) en pixels.
const HAUTEUR_NIVEAU_ARBRE: u32 = 44;
/// Largeur d'une boîte de la piste en pixels.
const LARGEUR_BOITE: u32 = 110;
/// Couleur de la piste (bordures des niveaux).
const COULEUR_PISTE: &str = "#2980b9";
/// Couleur de la fiche contrôleur.
const COULEUR_FICHE: &str = "#555";
/// Couleur des conversions fléchées.
const COULEUR_CONVERSION: &str = "#e67e22";
/// Couleur du texte.
const COULEUR_TEXTE: &str = "#222";
/// Fond de la fiche contrôleur.
const FOND_FICHE: &str = "#f7f7f7";
/// Fond des boîtes de niveau.
const FOND_BOITE: &str = "#fff";
/// Taille de police du titre de la vue.
const TAILLE_TITRE: u32 = 14;
/// Taille de police d'un nom de royaume / titre de fiche.
const TAILLE_NOM: u32 = 13;
/// Taille de police d'un champ de fiche.
const TAILLE_CHAMP: u32 = 12;
/// Taille de police d'un libellé de piste / conversion.
const TAILLE_PISTE: u32 = 11;
/// Décalage intérieur standard (marge dans un panneau).
const DECALAGE: u32 = 8;
/// Hauteur du bandeau de titre de la vue.
const HAUTEUR_TITRE: u32 = 30;
/// Espace après la piste d'un royaume.
const ESPACE_APRES_PISTE: u32 = 16;
/// Offset vertical du titre de fiche.
const DECALAGE_TITRE_FICHE: u32 = 16;
/// Offset vertical de la première ligne de champ.
const DECALAGE_LIGNE_FICHE: u32 = 24;
/// Offset vertical de la baseline d'une ligne de champ.
const BASELINE_LIGNE_FICHE: u32 = 12;
/// Offset horizontal d'une boîte dans la piste.
const DECALAGE_BOITE: u32 = 4;
/// Baseline du texte dans une boîte de niveau.
const BASELINE_BOITE: u32 = 18;
/// Centre vertical d'une boîte de niveau.
const MILIEU_BOITE: u32 = 14;
/// Offset horizontal de la flèche de conversion.
const DECALAGE_FLECHE: u32 = 40;
/// Offset horizontal de l'étiquette de conversion.
const DECALAGE_ETIQUETTE: u32 = 48;
/// Offset vertical de l'étiquette de conversion sous le milieu de la flèche.
const DECALAGE_ETIQUETTE_Y: u32 = 4;
/// Géométrie des marqueurs de flèche (taille, refX, refY).
const MARQUEUR_TAILLE: u32 = 8;
const MARQUEUR_REF_Y: u32 = 4;
/// Nombre de champs de la fiche contrôleur (C = huit champs).
const NOMBRE_CHAMPS_FICHE: u32 = 8;
/// Hauteur d'une boîte de niveau en pixels.
const HAUTEUR_BOITE: u32 = 28;
/// Épaisseur des traits en pixels.
const EPAISSEUR_TRAIT: u32 = 2;
/// Hauteur d'une ligne de conversion en pixels.
const HAUTEUR_LIGNE_CONVERSION: u32 = 30;
/// Pointillés des flèches de conversion.
const DASH_CONVERSION: &str = "6 3";

/// Hauteur de la fiche contrôleur en pixels.
const fn hauteur_fiche() -> u32 {
    DECALAGE_LIGNE_FICHE + NOMBRE_CHAMPS_FICHE * HAUTEUR_LIGNE_FICHE
}

/// Fiche contrôleur : les huit champs de `C = (δ, Σ, décomposition + O, R,
/// défauts, D^déf, encodage, attribution)`, en libellés neutres.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FicheControleur {
    /// δ : algèbre des clés et défauts par niveau.
    pub delta: String,
    /// Σ : expression de types dépliée par l'ordre.
    pub sigma: String,
    /// Décomposition + ordre O.
    pub decomposition_ordre: String,
    /// R : règles de réduction.
    pub regles: String,
    /// Défauts désignés par niveau.
    pub defauts: String,
    /// Défaut global D^déf désigné par le contrôleur.
    pub defaut_global: String,
    /// Encodage (royaume) : balayage d'univers, énumération…
    pub encodage: String,
    /// Attribution (canaux hérité/synthétisé/mémoire).
    pub attribution: String,
}

/// Nœud d'un arbre d'ordre (vtree) : un libellé et ses enfants.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NoeudVtree {
    /// Libellé du nœud (fragment de la décomposition).
    pub libelle: String,
    /// Enfants (sous-arbres).
    pub enfants: Vec<NoeudVtree>,
}

/// Piste d'ordre `O` : linéaire (niveaux dans l'ordre de balayage) ou
/// structurée en arbre (vtree).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PisteOrdre {
    /// O linéaire : libellés des niveaux dans l'ordre.
    Lineaire(Vec<String>),
    /// O structuré : arbre (vtree).
    Arbre(NoeudVtree),
}

/// Royaume de contrôleur : une fiche + une piste d'ordre.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RoyaumeVue {
    /// Nom du royaume (contrôleur).
    pub nom: String,
    /// Fiche contrôleur (huit champs).
    pub fiche: FicheControleur,
    /// Piste d'ordre.
    pub piste: PisteOrdre,
}

/// Conversion fléchée entre deux royaumes, avec son coût (événement
/// `Convert` : contrôleur source, cible, coût).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConversionVue {
    /// Nom du royaume source.
    pub source: String,
    /// Nom du royaume cible.
    pub cible: String,
    /// Coût mesuré/prédit de la conversion.
    pub cout: u64,
}

/// Hauteur en pixels d'un arbre (vtree).
fn hauteur_arbre(noeud: &NoeudVtree) -> u32 {
    let hauteur_enfants = noeud.enfants.iter().map(hauteur_arbre).max().unwrap_or(0);
    HAUTEUR_NIVEAU_ARBRE + hauteur_enfants
}

/// Hauteur en pixels d'une piste.
fn hauteur_piste(piste: &PisteOrdre) -> u32 {
    match piste {
        PisteOrdre::Lineaire(_) => HAUTEUR_PISTE,
        PisteOrdre::Arbre(racine) => hauteur_arbre(racine),
    }
}

/// Rend la fiche contrôleur (huit champs en panneau) ; retourne la hauteur
/// rendue.
fn rendre_fiche(svg: &mut String, fiche: &FicheControleur, x: u32, y: u32, largeur: u32) -> u32 {
    let champs = [
        ("δ", &fiche.delta),
        ("Σ", &fiche.sigma),
        ("décomposition + O", &fiche.decomposition_ordre),
        ("R", &fiche.regles),
        ("défauts", &fiche.defauts),
        ("D^déf", &fiche.defaut_global),
        ("encodage", &fiche.encodage),
        ("attribution", &fiche.attribution),
    ];
    debug_assert_eq!(champs.len() as u32, NOMBRE_CHAMPS_FICHE);
    let hauteur = hauteur_fiche();
    svg.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{largeur}\" height=\"{hauteur}\" fill=\"{FOND_FICHE}\" stroke=\"{COULEUR_FICHE}\"/>\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_NOM}\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">fiche contrôleur</text>\n",
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

/// Rend une piste linéaire : les niveaux dans l'ordre de balayage, en
/// boîtes reliées par des flèches.
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

/// Rend un arbre (vtree) : le nœud centré, les enfants répartis en largeur.
/// Retourne la hauteur rendue.
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

/// Rend une piste d'ordre (linéaire ou arbre) ; retourne la hauteur rendue.
fn rendre_piste(svg: &mut String, piste: &PisteOrdre, x: u32, y: u32, largeur: u32) -> u32 {
    match piste {
        PisteOrdre::Lineaire(niveaux) => {
            rendre_piste_lineaire(svg, niveaux, x, y, largeur);
            HAUTEUR_PISTE
        }
        PisteOrdre::Arbre(racine) => rendre_arbre(svg, racine, x, y, largeur),
    }
}

/// Position d'un royaume dans la vue (source de vérité unique du layout,
/// utilisée par le rendu : les deux passes ne peuvent pas diverger).
struct PositionRoyaume {
    y_nom: u32,
    y_fiche: u32,
    y_piste: u32,
    hauteur_piste: u32,
}

/// Calcule le layout complet : hauteur totale du SVG + position de chaque
/// royaume (nom, fiche, piste).
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

/// Rend la vue ordre en SVG statique : pour chaque royaume, sa fiche
/// contrôleur puis sa piste d'ordre ; les conversions fléchées avec leur
/// coût relient les pistes des royaumes source/cible (ancrées au milieu de
/// la piste, quelle que soit sa hauteur).
pub fn rendre_svg_vue_ordre(royaumes: &[RoyaumeVue], conversions: &[ConversionVue]) -> String {
    let largeur = LARGEUR_VUE - 2 * MARGE;
    let (hauteur, positions) = layout(royaumes, conversions.len());
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{LARGEUR_VUE}\" height=\"{hauteur}\" viewBox=\"0 0 {LARGEUR_VUE} {hauteur}\">\n<defs><marker id=\"fleche\" markerWidth=\"{MARQUEUR_TAILLE}\" markerHeight=\"{MARQUEUR_TAILLE}\" refX=\"{MARQUEUR_TAILLE}\" refY=\"{MARQUEUR_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARQUEUR_TAILLE},{MARQUEUR_REF_Y} L0,{MARQUEUR_TAILLE} z\" fill=\"{COULEUR_PISTE}\"/></marker><marker id=\"fleche-conversion\" markerWidth=\"{MARQUEUR_TAILLE}\" markerHeight=\"{MARQUEUR_TAILLE}\" refX=\"{MARQUEUR_TAILLE}\" refY=\"{MARQUEUR_REF_Y}\" orient=\"auto\"><path d=\"M0,0 L{MARQUEUR_TAILLE},{MARQUEUR_REF_Y} L0,{MARQUEUR_TAILLE} z\" fill=\"{COULEUR_CONVERSION}\"/></marker></defs>\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE}\" y=\"{DECALAGE_TITRE_FICHE}\" font-family=\"serif\" font-size=\"{TAILLE_TITRE}\" fill=\"{COULEUR_TEXTE}\">vue ordre — {} royaume(x), {} conversion(s)</text>\n",
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
    // Conversions fléchées entre pistes, avec le coût.
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
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{TAILLE_PISTE}\" text-anchor=\"end\" fill=\"{COULEUR_CONVERSION}\">convert {} → {} (coût {})</text>\n",
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

    /// Le layout est cohérent : chaque piste est posée après sa fiche, et
    /// la hauteur totale couvre toutes les pistes (pas de clipping).
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
                "piste posée après la fiche"
            );
            assert!(
                position.y_piste + position.hauteur_piste <= hauteur,
                "piste couverte par la hauteur totale"
            );
        }
        assert!(positions[1].y_piste > positions[0].y_piste);
    }
}
