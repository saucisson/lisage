//! Memory and quotient view (M4-5, reader doc § 5 « the pool in the
//! background » and « the semantic zoom ») : pools grouped by plane with
//! their GC metrics, multiplicity badges (replicas), quotient classes
//! (forms × anchors), and naive/DD/Lisage comparison mode. Static SVG
//! rendering, like the other views.

use lisage_core::biplan::MetricsPool;

/// Title font size.
const SIZE_TITRE: u32 = 14;
/// Body font size.
const SIZE_CORPS: u32 = 11;
/// Left margin in pixels.
const MARGIN_GAUCHE: u32 = 16;
/// Height of a pool line in pixels.
const HEIGHT_LINE_POOL: u32 = 18;
/// Height of a quotient class line in pixels.
const HEIGHT_LINE_CLASSE: u32 = 18;
/// Height of a comparison bar in pixels.
const HEIGHT_BARRE: u32 = 22;
/// Maximum width of a comparison bar in pixels.
const WIDTH_BARRE_MAX: u32 = 400;
/// Color of the naive/DD bars.
const COLOR_BARRE_NEUTRE: &str = "#999";
/// Color of the Lisage bar (highlighted).
const COLOR_BARRE_LISAGE: &str = "#2a7";
/// Color of the multiplicity badge.
const COLOR_BADGE: &str = "#c60";
/// Vertical offset between panels in pixels.
const DECALAGE_PANNEAU: u32 = 24;
/// Height reserved for the title in pixels.
const HEIGHT_TITRE: u32 = 40;
/// Bottom margin in pixels.
const MARGIN_BAS: u32 = 20;
/// Width of the comparison panel beyond the max bar, in pixels.
const WIDTH_PANNEAU: u32 = 120;
/// Vertical position of the first panel in pixels.
const Y_PREMIER_PANNEAU: u32 = 44;
/// Horizontal offset of the ×k badge in a class line, in pixels.
const DECALAGE_BADGE: u32 = 220;
/// Inner height of a comparison bar in pixels.
const HEIGHT_BARRE_INTERIEURE: u32 = 14;
/// Horizontal offset of the bars after the label, in pixels.
const DECALAGE_BARRE: u32 = 60;
/// Vertical offset of the bar rectangle below the text line, in pixels.
const DECALAGE_TEXTE_BARRE: u32 = 12;
/// Horizontal offset of the value after the bar, in pixels.
const DECALAGE_VALEUR: u32 = 6;

/// Multiplicity badge: a handle and how many positional replicas it
/// represents (×k badge displayed when k > 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BadgeMultiplicite {
    /// Label of the handle (reduced controller/anchor/form triple).
    pub handle: String,
    /// Number of positional replicas represented.
    pub multiplicite: u64,
}

/// Quotient class: a form under an anchor, with its multiplicity
/// (forms × anchors — C1: same form under two anchors = two classes).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClasseQuotient {
    /// Label of the form.
    pub form: String,
    /// Label of the anchor.
    pub anchor: String,
    /// Number of replicas in the class.
    pub multiplicite: u64,
}

/// Memory view of a pool: GC metrics and live nodes per plane.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ViewPool {
    /// Name of the pool (controller).
    pub name: String,
    /// Pool metrics (creations, shares, live, dead, GC,
    /// compactions, hit rate).
    pub metrics: MetricsPool,
    /// Live count of the main plane.
    pub live_main: u64,
    /// Live count of the shadow plane.
    pub live_shadow: u64,
}

impl ClasseQuotient {
    /// Multiplicity badge of the class (handle = « form @ anchor »).
    pub fn badge(&self) -> BadgeMultiplicite {
        BadgeMultiplicite {
            handle: format!("{} @ {}", self.form, self.anchor),
            multiplicite: self.multiplicite,
        }
    }
}

impl ViewPool {
    /// View of a pool from its metrics and the live counts per plane.
    pub fn since_metrics(
        name: &str,
        metrics: MetricsPool,
        live_main: u64,
        live_shadow: u64,
    ) -> Self {
        ViewPool {
            name: name.to_string(),
            metrics,
            live_main,
            live_shadow,
        }
    }
}

/// Naive/DD/Lisage comparison: live nodes per representation mode.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComparisonView {
    /// Live nodes in naive enumeration (full expansion).
    pub naive: u64,
    /// Live nodes in flat DD.
    pub dd: u64,
    /// Live nodes in Lisage (shared forms, O(1) + border).
    pub lisage: u64,
}

/// Width of a comparison bar, proportional to the count (max =
/// `WIDTH_BARRE_MAX`).
fn width_bar(count: u64, max: u64) -> u32 {
    if max == 0 {
        0
    } else {
        (count as u128 * WIDTH_BARRE_MAX as u128 / max as u128) as u32
    }
}

/// Static SVG rendering of the memory and quotient view: pools panel
/// (GC metrics + live per plane), quotient classes panel
/// (forms × anchors with ×k badges), naive/DD/Lisage comparison bars.
pub fn render_svg_view_memory(
    pools: &[ViewPool],
    classes: &[ClasseQuotient],
    comparison: &ComparisonView,
) -> String {
    let height_pools = pools.len() as u32 * HEIGHT_LINE_POOL + DECALAGE_PANNEAU;
    let height_classes = classes.len() as u32 * HEIGHT_LINE_CLASSE + DECALAGE_PANNEAU;
    let height_comparison = 3 * HEIGHT_BARRE + DECALAGE_PANNEAU;
    let height = HEIGHT_TITRE + height_pools + height_classes + height_comparison + MARGIN_BAS;
    let width = MARGIN_GAUCHE * 2 + WIDTH_BARRE_MAX + WIDTH_PANNEAU;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGIN_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"{SIZE_TITRE}\">memory and quotient view</text>\n"
    ));
    // Pools panel.
    let mut y = Y_PREMIER_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGIN_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">pools (main / shadow plane, GC)</text>\n"
    ));
    for pool in pools {
        y += HEIGHT_LINE_POOL;
        let metrics = &pool.metrics;
        svg.push_str(&format!(
            "<text x=\"{MARGIN_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{} : live={}/{} dead={} gc={} compactions={} hit={}%</text>\n",
            pool.name,
            pool.live_main,
            pool.live_shadow,
            metrics.dead,
            metrics.gc,
            metrics.compactions,
            metrics.rate_hit()
        ));
    }
    // Quotient classes panel.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGIN_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">quotient classes (forms × anchors)</text>\n"
    ));
    for classe in classes {
        y += HEIGHT_LINE_CLASSE;
        svg.push_str(&format!(
            "<text x=\"{MARGIN_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{} @ {}</text>\n",
            classe.form, classe.anchor
        ));
        let badge = classe.badge();
        if badge.multiplicite > 1 {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\" fill=\"{COLOR_BADGE}\">×{}</text>\n",
                MARGIN_GAUCHE + DECALAGE_BADGE,
                badge.multiplicite
            ));
        }
    }
    // Naive/DD/Lisage comparison bars.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGIN_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">naive / DD / Lisage comparison (live nodes)</text>\n"
    ));
    let max = comparison.naive.max(comparison.dd).max(comparison.lisage);
    for (libelle, count, color) in [
        ("naive", comparison.naive, COLOR_BARRE_NEUTRE),
        ("DD", comparison.dd, COLOR_BARRE_NEUTRE),
        ("Lisage", comparison.lisage, COLOR_BARRE_LISAGE),
    ] {
        y += HEIGHT_BARRE;
        let width = width_bar(count, max);
        svg.push_str(&format!(
            "<text x=\"{MARGIN_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{libelle}</text>\n"
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{HEIGHT_BARRE_INTERIEURE}\" fill=\"{color}\"/>\n",
            MARGIN_GAUCHE + DECALAGE_BARRE,
            y - DECALAGE_TEXTE_BARRE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_CORPS}\">{count}</text>\n",
            MARGIN_GAUCHE + DECALAGE_BARRE + width + DECALAGE_VALEUR
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
