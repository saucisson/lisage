//! Memory and quotient view (M4-5, liseuse doc § 5 « the pool in the background » and
//! « the semantic zoom »): pools grouped by plane with their GC
//! metrics, multiplicity badges (replicas), quotient classes
//! (forms × anchors), and naive/DD/Lisage comparison mode. Static SVG
//! rendering, like the other views.

use lisage_core::biplan::MetriquesPool;

/// Title font size.
const SIZE_TITRE: u32 = 14;
/// Body font size.
const SIZE_CORPS: u32 = 11;
/// Left margin in pixels.
const MARGE_GAUCHE: u32 = 16;
/// Height of a pool line in pixels.
const HEIGHT_LIGNE_POOL: u32 = 18;
/// Height of a quotient class line in pixels.
const HEIGHT_LIGNE_CLASS: u32 = 18;
/// Height of a comparison bar in pixels.
const HEIGHT_BARRE: u32 = 22;
/// Maximum width of a comparison bar in pixels.
const WIDTH_BARRE_MAX: u32 = 400;
/// Color of the naive/DD bars.
const COULEUR_BARRE_NEUTRE: &str = "#999";
/// Color of the Lisage bar (highlighted).
const COULEUR_BARRE_LISAGE: &str = "#2a7";
/// Color of the multiplicity badge.
const COULEUR_BADGE: &str = "#c60";
/// Vertical offset between panels in pixels.
const DECALAGE_PANNEAU: u32 = 24;
/// Height reserved for the title in pixels.
const HEIGHT_TITRE: u32 = 40;
/// Bottom margin in pixels.
const MARGE_BAS: u32 = 20;
/// Width of the comparison panel beyond the max bar, in pixels.
const WIDTH_PANNEAU: u32 = 120;
/// Vertical position of the first panel in pixels.
const Y_FIRST_PANNEAU: u32 = 44;
/// Horizontal offset of the ×k badge in a class line, in pixels.
const DECALAGE_BADGE: u32 = 220;
/// Inner height of a comparison bar in pixels.
const HEIGHT_BARRE_INTERIEURE: u32 = 14;
/// Horizontal offset of the bars after the label, in pixels.
const DECALAGE_BARRE: u32 = 60;
/// Vertical offset of the bar rectangle under the text line, in pixels.
const DECALAGE_TEXTE_BARRE: u32 = 12;
/// Horizontal offset of the value after the bar, in pixels.
const DECALAGE_VALUE: u32 = 6;

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
pub struct ClassQuotient {
    /// Form label.
    pub form: String,
    /// Anchor label.
    pub anchor: String,
    /// Number of replicas in the class.
    pub multiplicite: u64,
}

/// Memory view of a pool: GC metrics and live nodes per plane.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ViewPool {
    /// Pool name (controller).
    pub name: String,
    /// Pool metrics (creations, shares, live, dead, GC,
    /// compactions, hit rate).
    pub metriques: MetriquesPool,
    /// Live nodes of the main plane.
    pub live_principal: u64,
    /// Live nodes of the shadow plane.
    pub live_ombre: u64,
}

impl ClassQuotient {
    /// Multiplicity badge of the class (handle = « form @ anchor »).
    pub fn badge(&self) -> BadgeMultiplicite {
        BadgeMultiplicite {
            handle: format!("{} @ {}", self.form, self.anchor),
            multiplicite: self.multiplicite,
        }
    }
}

impl ViewPool {
    /// View of a pool from its metrics and the live nodes per plane.
    pub fn depuis_metriques(
        name: &str,
        metriques: MetriquesPool,
        live_principal: u64,
        live_ombre: u64,
    ) -> Self {
        ViewPool {
            name: name.to_string(),
            metriques,
            live_principal,
            live_ombre,
        }
    }
}

/// Naive/DD/Lisage comparison: live nodes per representation mode.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComparaisonView {
    /// Live nodes in naive enumeration (full explicitation).
    pub naif: u64,
    /// Live nodes in flat DD.
    pub dd: u64,
    /// Live nodes in Lisage (shared forms, O(1) + edge).
    pub lisage: u64,
}

/// Width of a comparison bar, proportional to the count (max =
/// `WIDTH_BARRE_MAX`).
fn width_barre(count: u64, max: u64) -> u32 {
    if max == 0 {
        0
    } else {
        (count as u128 * WIDTH_BARRE_MAX as u128 / max as u128) as u32
    }
}

/// Static SVG rendering of the memory and quotient view: pools panel
/// (GC metrics + live nodes per plane), quotient classes panel
/// (forms × anchors with ×k badges), naive/DD/Lisage comparison bars.
pub fn rendre_svg_view_memory(
    pools: &[ViewPool],
    classes: &[ClassQuotient],
    comparaison: &ComparaisonView,
) -> String {
    let height_pools = pools.len() as u32 * HEIGHT_LIGNE_POOL + DECALAGE_PANNEAU;
    let height_classes = classes.len() as u32 * HEIGHT_LIGNE_CLASS + DECALAGE_PANNEAU;
    let height_comparaison = 3 * HEIGHT_BARRE + DECALAGE_PANNEAU;
    let height = HEIGHT_TITRE + height_pools + height_classes + height_comparaison + MARGE_BAS;
    let width = MARGE_GAUCHE * 2 + WIDTH_BARRE_MAX + WIDTH_PANNEAU;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"{SIZE_TITRE}\">memory and quotient view</text>\n"
    ));
    // Pools panel.
    let mut y = Y_FIRST_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">pools (main / shadow plane, GC)</text>\n"
    ));
    for pool in pools {
        y += HEIGHT_LIGNE_POOL;
        let metriques = &pool.metriques;
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{} : alive={}/{} dead={} gc={} compactions={} hit={}%</text>\n",
            pool.name,
            pool.live_principal,
            pool.live_ombre,
            metriques.dead,
            metriques.gc,
            metriques.compactions,
            metriques.rate_hit()
        ));
    }
    // Quotient classes panel.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">quotient classes (forms × anchors)</text>\n"
    ));
    for class in classes {
        y += HEIGHT_LIGNE_CLASS;
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{} @ {}</text>\n",
            class.form, class.anchor
        ));
        let badge = class.badge();
        if badge.multiplicite > 1 {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\" fill=\"{COULEUR_BADGE}\">×{}</text>\n",
                MARGE_GAUCHE + DECALAGE_BADGE,
                badge.multiplicite
            ));
        }
    }
    // Naive/DD/Lisage comparison bars.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">comparison naive / DD / Lisage (live nodes)</text>\n"
    ));
    let max = comparaison.naif.max(comparaison.dd).max(comparaison.lisage);
    for (libelle, count, couleur) in [
        ("naive", comparaison.naif, COULEUR_BARRE_NEUTRE),
        ("DD", comparaison.dd, COULEUR_BARRE_NEUTRE),
        ("Lisage", comparaison.lisage, COULEUR_BARRE_LISAGE),
    ] {
        y += HEIGHT_BARRE;
        let width = width_barre(count, max);
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{libelle}</text>\n"
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{HEIGHT_BARRE_INTERIEURE}\" fill=\"{couleur}\"/>\n",
            MARGE_GAUCHE + DECALAGE_BARRE,
            y - DECALAGE_TEXTE_BARRE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{count}</text>\n",
            MARGE_GAUCHE + DECALAGE_BARRE + width + DECALAGE_VALUE
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
