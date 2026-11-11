//! Mode publication (M5-5) : export de figures propres à la mise en page
//! article — SVG autonome ou PGF/TikZ, thème serif, **légende
//! auto-générée depuis la légende des canaux** (`CanalAttr`) : le même
//! fichier qui sert au débogage sert à la publication (doc liseuse § 5).

use lisage_core::observer::CanalAttr;
use lisage_core::{NodeId, Pool};

use crate::svg::{
    Couleurs, DEMI_LARGEUR, HAUTEUR_NIVEAU, MARGE_GAUCHE, MARGE_HAUT, dessiner_corps, echapper_xml,
    placer,
};

/// Thème de publication : serif, encre sombre sur fond blanc, couleurs de
/// canaux calées sur la légende de `CanalAttr` (hérité = bleu, synthétisé =
/// orange, mémoire = violet).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Theme {
    /// Police (serif pour la mise en page article).
    pub police: &'static str,
    /// Taille du titre en pixels.
    pub taille_titre: u32,
    /// Taille du texte des nœuds en pixels.
    pub taille_texte: u32,
    /// Taille de la légende en pixels.
    pub taille_legende: u32,
    /// Couleur d'encre (contours, texte).
    pub encre: &'static str,
    /// Couleur des arcs.
    pub arete: &'static str,
    /// Couleur des étiquettes d'arcs.
    pub etiquette: &'static str,
    /// Fond de la figure.
    pub fond: &'static str,
    /// Fond des nœuds.
    pub fond_noeud: &'static str,
    /// Fond des terminaux.
    pub fond_terminal: &'static str,
}

impl Theme {
    /// Thème de publication par défaut (serif, fond blanc, encre sombre).
    pub fn publication() -> Self {
        Theme {
            police: "serif",
            taille_titre: 16,
            taille_texte: 12,
            taille_legende: 11,
            encre: "#1a1a1a",
            arete: "#444444",
            etiquette: "#666666",
            fond: "#ffffff",
            fond_noeud: "#ffffff",
            fond_terminal: "#f4f4f4",
        }
    }

    /// Palette de rendu du diagramme pour ce thème.
    fn couleurs(&self) -> Couleurs {
        Couleurs {
            encre: self.encre,
            arete: self.arete,
            etiquette: self.etiquette,
            fond_noeud: self.fond_noeud,
            fond_terminal: self.fond_terminal,
        }
    }
}

/// Légende des canaux : source de vérité unique (canal, étiquette, couleur),
/// rendue telle quelle dans la légende auto-générée.
pub const LEGENDE_CANAUX: &[(CanalAttr, &str, &str)] = &[
    (CanalAttr::Herite, "hérité (φ_disp)", "#1f6feb"),
    (CanalAttr::Synthétise, "synthétisé (φ_aggr)", "#d29922"),
    (CanalAttr::Memoire, "mémoire de niveau", "#8250df"),
];

/// Hauteur réservée à la légende en pixels (une ligne par canal).
const HAUTEUR_LEGENDE: u32 = 18;
/// Largeur de la colonne de légende en pixels.
const LARGEUR_LEGENDE: u32 = 220;
/// Taille du pavé de couleur d'un canal en pixels.
const TAILLE_PAVE_LEGENDE: u32 = 12;
/// Décalage horizontal entre le pavé et son étiquette en pixels.
const DECALAGE_ETIQUETTE_LEGENDE: u32 = 18;
/// Décalage vertical du pavé par rapport à la ligne de texte en pixels.
const DECALAGE_PAVE_LEGENDE: u32 = 10;
/// Marge entre le diagramme et la colonne de légende en pixels.
const MARGE_LEGENDE: u32 = 20;
/// Ligne de base du titre en pixels.
const BASELINE_TITRE: u32 = 26;
/// Marge basse de la figure en pixels.
const MARGE_BAS: u32 = 20;

/// Rend la légende des canaux en fragment SVG (un pavé coloré + une étiquette
/// par canal, auto-générée depuis [`LEGENDE_CANAUX`]).
pub fn rendre_legende_svg(theme: &Theme, x: u32, y: u32) -> String {
    let mut svg = String::new();
    for (index, (_canal, etiquette, couleur)) in LEGENDE_CANAUX.iter().enumerate() {
        let ligne_y = y + index as u32 * HAUTEUR_LEGENDE;
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{}\" width=\"{TAILLE_PAVE_LEGENDE}\" height=\"{TAILLE_PAVE_LEGENDE}\" fill=\"{couleur}\"/>\n",
            ligne_y - DECALAGE_PAVE_LEGENDE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
            x + DECALAGE_ETIQUETTE_LEGENDE,
            ligne_y,
            theme.police,
            theme.taille_legende,
            theme.encre,
            echapper_xml(etiquette)
        ));
    }
    svg
}

/// Rend le sous-diagramme atteignable depuis `racine` en **SVG autonome de
/// publication** : fond blanc, titre en serif, diagramme au thème donné, et
/// légende des canaux auto-générée si `avec_legende`.
pub fn rendre_svg_publication<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    titre: &str,
    theme: &Theme,
    avec_legende: bool,
) -> String {
    let (placement, nombre_niveaux) = placer(pool, racine);
    let largeur_diagramme =
        MARGE_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_LARGEUR;
    let largeur_legende = if avec_legende { LARGEUR_LEGENDE } else { 0 };
    let largeur = largeur_diagramme + largeur_legende;
    let hauteur_legende = LEGENDE_CANAUX.len() as u32 * HAUTEUR_LEGENDE;
    let hauteur = (MARGE_HAUT + (nombre_niveaux + 1) * HAUTEUR_NIVEAU + MARGE_BAS)
        .max(hauteur_legende + MARGE_HAUT);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{largeur}\" height=\"{hauteur}\" fill=\"{}\"/>\n",
        theme.fond
    ));
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{BASELINE_TITRE}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
        theme.police,
        theme.taille_titre,
        theme.encre,
        echapper_xml(titre)
    ));
    svg.push_str(&dessiner_corps(pool, racine, &placement, &theme.couleurs()));
    if avec_legende {
        svg.push_str(&rendre_legende_svg(
            theme,
            largeur_diagramme + MARGE_LEGENDE,
            MARGE_HAUT + DECALAGE_PAVE_LEGENDE,
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

/// Échelle de conversion pixels → centimètres pour le PGF (1 px = 0,05 cm).
const ECHELLE_PGF_CM_PAR_PIXEL: f64 = 0.05;

/// Rend le sous-diagramme atteignable depuis `racine` en **PGF/TikZ
/// autonome** (document `standalone`), pour inclusion dans un article LaTeX.
pub fn rendre_pgf<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    titre: &str,
) -> String {
    let (placement, _nombre_niveaux) = placer(pool, racine);
    let mut pgf = String::new();
    pgf.push_str(
        "\\documentclass[tikz,border=5pt]{standalone}\n\\usepackage{tikz}\n\\begin{document}\n",
    );
    pgf.push_str("\\begin{tikzpicture}[every node/.style={font=\\ttfamily\\small}]\n");
    pgf.push_str(&format!(
        "\\node[font=\\rmfamily\\bfseries] at (0,0) {{{}}};\n",
        echapper_pgf(titre)
    ));
    // Nœuds.
    for (noeud, position) in &placement {
        let x = position.x as f64 * ECHELLE_PGF_CM_PAR_PIXEL;
        let y = -(position.y as f64 * ECHELLE_PGF_CM_PAR_PIXEL);
        match pool.resolve(*noeud) {
            Ok(lisage_core::Form::Terminal(payload)) => {
                let label = if *payload { "$\\top$" } else { "$\\bot$" };
                pgf.push_str(&format!(
                    "\\node[draw, rectangle, minimum width=1.2cm] (n{}) at ({x:.2},{y:.2}) {{{label}}};\n",
                    noeud.0
                ));
            }
            Ok(lisage_core::Form::Node { level, .. }) => {
                pgf.push_str(&format!(
                    "\\node[draw, rectangle, minimum width=1.2cm] (n{}) at ({x:.2},{y:.2}) {{$\\ell{}$}};\n",
                    noeud.0,
                    level.0
                ));
            }
            Err(_) => {}
        }
    }
    // Arcs.
    let mut pile = vec![racine];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(noeud) = pile.pop() {
        if !deja_vu.insert(noeud) {
            continue;
        }
        if let Ok(lisage_core::Form::Node { arcs, .. }) = pool.resolve(noeud) {
            for (cle, cible) in arcs {
                pgf.push_str(&format!(
                    "\\draw[->] (n{}) -- (n{}) node[midway, above, font=\\ttfamily\\scriptsize] {{{}}};\n",
                    noeud.0,
                    cible.0,
                    echapper_pgf(&cle.to_string())
                ));
                pile.push(*cible);
            }
        }
    }
    pgf.push_str("\\end{tikzpicture}\n\\end{document}\n");
    pgf
}

/// Échappement minimal pour TikZ (les caractères spéciaux LaTeX).
fn echapper_pgf(texte: &str) -> String {
    // Le backslash passe par un placeholder : échappé en premier il
    // protégerait mal ses propres accolades, en dernier il ré-échapperait
    // les backslashes insérés par les autres règles.
    const PLACEHOLDER_BASCUL: char = '\u{0}';
    texte
        .replace('\\', PLACEHOLDER_BASCUL.to_string().as_str())
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('$', "\\$")
        .replace('&', "\\&")
        .replace('#', "\\#")
        .replace('%', "\\%")
        .replace('_', "\\_")
        .replace('^', "\\textasciicircum{}")
        .replace(PLACEHOLDER_BASCUL, "\\textbackslash{}")
}
