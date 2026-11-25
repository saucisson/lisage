//! Memory and quotient view (M4-5, liseuse doc § 5 « the pool in the background » and
//! « the semantic zoom »): pools grouped by plane with their GC
//! metrics, multiplicity badges (replicas), quotient classes
//! (forms × anchors), and naive/DD/Lisage comparison mode. Static SVG
//! rendering, like the other views.

use lisage_core::biplan::MetricsPool;

/// Title font size.
const SIZE_TITLE: u32 = 14;
/// Body font size.
const SIZE_BODY: u32 = 11;
/// Left margin in pixels.
const MARGIN_LEFT: u32 = 16;
/// Height of a pool line in pixels.
const HEIGHT_LINE_POOL: u32 = 18;
/// Height of a quotient class line in pixels.
const HEIGHT_LINE_CLASS: u32 = 18;
/// Height of a comparison bar in pixels.
const HEIGHT_BAR: u32 = 22;
/// Maximum width of a comparison bar in pixels.
const WIDTH_BAR_MAX: u32 = 400;
/// Color of the naive/DD bars.
const COLOR_BAR_NEUTRAL: &str = "#999";
/// Color of the Lisage bar (highlighted).
const COLOR_BAR_LISAGE: &str = "#2a7";
/// Color of the multiplicity badge.
const COLOR_BADGE: &str = "#c60";
/// Vertical offset between panels in pixels.
const SHIFT_PANEL: u32 = 24;
/// Height reserved for the title in pixels.
const HEIGHT_TITLE: u32 = 40;
/// Bottom margin in pixels.
const MARGIN_BAS: u32 = 20;
/// Width of the comparison panel beyond the max bar, in pixels.
const WIDTH_PANEL: u32 = 120;
/// Vertical position of the first panel in pixels.
const Y_FIRST_PANEL: u32 = 44;
/// Horizontal offset of the ×k badge in a class line, in pixels.
const SHIFT_BADGE: u32 = 220;
/// Inner height of a comparison bar in pixels.
const HEIGHT_BAR_INTERIEURE: u32 = 14;
/// Horizontal offset of the bars after the label, in pixels.
const SHIFT_BAR: u32 = 60;
/// Vertical offset of the bar rectangle under the text line, in pixels.
const SHIFT_TEXT_BAR: u32 = 12;
/// Horizontal offset of the value after the bar, in pixels.
const SHIFT_VALUE: u32 = 6;

/// Multiplicity badge: a handle and how many positional replicas
/// it represents (×k badge displayed when k > 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BadgeMultiplicity {
    /// Handle label (reduced controller/anchor/form triple).
    pub handle: String,
    /// Number of positional replicas represented.
    pub multiplicity: u64,
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
    pub multiplicity: u64,
}

/// Memory view of a pool: GC metrics and live nodes per plane.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ViewPool {
    /// Pool name (controller).
    pub name: String,
    /// Pool metrics (creations, shares, live, dead, GC,
    /// compactions, hit rate).
    pub metrics: MetricsPool,
    /// Live nodes of the main plane.
    pub live_principal: u64,
    /// Live nodes of the shadow plane.
    pub live_shadow: u64,
}

impl ClassQuotient {
    /// Multiplicity badge of the class (handle = « form @ anchor »).
    pub fn badge(&self) -> BadgeMultiplicity {
        BadgeMultiplicity {
            handle: format!("{} @ {}", self.form, self.anchor),
            multiplicity: self.multiplicity,
        }
    }
}

impl ViewPool {
    /// View of a pool from its metrics and the live nodes per plane.
    pub fn from_metrics(
        name: &str,
        metrics: MetricsPool,
        live_principal: u64,
        live_shadow: u64,
    ) -> Self {
        ViewPool {
            name: name.to_string(),
            metrics,
            live_principal,
            live_shadow,
        }
    }
}

/// Naive/DD/Lisage comparison: live nodes per representation mode.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComparisonView {
    /// Live nodes in naive enumeration (full explicitation).
    pub naive: u64,
    /// Live nodes in flat DD.
    pub dd: u64,
    /// Live nodes in Lisage (shared forms, O(1) + edge).
    pub lisage: u64,
}

/// Width of a comparison bar, proportional to the count (max =
/// `WIDTH_BAR_MAX`).
fn width_bar(count: u64, max: u64) -> u32 {
    if max == 0 {
        0
    } else {
        (count as u128 * WIDTH_BAR_MAX as u128 / max as u128) as u32
    }
}

/// Static SVG rendering of the memory and quotient view: pools panel
/// (GC metrics + live nodes per plane), quotient classes panel
/// (forms × anchors with ×k badges), naive/DD/Lisage comparison bars.
pub fn render_svg_view_memory(
    pools: &[ViewPool],
    classes: &[ClassQuotient],
    comparison: &ComparisonView,
) -> String {
    let height_pools = pools.len() as u32 * HEIGHT_LINE_POOL + SHIFT_PANEL;
    let height_classes = classes.len() as u32 * HEIGHT_LINE_CLASS + SHIFT_PANEL;
    let height_comparison = 3 * HEIGHT_BAR + SHIFT_PANEL;
    let height = HEIGHT_TITLE + height_pools + height_classes + height_comparison + MARGIN_BAS;
    let width = MARGIN_LEFT * 2 + WIDTH_BAR_MAX + WIDTH_PANEL;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGIN_LEFT}\" y=\"24\" font-family=\"serif\" font-size=\"{SIZE_TITLE}\">memory and quotient view</text>\n"
    ));
    // Pools panel.
    let mut y = Y_FIRST_PANEL;
    svg.push_str(&format!(
        "<text x=\"{MARGIN_LEFT}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">pools (main / shadow plane, GC)</text>\n"
    ));
    for pool in pools {
        y += HEIGHT_LINE_POOL;
        let metrics = &pool.metrics;
        svg.push_str(&format!(
            "<text x=\"{MARGIN_LEFT}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">{} : alive={}/{} dead={} gc={} compactions={} hit={}%</text>\n",
            pool.name,
            pool.live_principal,
            pool.live_shadow,
            metrics.dead,
            metrics.gc,
            metrics.compactions,
            metrics.rate_hit()
        ));
    }
    // Quotient classes panel.
    y += SHIFT_PANEL;
    svg.push_str(&format!(
        "<text x=\"{MARGIN_LEFT}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">quotient classes (forms × anchors)</text>\n"
    ));
    for class in classes {
        y += HEIGHT_LINE_CLASS;
        svg.push_str(&format!(
            "<text x=\"{MARGIN_LEFT}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">{} @ {}</text>\n",
            class.form, class.anchor
        ));
        let badge = class.badge();
        if badge.multiplicity > 1 {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\" fill=\"{COLOR_BADGE}\">×{}</text>\n",
                MARGIN_LEFT + SHIFT_BADGE,
                badge.multiplicity
            ));
        }
    }
    // Naive/DD/Lisage comparison bars.
    y += SHIFT_PANEL;
    svg.push_str(&format!(
        "<text x=\"{MARGIN_LEFT}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">comparison naive / DD / Lisage (live nodes)</text>\n"
    ));
    let max = comparison.naive.max(comparison.dd).max(comparison.lisage);
    for (label, count, color) in [
        ("naive", comparison.naive, COLOR_BAR_NEUTRAL),
        ("DD", comparison.dd, COLOR_BAR_NEUTRAL),
        ("Lisage", comparison.lisage, COLOR_BAR_LISAGE),
    ] {
        y += HEIGHT_BAR;
        let width = width_bar(count, max);
        svg.push_str(&format!(
            "<text x=\"{MARGIN_LEFT}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">{label}</text>\n"
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{HEIGHT_BAR_INTERIEURE}\" fill=\"{color}\"/>\n",
            MARGIN_LEFT + SHIFT_BAR,
            y - SHIFT_TEXT_BAR
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{SIZE_BODY}\">{count}</text>\n",
            MARGIN_LEFT + SHIFT_BAR + width + SHIFT_VALUE
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
