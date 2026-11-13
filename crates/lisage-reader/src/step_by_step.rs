//! Reader v1 — **step-by-step** view (M2-6, reader doc § 6) : scrubber over
//! the trace, colored channels (inherited blue / synthesized orange / memory
//! purple), `MemoHit`/`MemoMiss`, saturation rounds, and before/after
//! differential with **ghosts** (absorbed nodes rendered as watermark).
//!
//! The scrubber is a position `k` on the trace ; the state at `k` is the
//! replay of events `0..k`. The static SVG rendering at a given position
//! embeds the event timeline — the interactive JS version is out of scope
//! (doc § 7 : the static SVG output remains complete).

use std::collections::{BTreeMap, BTreeSet};

use lisage_core::observer::{ChannelAttribute, TraceEvent};
use lisage_core::{Form, NodeId, Pool};

use crate::svg::{
    DEMI_WIDTH, HEIGHT_LEVEL, MARGIN_GAUCHE, MARGIN_HAUT, Placement, WIDTH_NODE, escape_xml, placer,
};

/// Color of the inherited channel (blue).
const COLOR_HERITE: &str = "#2980b9";
/// Color of the synthesized channel (orange).
const COLOR_SYNTHETISE: &str = "#e67e22";
/// Color of the level memory (purple).
const COLOR_MEMOIRE: &str = "#8e44ad";
/// Color of ghosts (gray watermark).
const COLOR_FANTOME: &str = "#bbb";
/// Default color of nodes without attribute (M2-6).
const COLOR_DEFAUT: &str = "#333";
/// Color of a `MemoHit` on the timeline (green).
const COLOR_MEMO_HIT: &str = "#27ae60";
/// Color of a `MemoMiss` on the timeline (red).
const COLOR_MEMO_MISS: &str = "#c0392b";
/// Color of the other events on the timeline (neutral gray).
const COLOR_AUTRE: &str = "#999";
/// Color of the diagram edges.
const COLOR_EDGE: &str = "#555";
/// Color of edge labels.
const COLOR_ETIQUETTE: &str = "#777";
/// Color of the position cursor on the timeline (black).
const COLOR_CURSEUR: &str = "#000";
/// Background of terminals.
const COLOR_BACKGROUND_TERMINAL: &str = "#eee";
/// Background of internal nodes.
const COLOR_BACKGROUND_NODE: &str = "#fff";
/// Height of the timeline in pixels.
const HEIGHT_FRISE: u32 = 46;

/// Trace state at a scrubber position: replay of events `0..k`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateTrace {
    /// Nodes seen (created or shared) up to the position.
    pub nodes: BTreeSet<NodeId>,
    /// Attributes seen `(channel, node, value)`, in emission order.
    pub attributes: Vec<(ChannelAttribute, NodeId, String)>,
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

/// Scrubber: position on a trace of events + state replay.
pub struct Scrubber {
    events: Vec<TraceEvent>,
    position: usize,
}

impl Scrubber {
    /// New scrubber over the given trace, positioned at the start.
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
    pub fn go_to(&mut self, position: usize) {
        self.position = position.min(self.events.len());
    }

    /// Replay of events `0..position`: trace state at the current position.
    pub fn state(&self) -> StateTrace {
        let mut state = StateTrace::default();
        for event in &self.events[..self.position] {
            match event {
                TraceEvent::NodeCreated { node, .. } | TraceEvent::NodeShared { node } => {
                    state.nodes.insert(*node);
                }
                TraceEvent::Attribute {
                    channel,
                    node,
                    value,
                } => {
                    state.attributes.push((*channel, *node, value.clone()));
                }
                TraceEvent::MemoHit { .. } => state.memo_hits += 1,
                TraceEvent::MemoMiss { .. } => state.memo_misses += 1,
                TraceEvent::Round { number } => state.rounds = state.rounds.max(*number),
                TraceEvent::ReductionApplied { .. }
                | TraceEvent::DiagramBuilt { .. }
                | TraceEvent::Gc { .. } => {}
            }
        }
        state
    }

    /// Differential between the current position and position `other`.
    pub fn diff_toward(&self, other: usize) -> DiffTrace {
        let mut other_scrubber = Scrubber::new(self.events.clone());
        other_scrubber.go_to(other);
        diff(&self.state(), &other_scrubber.state())
    }
}

/// Color of an attribute channel.
fn color_channel(channel: ChannelAttribute) -> &'static str {
    match channel {
        ChannelAttribute::Inherited => COLOR_HERITE,
        ChannelAttribute::Synthesize => COLOR_SYNTHETISE,
        ChannelAttribute::Memory => COLOR_MEMOIRE,
    }
}

/// Border color of a node according to the **last** attribute channel seen
/// on this node (memory > synthesized > inherited > default).
fn color_node(state: &StateTrace, node: NodeId) -> &'static str {
    let mut color = COLOR_DEFAUT;
    for (channel, n, _) in &state.attributes {
        if *n == node {
            color = color_channel(*channel);
        }
    }
    color
}

/// Height and color of a timeline dash according to the event type.
fn dash_timeline(event: &TraceEvent) -> (u32, &'static str) {
    match event {
        TraceEvent::Round { .. } => (HEIGHT_FRISE - 16, COLOR_MEMOIRE),
        TraceEvent::MemoHit { .. } => (10, COLOR_MEMO_HIT),
        TraceEvent::MemoMiss { .. } => (10, COLOR_MEMO_MISS),
        TraceEvent::Attribute { channel, .. } => (10, color_channel(*channel)),
        _ => (6, COLOR_AUTRE),
    }
}

/// Opens the SVG document and renders the title (position, rounds, memo counters).
fn open_svg(svg: &mut String, width: u32, height: u32, scrubber: &Scrubber, state: &StateTrace) {
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{MARGIN_GAUCHE}\" y=\"20\" font-family=\"serif\" font-size=\"14\">step by step — position {}/{}, rounds {}, memo {}/{}</text>\n",
        scrubber.position(),
        scrubber.length(),
        state.rounds,
        state.memo_hits,
        state.memo_misses
    ));
}

/// Renders the timeline: one dash per event, and the current position
/// cursor highlighted.
fn render_timeline(svg: &mut String, scrubber: &Scrubber, width: u32) {
    let y_timeline = MARGIN_HAUT + 8;
    let width_timeline = width - 2 * MARGIN_GAUCHE;
    let not_ = if scrubber.length() == 0 {
        0
    } else {
        width_timeline / scrubber.length() as u32
    };
    for (index, event) in scrubber.events.iter().enumerate() {
        let x = MARGIN_GAUCHE + index as u32 * not_;
        let (height_dash, color) = dash_timeline(event);
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{color}\" stroke-width=\"2\"/>\n",
            y_timeline + HEIGHT_FRISE - 16 - height_dash,
            y_timeline + HEIGHT_FRISE - 16
        ));
    }
    if scrubber.position() > 0 && scrubber.position() <= scrubber.length() {
        let x = MARGIN_GAUCHE + scrubber.position() as u32 * not_;
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" stroke=\"{COLOR_CURSEUR}\" stroke-width=\"2\"/>\n",
            y_timeline,
            y_timeline + HEIGHT_FRISE - 12
        ));
    }
}

/// Renders the diagram edges (under the nodes) with their labels.
fn render_edges<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    root: NodeId,
    placement: &BTreeMap<NodeId, Placement>,
    y_offset: u32,
) {
    let mut stack = vec![root];
    let mut already_seen = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !already_seen.insert(node) {
            continue;
        }
        if let Ok(Form::Node { edges, .. }) = pool.resolve(node) {
            let start = &placement[&node];
            for (key, target) in edges {
                let end = &placement[target];
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{COLOR_EDGE}\"/>\n",
                    start.x,
                    start.y + y_offset + 14,
                    end.x,
                    end.y + y_offset - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"{COLOR_ETIQUETTE}\">{}</text>\n",
                    (start.x + end.x) / 2,
                    (start.y + end.y) / 2 + y_offset,
                    escape_xml(&key.to_string())
                ));
                stack.push(*target);
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
    fantome: bool,
    state: &StateTrace,
    y_offset: u32,
) {
    let color = if fantome {
        COLOR_FANTOME
    } else {
        color_node(state, node)
    };
    let style = if fantome {
        format!("stroke=\"{color}\" stroke-dasharray=\"4 3\"")
    } else {
        format!("stroke=\"{color}\" stroke-width=\"2\"")
    };
    let x = position.x - DEMI_WIDTH;
    let y = position.y + y_offset - 14;
    match pool.resolve(node) {
        Ok(Form::Terminal(payload)) => {
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{COLOR_BACKGROUND_TERMINAL}\" {style}/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">{}</text>\n",
                position.x,
                position.y + y_offset + 4,
                if *payload { "⊤" } else { "⊥" }
            ));
        }
        Ok(Form::Node { level, .. }) => {
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{WIDTH_NODE}\" height=\"28\" fill=\"{COLOR_BACKGROUND_NODE}\" {style}/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                position.x,
                position.y + y_offset + 4,
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

/// Renders all nodes of the placement, ghosts as watermark.
fn render_nodes<P: Pool<Key = u64, Payload = bool>>(
    svg: &mut String,
    pool: &P,
    placement: &BTreeMap<NodeId, Placement>,
    diff: &DiffTrace,
    state: &StateTrace,
    y_offset: u32,
) {
    for (node, position) in placement {
        render_node(
            svg,
            pool,
            *node,
            position,
            diff.ghosts.contains(node),
            state,
            y_offset,
        );
    }
}

/// Renders the step-by-step view at the scrubber position as static SVG:
/// event timeline (rounds as vertical bars, current position highlighted),
/// diagram with colored channels, ghosts as watermark, and dashboard
/// (hits/misses/rounds).
pub fn render_svg_not_a_not<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    scrubber: &Scrubber,
    diff: &DiffTrace,
) -> String {
    let state = scrubber.state();
    let (placement, number_levels) = placer(pool, root);
    let width = MARGIN_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_WIDTH;
    let height = MARGIN_HAUT + HEIGHT_FRISE + (number_levels + 1) * HEIGHT_LEVEL + 20;
    let mut svg = String::new();
    open_svg(&mut svg, width, height, scrubber, &state);
    render_timeline(&mut svg, scrubber, width);
    let y_offset = HEIGHT_FRISE + 8;
    render_edges(&mut svg, pool, root, &placement, y_offset);
    render_nodes(&mut svg, pool, &placement, diff, &state, y_offset);
    svg.push_str("</svg>\n");
    svg
}

/// Legend of the channels (colors) for the step-by-step rendering.
pub fn legend_channels() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("inherited", COLOR_HERITE),
        ("synthesized", COLOR_SYNTHETISE),
        ("memory", COLOR_MEMOIRE),
        ("ghost", COLOR_FANTOME),
    ])
}
