//! D3-1: command bus of the `lisage-debug` binary — navigation over a
//! trace (step / step-over / step-out / next-op / next-round / back /
//! goto / until / where / frame / continue), with the visit stack and
//! granularity parity with the span tree (D2-4): step-over/step-out
//! stop on span boundaries from the [`SpanTree`].
//!
//! The [`Debugger`] is a pure state machine over the trace (position
//! cursor); the REPL binary only feeds it parsed commands, so the whole
//! command bus is testable without a TTY.

use std::collections::{BTreeMap, BTreeSet};

use lisage_core::arena::Arena;
use lisage_core::observer::{Observer, SpanId, SpanKind, TraceEvent};
use lisage_core::saturate::{ParametresSaturation, PolicySaturation, saturer};
use lisage_core::schema::{
    ABSENT, Domain, PRESENT, ReferenceController, Schema, SchemaKeys, compiler,
};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_core::{Anchor, Form, Level, NodeId, Pool};
use lisage_lang::value::{Atom, Value};

use crate::spans::{Balances, SpanNode, SpanTree};

/// Fingerprint of the [`ChainStep`] scenario transduction.
const CHAIN_STEP_FINGERPRINT: u64 = 4164;

/// Step transduction of the standard scenario: `τ(X) = {{m}}` where
/// `m = max atom of X + 1` — an increasing chain of singletons, so a
/// `Set` domain of size `D` saturates in exactly `D` rounds.
/// Public for D3-3 (`standard_scenario_full` in `inspection.rs`).
#[derive(Clone)]
pub struct ChainStep {
    /// Number of levels of the `Set` schema (domain size).
    pub number_levels: u32,
}

impl Transduction for ChainStep {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        CHAIN_STEP_FINGERPRINT
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}
    fn phi_aggr(&self, level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        let mut max = 0u64;
        for (arc, (_, seen_below)) in arcs {
            max = max.max(*seen_below);
            if *arc == PRESENT {
                max = max.max(level.0 as u64 + 1);
            }
        }
        max
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<()>) {
        let mut m = 0u64;
        for (arc, (_, seen_below)) in &ctx.arcs {
            m = m.max(*seen_below);
            if *arc == PRESENT {
                m = m.max(ctx.level.0 as u64 + 1);
            }
        }
        let node = if m >= self.number_levels as u64 {
            pool.intern(ctx.anchor, Form::Terminal(false))
        } else {
            let mut current = pool.intern(ctx.anchor, Form::Terminal(true));
            for level in (0..self.number_levels).rev() {
                let arcs: BTreeMap<u64, NodeId> = if level as u64 == m {
                    BTreeMap::from([(PRESENT, current)])
                } else {
                    BTreeMap::from([(ABSENT, current)])
                };
                current = pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: Level(level),
                        arcs: arcs.into(),
                    },
                );
            }
            current
        };
        (node, lisage_core::memory::EffetMemory::new())
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), 0)
    }
}

/// Observer that collects the typed events in memory.
#[derive(Clone, Default)]
struct Collector {
    events: std::sync::Arc<std::sync::Mutex<Vec<TraceEvent>>>,
}

impl Observer for Collector {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

/// The standard scenario (DoD): a saturation of the chain
/// `{{0}} → {{0},{1}} → …` over a `Set` domain of size `n`, traced —
/// exactly `n` rounds, one `Round` span each. `n = 4` is the reference
/// scenario.
pub fn standard_scenario(n: u64) -> Vec<TraceEvent> {
    let schema = Schema::Set(Domain::new((0..n).map(Atom).collect()));
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let initial = compiler(
        &mut pool,
        anchor,
        &schema,
        &Value::Set([Value::Atom(Atom(0))].into_iter().collect()),
    )
    .expect("scenario initial compilable");

    let collector = Collector::default();
    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy = PolicySaturation::Statique(levels);
    let step = ChainStep {
        number_levels: schema.number_levels(),
    };
    let mut params = ParametresSaturation {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(std::sync::Arc::new(collector.clone())),
    };
    let result = saturer(&mut params).expect("scenario saturation terminates");
    assert_eq!(
        result.iterations, n as u32,
        "the chain saturates in n rounds"
    );
    collector.events.lock().expect("mutex poisoned").clone()
}

/// One debugger command (the bus vocabulary of D3-1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Advance one event.
    Step,
    /// Retreat one event.
    Back,
    /// Jump to an event position.
    Goto(usize),
    /// Run to the end of the innermost enclosing span (span boundary).
    StepOver,
    /// Run to the end of the parent of the innermost enclosing span.
    StepOut,
    /// Jump to the next `OpStart`.
    NextOp,
    /// Jump to the next `Round` event.
    NextRound,
    /// Advance until the next event of the given kind.
    Until(EventKind),
    /// Run to the end of the trace.
    Continue,
    /// Where am I: position, event, enclosing spans, visit stack.
    Where,
    /// Current frame: innermost span with its folded balance.
    Frame,
    /// D3-3: inspect a node (creation, level, form, index ranges).
    Node(NodeId),
    /// D3-3: memo table at the cursor (live/dead entries, L1).
    Memo,
    /// D3-3: level-memory version history at the cursor.
    Memoire,
    /// D3-3: structure-change diff (input/output fragments) at the cursor.
    Diff,
    /// D3-3: evaluate a valuation over the counters at the cursor.
    Eval(String),
    /// D3-3: dashboard at the cursor.
    Display,
    /// D3-3: list the operations of the trace.
    Ops,
}

/// Event kinds accepted by `until`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// Any `Round` event.
    Round,
    /// `OpStart`.
    OpStart,
    /// `OpEnd`.
    OpEnd,
    /// `NodeCreated`.
    NodeCreated,
    /// `MemoHit`.
    MemoHit,
    /// `MemoMiss`.
    MemoMiss,
    /// `SpanOpen`.
    SpanOpen,
    /// `SpanClose`.
    SpanClose,
}

impl EventKind {
    /// Parses the `until` argument.
    pub fn parse(word: &str) -> Option<EventKind> {
        match word {
            "round" => Some(EventKind::Round),
            "op-start" | "op" => Some(EventKind::OpStart),
            "op-end" => Some(EventKind::OpEnd),
            "node-created" => Some(EventKind::NodeCreated),
            "memo-hit" => Some(EventKind::MemoHit),
            "memo-miss" => Some(EventKind::MemoMiss),
            "span-open" => Some(EventKind::SpanOpen),
            "span-close" => Some(EventKind::SpanClose),
            _ => None,
        }
    }

    fn matches(&self, event: &TraceEvent) -> bool {
        matches!(
            (self, event),
            (EventKind::Round, TraceEvent::Round { .. })
                | (EventKind::OpStart, TraceEvent::OpStart { .. })
                | (EventKind::OpEnd, TraceEvent::OpEnd { .. })
                | (EventKind::NodeCreated, TraceEvent::NodeCreated { .. })
                | (EventKind::MemoHit, TraceEvent::MemoHit { .. })
                | (EventKind::MemoMiss, TraceEvent::MemoMiss { .. })
                | (EventKind::SpanOpen, TraceEvent::SpanOpen { .. })
                | (EventKind::SpanClose, TraceEvent::SpanClose { .. })
        )
    }
}

/// Parses one REPL line into a command. `None` on unknown input.
pub fn parse_command(line: &str) -> Option<Command> {
    // `eval` takes the rest of the line as its valuation.
    if let Some(rest) = line.strip_prefix("eval ") {
        return Some(Command::Eval(rest.trim().to_string()));
    }
    let mut words = line.split_whitespace();
    match words.next()? {
        "step" | "s" => Some(Command::Step),
        "back" | "b" => Some(Command::Back),
        "step-over" | "so" => Some(Command::StepOver),
        "step-out" | "out" => Some(Command::StepOut),
        "next-op" | "nop" => Some(Command::NextOp),
        "next-round" | "nr" => Some(Command::NextRound),
        "continue" | "c" => Some(Command::Continue),
        "where" | "w" => Some(Command::Where),
        "frame" | "f" => Some(Command::Frame),
        "goto" => words.next()?.parse().ok().map(Command::Goto),
        "until" => EventKind::parse(words.next()?).map(Command::Until),
        "node" => words
            .next()?
            .parse()
            .ok()
            .map(|id: u32| Command::Node(NodeId(id))),
        "memo" => Some(Command::Memo),
        "memoire" => Some(Command::Memoire),
        "diff" => Some(Command::Diff),
        "display" => Some(Command::Display),
        "ops" => Some(Command::Ops),
        _ => None,
    }
}

/// Name of an event's kind, for `where` output (public for D3-3
/// `display`).
pub fn event_kind_name(event: &TraceEvent) -> &'static str {
    match event {
        TraceEvent::SpanOpen { .. } => "SpanOpen",
        TraceEvent::SpanClose { .. } => "SpanClose",
        TraceEvent::NodeCreated { .. } => "NodeCreated",
        TraceEvent::NodeShared { .. } => "NodeShared",
        TraceEvent::ReductionApplied { .. } => "ReductionApplied",
        TraceEvent::DiagramBuilt { .. } => "DiagramBuilt",
        TraceEvent::Attr { .. } => "Attr",
        TraceEvent::MemoHit { .. } => "MemoHit",
        TraceEvent::MemoMiss { .. } => "MemoMiss",
        TraceEvent::Gc { .. } => "Gc",
        TraceEvent::Round { .. } => "Round",
        TraceEvent::Histogram { .. } => "Histogram",
        TraceEvent::OpStart { .. } => "OpStart",
        TraceEvent::OpEnd { .. } => "OpEnd",
        TraceEvent::VisiteEntree { .. } => "VisiteEntree",
        TraceEvent::VisiteSortie { .. } => "VisiteSortie",
        TraceEvent::MemoConsult { .. } => "MemoConsult",
        TraceEvent::MemoInsert { .. } => "MemoInsert",
        TraceEvent::EcritureMemoire { .. } => "EcritureMemoire",
        TraceEvent::StructureChange { .. } => "StructureChange",
    }
}

/// The debugger: a position cursor over a trace, with the D2-3 index,
/// the D2-4 span tree and prefix sums, and the D3-3 inspection views.
pub struct Debugger {
    events: Vec<TraceEvent>,
    tree: SpanTree,
    balances: Balances,
    inspector: crate::inspection::Inspector,
    position: usize,
}

impl Debugger {
    /// New debugger over the trace, cursor at position 0.
    pub fn new(events: Vec<TraceEvent>) -> Self {
        let tree = SpanTree::build(&events);
        let balances = Balances::build(&events);
        let inspector = crate::inspection::Inspector::new(events.clone());
        Debugger {
            events,
            tree,
            balances,
            inspector,
            position: 0,
        }
    }

    /// Current event position (0..=length).
    pub fn position(&self) -> usize {
        self.position
    }

    /// Number of events in the trace.
    pub fn length(&self) -> usize {
        self.events.len()
    }

    /// The event at the cursor, `None` at the trace end.
    pub fn current_event(&self) -> Option<&TraceEvent> {
        self.events.get(self.position)
    }

    /// Spans enclosing the current position, outermost first.
    pub fn enclosing_spans(&self) -> Vec<&SpanNode> {
        fn collect<'a>(node: &'a SpanNode, position: usize, out: &mut Vec<&'a SpanNode>) {
            if node.range.start <= position && position < node.range.end {
                out.push(node);
                for child in &node.children {
                    collect(child, position, out);
                }
            }
        }
        let mut out = Vec::new();
        for root in self.tree.roots() {
            collect(root, self.position, &mut out);
        }
        out
    }

    /// The visit stack: `Visite` spans enclosing the current position,
    /// outermost first (the innermost visit is last).
    pub fn visit_stack(&self) -> Vec<SpanId> {
        self.enclosing_spans()
            .iter()
            .filter(|node| node.kind == SpanKind::Visite)
            .map(|node| node.span)
            .collect()
    }

    /// Runs one command and returns its output text.
    pub fn run(&mut self, command: Command) -> String {
        match command {
            Command::Step => {
                self.position = (self.position + 1).min(self.events.len());
                self.where_output()
            }
            Command::Back => {
                self.position = self.position.saturating_sub(1);
                self.where_output()
            }
            Command::Goto(target) => {
                self.position = target.min(self.events.len());
                self.where_output()
            }
            Command::StepOver => {
                match self.enclosing_spans().last() {
                    Some(innermost) => {
                        self.position = innermost.range.end.min(self.events.len());
                    }
                    None => {
                        self.position = (self.position + 1).min(self.events.len());
                    }
                }
                self.where_output()
            }
            Command::StepOut => {
                let spans = self.enclosing_spans();
                self.position = match spans.len() {
                    0 => (self.position + 1).min(self.events.len()),
                    1 => self.events.len(),
                    n => spans[n - 2].range.end.min(self.events.len()),
                };
                self.where_output()
            }
            Command::NextOp => {
                self.position = self.next_matching(|e| matches!(e, TraceEvent::OpStart { .. }));
                self.where_output()
            }
            Command::NextRound => {
                self.position = self.next_matching(|e| matches!(e, TraceEvent::Round { .. }));
                self.where_output()
            }
            Command::Until(kind) => {
                self.position = self.next_matching(|e| kind.matches(e));
                self.where_output()
            }
            Command::Continue => {
                self.position = self.events.len();
                self.where_output()
            }
            Command::Where => self.where_output(),
            Command::Frame => self.frame_output(),
            Command::Node(node) => self.inspector.node_view(self.position, node),
            Command::Memo => self.inspector.memo_view(self.position),
            Command::Memoire => self.inspector.memory_view(self.position),
            Command::Diff => self.inspector.diff_view(self.position),
            Command::Eval(expression) => match self.inspector.eval(self.position, &expression) {
                Ok(value) => format!("{value}"),
                Err(error) => format!("eval error: {error}"),
            },
            Command::Display => self.inspector.display(
                self.position,
                self.enclosing_spans().len(),
                self.visit_stack().len(),
            ),
            Command::Ops => self.inspector.ops_view(self.position),
        }
    }

    /// First position strictly after the cursor matching `predicate`,
    /// or the trace end.
    fn next_matching(&self, predicate: impl Fn(&TraceEvent) -> bool) -> usize {
        (self.position + 1..self.events.len())
            .find(|&p| predicate(&self.events[p]))
            .unwrap_or(self.events.len())
    }

    fn where_output(&self) -> String {
        let event = self
            .events
            .get(self.position)
            .map(event_kind_name)
            .unwrap_or("<end>");
        let spans = self
            .enclosing_spans()
            .iter()
            .map(|node| format!("{:?}[{}..{}]", node.kind, node.range.start, node.range.end))
            .collect::<Vec<_>>()
            .join(" > ");
        let visits = self
            .visit_stack()
            .iter()
            .map(|span| format!("{span:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "pos {}/{} event {event} spans [{spans}] visits [{visits}]",
            self.position,
            self.events.len()
        )
    }

    fn frame_output(&self) -> String {
        match self.enclosing_spans().last() {
            None => format!("pos {}: no enclosing span", self.position),
            Some(node) => {
                let balance = self.balances.balance(node.range.clone());
                format!(
                    "frame {:?} span {:?} lane {} range {}..{} children {} balance events={} nodes_created={} hits={} growth={:+}",
                    node.kind,
                    node.span,
                    node.lane,
                    node.range.start,
                    node.range.end,
                    node.children.len(),
                    balance.events,
                    balance.nodes_created,
                    balance.hits,
                    balance.growth
                )
            }
        }
    }
}

/// Round numbers present in the trace, ascending (for `next-round`
/// style queries and tests).
pub fn round_numbers(events: &[TraceEvent]) -> BTreeSet<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            TraceEvent::Round { numero, .. } => Some(*numero),
            _ => None,
        })
        .collect()
}
