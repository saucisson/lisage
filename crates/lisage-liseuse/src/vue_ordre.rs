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
    let hauteur = 24 + champs.len() as u32 * HAUTEUR_LIGNE_FICHE;
    svg.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{largeur}\" height=\"{hauteur}\" fill=\"#f7f7f7\" stroke=\"{COULEUR_FICHE}\"/>\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"13\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">fiche contrôleur</text>\n",
        x + 8,
        y + 16
    ));
    for (index, (champ, valeur)) in champs.iter().enumerate() {
        let ligne_y = y + 24 + index as u32 * HAUTEUR_LIGNE_FICHE + 12;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{ligne_y}\" font-family=\"serif\" font-size=\"12\" fill=\"{COULEUR_TEXTE}\">{champ} = {}</text>\n",
            x + 8,
            echapper_xml(valeur)
        ));
    }
    hauteur
}

/// Rend une piste linéaire : les niveaux dans l'ordre de balayage, en
/// boîtes reliées par des flèches.
fn rendre_piste_lineaire(svg: &mut String, niveaux: &[String], x: u32, y: u32, largeur: u32) {
    let y_boite = y + 8;
    let nombre = niveaux.len().max(1) as u32;
    let pas = largeur / nombre;
    for (index, libelle) in niveaux.iter().enumerate() {
        let boite_x = x + index as u32 * pas + 4;
        let boite_largeur = pas.saturating_sub(8).min(LARGEUR_BOITE);
        svg.push_str(&format!(
            "<rect x=\"{boite_x}\" y=\"{y_boite}\" width=\"{boite_largeur}\" height=\"28\" fill=\"#fff\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"2\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" text-anchor=\"middle\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
            boite_x + boite_largeur / 2,
            y_boite + 18,
            echapper_xml(libelle)
        ));
        if index + 1 < niveaux.len() {
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"2\" marker-end=\"url(#fleche)\"/>\n",
                boite_x + boite_largeur,
                y_boite + 14,
                x + (index as u32 + 1) * pas + 4,
                y_boite + 14
            ));
        }
    }
}

/// Rend un arbre (vtree) : le nœud centré, les enfants répartis en largeur.
/// Retourne la hauteur rendue.
fn rendre_arbre(svg: &mut String, noeud: &NoeudVtree, x: u32, y: u32, largeur: u32) -> u32 {
    let centre_x = x + largeur / 2;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{y}\" width=\"{LARGEUR_BOITE}\" height=\"28\" fill=\"#fff\" stroke=\"{COULEUR_PISTE}\" stroke-width=\"2\"/>\n",
        centre_x - LARGEUR_BOITE / 2
    ));
    svg.push_str(&format!(
        "<text x=\"{centre_x}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" text-anchor=\"middle\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
        y + 18,
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
            y + 28
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

/// Rend la vue ordre en SVG statique : pour chaque royaume, sa fiche
/// contrôleur puis sa piste d'ordre ; les conversions fléchées avec leur
/// coût relient les pistes des royaumes source/cible.
pub fn rendre_svg_vue_ordre(royaumes: &[RoyaumeVue], conversions: &[ConversionVue]) -> String {
    let largeur = LARGEUR_VUE - 2 * MARGE;
    // Première passe : hauteur totale + position verticale de chaque piste.
    let mut hauteur = MARGE + 30;
    let mut pistes_y: Vec<(&str, u32)> = Vec::new();
    for royaume in royaumes {
        hauteur += 24 + 8 * HAUTEUR_LIGNE_FICHE + 8; // fiche
        pistes_y.push((&royaume.nom, hauteur));
        hauteur += hauteur_piste(&royaume.piste) + 16;
    }
    hauteur += conversions.len() as u32 * 30 + MARGE;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{LARGEUR_VUE}\" height=\"{hauteur}\" viewBox=\"0 0 {LARGEUR_VUE} {hauteur}\">\n<defs><marker id=\"fleche\" markerWidth=\"8\" markerHeight=\"8\" refX=\"8\" refY=\"4\" orient=\"auto\"><path d=\"M0,0 L8,4 L0,8 z\" fill=\"{COULEUR_PISTE}\"/></marker><marker id=\"fleche-conversion\" markerWidth=\"8\" markerHeight=\"8\" refX=\"8\" refY=\"4\" orient=\"auto\"><path d=\"M0,0 L8,4 L0,8 z\" fill=\"{COULEUR_CONVERSION}\"/></marker></defs>\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE}\" y=\"22\" font-family=\"serif\" font-size=\"14\" fill=\"{COULEUR_TEXTE}\">vue ordre — {} royaume(x), {} conversion(s)</text>\n",
        royaumes.len(),
        conversions.len()
    ));
    let mut y = MARGE + 30;
    for royaume in royaumes {
        svg.push_str(&format!(
            "<text x=\"{MARGE}\" y=\"{y}\" font-family=\"serif\" font-size=\"13\" font-weight=\"bold\" fill=\"{COULEUR_TEXTE}\">{}</text>\n",
            echapper_xml(&royaume.nom)
        ));
        y += 8 + rendre_fiche(&mut svg, &royaume.fiche, MARGE, y + 8, largeur) + 8;
        y += rendre_piste(&mut svg, &royaume.piste, MARGE, y, largeur) + 16;
    }
    // Conversions fléchées entre pistes, avec le coût.
    for conversion in conversions {
        let Some((_, y_source)) = pistes_y.iter().find(|(nom, _)| *nom == conversion.source) else {
            continue;
        };
        let Some((_, y_cible)) = pistes_y.iter().find(|(nom, _)| *nom == conversion.cible) else {
            continue;
        };
        let y1 = y_source + HAUTEUR_PISTE / 2;
        let y2 = y_cible + HAUTEUR_PISTE / 2;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{y1}\" x2=\"{}\" y2=\"{y2}\" stroke=\"{COULEUR_CONVERSION}\" stroke-width=\"2\" stroke-dasharray=\"6 3\" marker-end=\"url(#fleche-conversion)\"/>\n",
            LARGEUR_VUE - MARGE - 40,
            LARGEUR_VUE - MARGE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" text-anchor=\"end\" fill=\"{COULEUR_CONVERSION}\">convert {} → {} (coût {})</text>\n",
            LARGEUR_VUE - MARGE - 48,
            (y1 + y2) / 2 + 4,
            echapper_xml(&conversion.source),
            echapper_xml(&conversion.cible),
            conversion.cout
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
