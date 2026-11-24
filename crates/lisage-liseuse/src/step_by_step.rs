//! Liseuse v1 — **step by step** view (M2-6, liseuse doc § 6): scrubber on the
//! trace, colored channels (inherited blue / synthesized orange / memory purple),
//! `MemoHit`/`MemoMiss`, saturation rounds, and before/after differential
//! with **ghosts** (absorbed nodes rendered as watermark).
//!
//! The scrubber is a position `k` on the trace; the state at `k` is the
//! replay of events `0..k`. The static SVG rendering at a given position
//! embeds the event timeline — the interactive JS version is out of scope
//! (doc § 7: the static SVG output remains complete).

use std::collections::{BTreeMap, BTreeSet};

use lisage_core::observer::{ChannelAttr, TraceEvent, TraceVal};
use lisage_core::{Form, NodeId, Pool};

use crate::svg::{
    HALF_WIDTH, HEIGHT_LEVEL, MARGIN_HAUT, MARGIN_LEFT, Placement, WIDTH_NODE, escape_xml, placer,
};

/// Color of the inherited channel (blue).
const COLOR_INHERITED: &str = "#2980b9";
/// Color of the synthesized channel (orange).
const COLOR_SYNTHESIZED: &str = "#e67e22";
/// Color of the level memory (purple).
const COLOR_MEMORY: &str = "#8e44ad";
/// Color of ghosts (gray watermark).
const COLOR_GHOST: &str = "#bbb";
/// Default color of nodes without attribute (M2-6).
const COLOR_DEFAULT: &str = "#333";
/// Color of a `MemoHit` on the timeline (green).
const COLOR_MEMO_HIT: &str = "#27ae60";
/// Color of a `MemoMiss` on the timeline (red).
const COLOR_MEMO_MISS: &str = "#c0392b";
/// Color of the other events on the timeline (neutral gray).
const COLOR_OTHER: &str = "#999";
/// Color of the diagram arcs.
const COLOR_ARC: &str = "#555";
/// Color of the arc labels.
const COLOR_LABEL: &str = "#777";
/// Color of the position cursor on the timeline (black).
const COLOR_CURSOR: &str = "#000";
/// Background of terminals.
const COLOR_BACKGROUND_TERMINAL: &str = "#eee";
/// Background of internal nodes.
const COLOR_BACKGROUND_NODE: &str = "#fff";
/// Height of the timeline in pixels.
const HEIGHT_TIMELINE: u32 = 46;

/// State of the trace at a scrubber position: replay of events
/// `0..k`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateTrace {
    /// Nodes seen (created or shared) up to the position.
    pub nodes: BTreeSet<NodeId>,
    /// Attributes seen `(channel, node, value)`, in emission order.
    pub attributes: Vec<(ChannelAttr, NodeId, TraceVal)>,
    /// Number of `MemoHit` up to the position.
    pub memo_hits: usize,
    /// Number of `MemoMiss` up to the position.
    pub memo_misses: usize,
    /// Number of the last saturation round seen (0 = none).
    pub rounds: u32,
}

/// Before/after differential between two trace states: `ghosts` =
/// nodes present before but not after (rendered as watermark).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffTrace {
    /// Nodes present after but not before.
    pub ajoutes: BTreeSet<NodeId>,
    /// Nodes present before but not after (ghosts).
    pub ghosts: BTreeSet<NodeId>,
}

/// Differential between two states: additions and ghosts.
pub fn diff(before: &StateTrace, after: &StateTrace) -> DiffTrace {
    DiffTrace {
        ajoutes: after.nodes.difference(&before.nodes).copied().collect(),
        ghosts: before.nodes.difference(&after.nodes).copied().collect(),
    }
}

/// Scrubber: position on an event trace + state replay.
pub struct Scrubber {
    events: Vec<TraceEvent>,
    position: usize,
}

impl Scrubber {
    /// New scrubber on the given trace, positioned at the start.
    pub fn new(events: Vec<TraceEvent>) -> Self {
        Scrubber {
            events,
            position: 0,
        }
    }

    /// Number of events in the trace.
    pub fn length(&self) -> usize {
        self.events.len()
    }

    /// Current position of the scrubber.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Moves the scrubber to `position` (clamped to the trace length).
    pub fn round_a(&mut self, position: usize) {
        self.position = position.min(self.events.len());
    }

    /// Replay of events `0..position`: state of the trace at the current
    /// position.
    pub fn state(&self) -> StateTrace {
        let mut state = StateTrace::default();
        for event in &self.events[..self.position] {
            match event {
                TraceEvent::NodeCreated { node, .. } | TraceEvent::NodeShared { node, .. } => {
                    state.nodes.insert(*node);
                }
                TraceEvent::Attr {
                    channel,
                    node,
                    value,
                    ..
                } => {
                    state.attributes.push((*channel, *node, value.clone()));
                }
                TraceEvent::MemoHit { .. } => state.memo_hits += 1,
                TraceEvent::MemoMiss { .. } => state.memo_misses += 1,
                TraceEvent::Round { number, .. } => state.rounds = state.rounds.max(*number),
                TraceEvent::ReductionApplied { .. }
                | TraceEvent::DiagramBuilt { .. }
                | TraceEvent::Gc { .. }
                | TraceEvent::Histogram { .. }
                | TraceEvent::SpanOpen { .. }
                | TraceEvent::SpanClose { .. }
                | TraceEvent::OpStart { .. }
                | TraceEvent::OpEnd { .. }
                | TraceEvent::VisitEntry { .. }
                | TraceEvent::VisitExit { .. }
                | TraceEvent::MemoConsult { .. }
                | TraceEvent::MemoInsert { .. }
                | TraceEvent::MemoryWrite { .. }
                | TraceEvent::StructureChange { .. }
                | TraceEvent::LocalFire { .. } => {}
            }
        }
        state
    }

    /// Differential between the current position and position `other`.
    pub fn diff_to(&self, other: usize) -> DiffTrace {
        let mut other_scrubber = Scrubber::new(self.events.clone());
        other_scrubber.round_a(other);
        diff(&self.state(), &other_scrubber.state())
    }
}

/// Color of an attribute channel.
fn color_channel(channel: ChannelAttr) -> &'static str {
    match channel {
        ChannelAttr::Inherited => COLOR_INHERITED,
        ChannelAttr::Synthesized => COLOR_SYNTHESIZED,
        ChannelAttr::Memory => COLOR_MEMORY,
    }
}

/// Border color of a node according to the **last** attribute channel seen
/// on this node (memory > synthesized > inherited > default).
fn color_node(state: &StateTrace, node: NodeId) -> &'static str {
    let mut color = COLOR_DEFAULT;
    for (channel, n, _) in &state.attributes {
        if *n == node {
            color = color_channel(*channel);
        }
    }
    color
}

/// Height and color of a timeline tick according to the event type.
fn tiret_timeline(event: &TraceEvent) -> (u32, &'static str) {
    match event {
        TraceEvent::Round { .. } => (HEIGHT_TIMELINE - 16, COLOR_MEMORY),
        TraceEvent::MemoHit { .. } => (10, COLOR_MEMO_HIT),
        TraceEvent::MemoMiss { .. } => (10, COLOR_MEMO_MISS),
        TraceEvent::Attr { channel, .. } => (10, color_channel(*channel)),
        _ => (6, COLOR_OTHER),
    }
}

/// Opens the SVG document and renders the title (position, rounds, memo counters).
fn open_svg(svg: &mut String, width: u32, height: u32, scrubber: &Scrubber, state: &StateTrace) {
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{MARGIN_LEFT}\" y=\"20\" font-family=\"serif\" font-size=\"14\">step by step — position {}/{}, rounds {}, memo {}/{}</text>\n",
        scrubber.position(),
        scrubber.length(),
        state.rounds,
        state.memo_hits,
        state.memo_misses
    ));
}

/// Renders the timeline: one tick per event, and the cursor of the
/// current position highlighted.
fn render_timeline(svg: &mut String, scrubber: &Scrubber, width: u32) {
    let y_timeline = MARGIN_HAUT + 8;
    let width_timeline = width - 2 * MARGIN_LEFT;
    let step = if scrubber.length() == 0 {
        0
    } else {
        width_timeline / scrubber.length() as u32
    };
    for (index, event) in scrubber.events.iter().enumerate() {
        let x = MARGIN_LEFT + index as u32 * step;
        let (height_tiret, color) = tiret_timeline(event);
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{color}\" stroke-width=\"2\"/>\n",
            y_timeline + HEIGHT_TIMELINE - 16 - height_tiret,
            y_timeline + HEIGHT_TIMELINE - 16
        ));
    }
    if scrubber.position() > 0 && scrubber.position() <= scrubber.length() {
        let x = MARGIN_LEFT + scrubber.position() as u32 * step;
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{COLOR_CURSOR}\" stroke-width=\"2\"/>\n",
            y_timeline,
            y_timeline + HEIGHT_TIMELINE - 12
        ));
    }
}

/// Renders the diagram arcs (under the nodes) with their labels.
fn render_arcs<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    root: NodeId,
    placement: &BTreeMap<NodeId, Placement>,
    y_shift: u32,
) {
    let mut stack = vec![root];
    let mut already_seen = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !already_seen.insert(node) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(node) {
            let depart = &placement[&node];
            for (key, target) in arcs {
                let arrivee = &placement[&target];
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COLOR_ARC}\"/>\n",
                    depart.x,
                    depart.y + y_shift + 14,
                    arrivee.x,
                    arrivee.y + y_shift - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"{COLOR_LABEL}\">{}</text>\n",
                    (depart.x + arrivee.x) / 2,
                    (depart.y + arrivee.y) / 2 + y_shift,
                    escape_xml(&key.to_string())
                ));
                stack.push(target);
            }
        }
    }
}

/// Renders a node (terminal or internal); as a ghost (dashed) if requested.
fn render_node<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    node: NodeId,
    position: &Placement,
    ghost: bool,
    state: &StateTrace,
    y_shift: u32,
) {
    let color = if ghost {
        COLOR_GHOST
    } else {
        color_node(state, node)
    };
    let style = if ghost {
        format!("stroke=\"{color}\" stroke-dasharray=\"4 3\"")
    } else {
        format!("stroke=\"{color}\" stroke-width=\"2\"")
    };
    let x = position.x - HALF_WIDTH;
    let y = position.y + y_shift - 14;
    match pool.resolve(node) {
        Ok(Form::Terminal(payload)) => {
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{COLOR_BACKGROUND_TERMINAL}\" {style}/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">{}</text>\n",
                position.x,
                position.y + y_shift + 4,
                if payload { "⊤" } else { "⊥" }
            ));
        }
        Ok(Form::Node { level, .. }) => {
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{COLOR_BACKGROUND_NODE}\" {style}/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                position.x,
                position.y + y_shift + 4,
                level.0,
                node.0
            ));
        }
        Err(_) => {
            // Unreachable: the placement is derived from the same pool via
            // `placer`, so `resolve` already succeeded on this node.
            debug_assert!(false, "resolve failed on a node of the placement");
        }
    }
}

/// Renders all the nodes of the placement, ghosts as watermark.
fn render_nodes<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    placement: &BTreeMap<NodeId, Placement>,
    diff: &DiffTrace,
    state: &StateTrace,
    y_shift: u32,
) {
    for (node, position) in placement {
        render_node(
            svg,
            pool,
            *node,
            position,
            diff.ghosts.contains(node),
            state,
            y_shift,
        );
    }
}

/// Renders in static SVG the step-by-step view at the scrubber position:
/// event timeline (rounds as vertical bars, current position
/// highlighted), diagram with colored channels, ghosts as watermark, and
/// dashboard (hits/misses/rounds).
pub fn render_svg_step_by_step<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    scrubber: &Scrubber,
    diff: &DiffTrace,
) -> String {
    let state = scrubber.state();
    let (placement, number_levels) = placer(pool, root);
    let width = MARGIN_LEFT * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + HALF_WIDTH;
    let height = MARGIN_HAUT + HEIGHT_TIMELINE + (number_levels + 1) * HEIGHT_LEVEL + 20;
    let mut svg = String::new();
    open_svg(&mut svg, width, height, scrubber, &state);
    render_timeline(&mut svg, scrubber, width);
    let y_shift = HEIGHT_TIMELINE + 8;
    render_arcs(&mut svg, pool, root, &placement, y_shift);
    render_nodes(&mut svg, pool, &placement, diff, &state, y_shift);
    svg.push_str("</svg>\n");
    svg
}

/// Legend of the channels (colors) for the step-by-step rendering.
pub fn legend_channels() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("inherited", COLOR_INHERITED),
        ("synthesized", COLOR_SYNTHESIZED),
        ("memory", COLOR_MEMORY),
        ("ghost", COLOR_GHOST),
    ])
}
