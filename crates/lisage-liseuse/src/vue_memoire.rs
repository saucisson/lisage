//! Memory and quotient view (M4-5, liseuse doc § 5 « the pool in the background » and
//! « the semantic zoom »): pools grouped by plane with their GC
//! metrics, multiplicity badges (replicas), quotient classes
//! (forms × anchors), and naive/DD/Lisage comparison mode. Static SVG
//! rendering, like the other views.

use lisage_core::biplan::MetriquesPool;

/// Title font size.
const TAILLE_TITRE: u32 = 14;
/// Body font size.
const TAILLE_CORPS: u32 = 11;
/// Left margin in pixels.
const MARGE_GAUCHE: u32 = 16;
/// Height of a pool line in pixels.
const HAUTEUR_LIGNE_POOL: u32 = 18;
/// Height of a quotient class line in pixels.
const HAUTEUR_LIGNE_CLASSE: u32 = 18;
/// Height of a comparison bar in pixels.
const HAUTEUR_BARRE: u32 = 22;
/// Maximum width of a comparison bar in pixels.
const LARGEUR_BARRE_MAX: u32 = 400;
/// Color of the naive/DD bars.
const COULEUR_BARRE_NEUTRE: &str = "#999";
/// Color of the Lisage bar (highlighted).
const COULEUR_BARRE_LISAGE: &str = "#2a7";
/// Color of the multiplicity badge.
const COULEUR_BADGE: &str = "#c60";
/// Vertical offset between panels in pixels.
const DECALAGE_PANNEAU: u32 = 24;
/// Height reserved for the title in pixels.
const HAUTEUR_TITRE: u32 = 40;
/// Bottom margin in pixels.
const MARGE_BAS: u32 = 20;
/// Width of the comparison panel beyond the max bar, in pixels.
const LARGEUR_PANNEAU: u32 = 120;
/// Vertical position of the first panel in pixels.
const Y_PREMIER_PANNEAU: u32 = 44;
/// Horizontal offset of the ×k badge in a class line, in pixels.
const DECALAGE_BADGE: u32 = 220;
/// Inner height of a comparison bar in pixels.
const HAUTEUR_BARRE_INTERIEURE: u32 = 14;
/// Horizontal offset of the bars after the label, in pixels.
const DECALAGE_BARRE: u32 = 60;
/// Vertical offset of the bar rectangle under the text line, in pixels.
const DECALAGE_TEXTE_BARRE: u32 = 12;
/// Horizontal offset of the value after the bar, in pixels.
const DECALAGE_VALEUR: u32 = 6;

/// Multiplicity badge: a handle and how many positional replicas
/// it represents (×k badge displayed when k > 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BadgeMultiplicite {
    /// Handle label (reduced controller/anchor/form triple).
    pub handle: String,
    /// Number of positional replicas represented.
    pub multiplicite: u64,
}

/// Quotient class: a form under an anchor, with its multiplicity
/// (forms × anchors — C1: same form under two anchors = two classes).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClasseQuotient {
    /// Form label.
    pub forme: String,
    /// Anchor label.
    pub ancre: String,
    /// Number of replicas in the class.
    pub multiplicite: u64,
}

/// Memory view of a pool: GC metrics and live nodes per plane.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VuePool {
    /// Pool name (controller).
    pub nom: String,
    /// Pool metrics (creations, shares, live, dead, GC,
    /// compactions, hit rate).
    pub metriques: MetriquesPool,
    /// Live nodes of the main plane.
    pub vivants_principal: u64,
    /// Live nodes of the shadow plane.
    pub vivants_ombre: u64,
}

impl ClasseQuotient {
    /// Multiplicity badge of the class (handle = « form @ anchor »).
    pub fn badge(&self) -> BadgeMultiplicite {
        BadgeMultiplicite {
            handle: format!("{} @ {}", self.forme, self.ancre),
            multiplicite: self.multiplicite,
        }
    }
}

impl VuePool {
    /// View of a pool from its metrics and the live nodes per plane.
    pub fn depuis_metriques(
        nom: &str,
        metriques: MetriquesPool,
        vivants_principal: u64,
        vivants_ombre: u64,
    ) -> Self {
        VuePool {
            nom: nom.to_string(),
            metriques,
            vivants_principal,
            vivants_ombre,
        }
    }
}

/// Naive/DD/Lisage comparison: live nodes per representation mode.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComparaisonVue {
    /// Live nodes in naive enumeration (full explicitation).
    pub naif: u64,
    /// Live nodes in flat DD.
    pub dd: u64,
    /// Live nodes in Lisage (shared forms, O(1) + edge).
    pub lisage: u64,
}

/// Width of a comparison bar, proportional to the count (max =
/// `LARGEUR_BARRE_MAX`).
fn largeur_barre(compte: u64, max: u64) -> u32 {
    if max == 0 {
        0
    } else {
        (compte as u128 * LARGEUR_BARRE_MAX as u128 / max as u128) as u32
    }
}

/// Static SVG rendering of the memory and quotient view: pools panel
/// (GC metrics + live nodes per plane), quotient classes panel
/// (forms × anchors with ×k badges), naive/DD/Lisage comparison bars.
pub fn rendre_svg_vue_memoire(
    pools: &[VuePool],
    classes: &[ClasseQuotient],
    comparaison: &ComparaisonVue,
) -> String {
    let hauteur_pools = pools.len() as u32 * HAUTEUR_LIGNE_POOL + DECALAGE_PANNEAU;
    let hauteur_classes = classes.len() as u32 * HAUTEUR_LIGNE_CLASSE + DECALAGE_PANNEAU;
    let hauteur_comparaison = 3 * HAUTEUR_BARRE + DECALAGE_PANNEAU;
    let hauteur = HAUTEUR_TITRE + hauteur_pools + hauteur_classes + hauteur_comparaison + MARGE_BAS;
    let largeur = MARGE_GAUCHE * 2 + LARGEUR_BARRE_MAX + LARGEUR_PANNEAU;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"{TAILLE_TITRE}\">memory and quotient view</text>\n"
    ));
    // Pools panel.
    let mut y = Y_PREMIER_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">pools (main / shadow plane, GC)</text>\n"
    ));
    for pool in pools {
        y += HAUTEUR_LIGNE_POOL;
        let metriques = &pool.metriques;
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{} : alive={}/{} dead={} gc={} compactions={} hit={}%</text>\n",
            pool.nom,
            pool.vivants_principal,
            pool.vivants_ombre,
            metriques.morts,
            metriques.gc,
            metriques.compactions,
            metriques.taux_hit()
        ));
    }
    // Quotient classes panel.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">quotient classes (forms × anchors)</text>\n"
    ));
    for classe in classes {
        y += HAUTEUR_LIGNE_CLASSE;
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{} @ {}</text>\n",
            classe.forme, classe.ancre
        ));
        let badge = classe.badge();
        if badge.multiplicite > 1 {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\" fill=\"{COULEUR_BADGE}\">×{}</text>\n",
                MARGE_GAUCHE + DECALAGE_BADGE,
                badge.multiplicite
            ));
        }
    }
    // Naive/DD/Lisage comparison bars.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">comparison naive / DD / Lisage (live nodes)</text>\n"
    ));
    let max = comparaison.naif.max(comparaison.dd).max(comparaison.lisage);
    for (libelle, compte, couleur) in [
        ("naive", comparaison.naif, COULEUR_BARRE_NEUTRE),
        ("DD", comparaison.dd, COULEUR_BARRE_NEUTRE),
        ("Lisage", comparaison.lisage, COULEUR_BARRE_LISAGE),
    ] {
        y += HAUTEUR_BARRE;
        let largeur = largeur_barre(compte, max);
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{libelle}</text>\n"
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{largeur}\" height=\"{HAUTEUR_BARRE_INTERIEURE}\" fill=\"{couleur}\"/>\n",
            MARGE_GAUCHE + DECALAGE_BARRE,
            y - DECALAGE_TEXTE_BARRE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{compte}</text>\n",
            MARGE_GAUCHE + DECALAGE_BARRE + largeur + DECALAGE_VALEUR
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
