//! D3-3: inspection views of the debugger (issue #166) — `node`, `memo`,
//! `memory`, `diff`, `eval <valuation>`, `display`, `ops`, and the
//! current SVG rendering (reusing `svg.rs` / `step_by_step.rs`).
//!
//! The [`Inspector`] is a pure query engine over the trace (plus the
//! D2-3 inverse index): every view takes the cursor position and replays
//! `events[..position]`. The memo view splits the computed table into
//! **live** entries (inserted at the current memory version) and **dead**
//! entries (inserted at an older version — invalidated by the L1 version
//! bump), which is the DoD: one `memo` command shows the invalidation.

use lisage_core::arena::Arena;
use lisage_core::observer::{Observer, TraceEvent};
use lisage_core::saturate::{PolicySaturation, SaturationParameters, saturate};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::{Anchor, Level, NodeId, Pool};
use lisage_lang::value::{Atom, Value};

use crate::debugger::ChainStep;
use crate::index::IndexTrace;
use crate::mirror::PoolMirror;
use crate::step_by_step::Scrubber;

/// One memo-table entry as reconstructed from the trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoEntry {
    /// Fingerprint of the operation.
    pub fingerprint: u64,
    /// Node of the key.
    pub node: NodeId,
    /// Memory version at insertion time.
    pub version: u64,
    /// Result stored under the key.
    pub result: NodeId,
}

/// Memo table replayed up to a position: entries with their insertion
/// version, and the current memory version.
#[derive(Clone, Debug, Default)]
pub struct MemoTable {
    /// Entries in insertion order.
    pub entries: Vec<MemoEntry>,
    /// Current memory version (last `MemoryWrite`).
    pub version: u64,
}

impl MemoTable {
    /// Replays `events[..position]` into the memo table.
    pub fn replay(events: &[TraceEvent], position: usize) -> Self {
        let mut table = MemoTable::default();
        for event in &events[..position.min(events.len())] {
            match event {
                TraceEvent::MemoInsert {
                    fingerprint,
                    node,
                    version,
                    result,
                    ..
                } => table.entries.push(MemoEntry {
                    fingerprint: *fingerprint,
                    node: *node,
                    version: *version,
                    result: *result,
                }),
                TraceEvent::MemoryWrite { version_after, .. } => {
                    table.version = *version_after;
                }
                _ => {}
            }
        }
        table
    }

    /// Live entries: inserted at the current memory version.
    pub fn live(&self) -> impl Iterator<Item = &MemoEntry> {
        self.entries.iter().filter(|e| e.version == self.version)
    }

    /// Dead entries: inserted at an older version, invalidated by the
    /// L1 version bump.
    pub fn dead(&self) -> impl Iterator<Item = &MemoEntry> {
        self.entries.iter().filter(|e| e.version < self.version)
    }
}

/// Counters available to the `eval` valuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counters {
    /// Cursor position.
    pub position: i64,
    /// Trace length.
    pub length: i64,
    /// Live nodes (mirror).
    pub live: i64,
    /// Live memo entries.
    pub memo_live: i64,
    /// Dead memo entries (L1 invalidation).
    pub memo_dead: i64,
    /// Current memory version.
    pub version: i64,
    /// Last round number seen.
    pub rounds: i64,
    /// Memo hits.
    pub hits: i64,
    /// Memo misses.
    pub misses: i64,
}

/// The inspection views: pure queries over the trace at a cursor position.
pub struct Inspector {
    events: Vec<TraceEvent>,
    index: IndexTrace,
}

impl Inspector {
    /// New inspector over the trace (builds the D2-3 index once).
    pub fn new(events: Vec<TraceEvent>) -> Self {
        let index = IndexTrace::build(&events);
        Inspector { events, index }
    }

    /// Number of events in the trace.
    pub fn length(&self) -> usize {
        self.events.len()
    }

    /// Counters at `position` (replay of `events[..position]`).
    pub fn counters(&self, position: usize) -> Counters {
        let position = position.min(self.events.len());
        let prefix = &self.events[..position];
        let mirror = PoolMirror::build(prefix);
        let memo = MemoTable::replay(&self.events, position);
        let mut rounds = 0i64;
        let mut hits = 0i64;
        let mut misses = 0i64;
        for event in prefix {
            match event {
                TraceEvent::Round { number, .. } => rounds = rounds.max(*number as i64),
                TraceEvent::MemoHit { .. } => hits += 1,
                TraceEvent::MemoMiss { .. } => misses += 1,
                _ => {}
            }
        }
        Counters {
            position: position as i64,
            length: self.events.len() as i64,
            live: mirror.live_count() as i64,
            memo_live: memo.live().count() as i64,
            memo_dead: memo.dead().count() as i64,
            version: memo.version as i64,
            rounds,
            hits,
            misses,
        }
    }

    /// `node <id>`: creation event (level, rendered form) and the index
    /// ranges of the events mentioning the node.
    pub fn node_view(&self, position: usize, node: NodeId) -> String {
        let creation = self.events.iter().enumerate().find_map(|(i, e)| match e {
            TraceEvent::NodeCreated {
                node: n,
                level,
                form,
                ..
            } if *n == node => Some((i, *level, form.render())),
            _ => None,
        });
        let ranges = self.index.node_ranges(node);
        let ranges_text = ranges
            .iter()
            .map(|r| format!("{}..{}", r.start, r.end))
            .collect::<Vec<_>>()
            .join(", ");
        match creation {
            Some((i, level, form)) => format!(
                "node {node:?} @ pos {position}: created @ {i} level {} form \"{form}\" events {} ranges [{ranges_text}]",
                level
                    .map(|l| l.0.to_string())
                    .unwrap_or_else(|| "null".to_string()),
                ranges.iter().map(|r| r.end - r.start).sum::<usize>(),
            ),
            None => format!(
                "node {node:?} @ pos {position}: not created in the trace, events {} ranges [{ranges_text}]",
                ranges.iter().map(|r| r.end - r.start).sum::<usize>(),
            ),
        }
    }

    /// `memo`: the computed table at the cursor — current version, live
    /// entries, and dead entries invalidated by the version bump (L1).
    pub fn memo_view(&self, position: usize) -> String {
        let memo = MemoTable::replay(&self.events, position);
        let mut out = format!(
            "memo @ pos {}/{}: version {}, live {}, dead {}",
            position.min(self.events.len()),
            self.events.len(),
            memo.version,
            memo.live().count(),
            memo.dead().count()
        );
        for entry in memo.live() {
            out.push_str(&format!(
                "\nlive: op {} node {:?} v{} -> {:?}",
                entry.fingerprint, entry.node, entry.version, entry.result
            ));
        }
        for entry in memo.dead() {
            out.push_str(&format!(
                "\ndead: op {} node {:?} v{} -> {:?}",
                entry.fingerprint, entry.node, entry.version, entry.result
            ));
        }
        out
    }

    /// `memory`: the level-memory version history up to the cursor.
    pub fn memory_view(&self, position: usize) -> String {
        let position = position.min(self.events.len());
        let mut version = 0u64;
        let mut writes: Vec<String> = Vec::new();
        for (i, event) in self.events[..position].iter().enumerate() {
            if let TraceEvent::MemoryWrite {
                version_before,
                version_after,
                ..
            } = event
            {
                writes.push(format!("{}->{} @ {i}", version_before, version_after));
                version = *version_after;
            }
        }
        format!(
            "memory @ pos {}/{}: version {}\nwrites: {}",
            position,
            self.events.len(),
            version,
            if writes.is_empty() {
                "none".to_string()
            } else {
                writes.join(", ")
            }
        )
    }

    /// `diff`: the structure-change fragments (input/output arcs) at the
    /// cursor, or at the next `StructureChange` if the cursor is elsewhere.
    pub fn diff_view(&self, position: usize) -> String {
        let position = position.min(self.events.len());
        let target = self.events[position..]
            .iter()
            .enumerate()
            .find_map(|(offset, e)| match e {
                TraceEvent::StructureChange { .. } => Some(position + offset),
                _ => None,
            });
        match target {
            None => format!(
                "diff @ pos {position}/{}: no StructureChange after the cursor",
                self.events.len()
            ),
            Some(at) => match &self.events[at] {
                TraceEvent::StructureChange {
                    node,
                    classification,
                    arcs_before,
                    arcs_after,
                    ..
                } => {
                    let render_arcs = |arcs: &[(u64, NodeId)]| -> String {
                        arcs.iter()
                            .map(|(k, t)| format!("{k}->{t:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    format!(
                        "diff @ pos {at}/{}: node {node:?} {:?}\n  before: [{}]\n  after: [{}]",
                        self.events.len(),
                        classification,
                        render_arcs(arcs_before),
                        render_arcs(arcs_after)
                    )
                }
                _ => unreachable!("target is a StructureChange"),
            },
        }
    }

    /// `ops`: the operations of the trace (D2-3 index), with the one
    /// containing the cursor marked.
    pub fn ops_view(&self, position: usize) -> String {
        let mut out = format!(
            "ops @ pos {}/{}: ",
            position.min(self.events.len()),
            self.events.len()
        );
        let mut number = 0;
        let mut lines: Vec<String> = Vec::new();
        while let Some(range) = self.index.op_range(number) {
            let fingerprint = match self.events.get(range.start) {
                Some(TraceEvent::OpStart { fingerprint, .. }) => *fingerprint,
                _ => 0,
            };
            let current = range.start <= position && position < range.end;
            lines.push(format!(
                "op {number} [{}..{}] fingerprint {fingerprint}{}",
                range.start,
                range.end,
                if current { " (current)" } else { "" }
            ));
            number += 1;
        }
        out.push_str(&format!("{} ops", lines.len()));
        for line in lines {
            out.push('\n');
            out.push_str(&line);
        }
        out
    }

    /// `display`: dashboard at the cursor — event, spans, visits, and
    /// the reconstructed counters.
    pub fn display(&self, position: usize, enclosing_spans: usize, visit_depth: usize) -> String {
        let position = position.min(self.events.len());
        let counters = self.counters(position);
        let event = self
            .events
            .get(position)
            .map(crate::debugger::event_kind_name)
            .unwrap_or("<end>");
        format!(
            "display @ pos {}/{}: event {event} spans {enclosing_spans} visits {visit_depth} | live {} memo {}+{} version {} rounds {} hits {} misses {}",
            position,
            self.events.len(),
            counters.live,
            counters.memo_live,
            counters.memo_dead,
            counters.version,
            counters.rounds,
            counters.hits,
            counters.misses
        )
    }

    /// `eval <valuation>`: evaluates an arithmetic expression over the
    /// counters at the cursor.
    pub fn eval(&self, position: usize, expression: &str) -> Result<i64, String> {
        let counters = self.counters(position);
        let mut parser = ExprParser {
            chars: expression.chars().collect(),
            position: 0,
            counters: &counters,
        };
        let value = parser.parse_sum()?;
        parser.skip_ws();
        if parser.position != parser.chars.len() {
            return Err("trailing characters in the expression".to_string());
        }
        Ok(value)
    }
}

/// Renders the current diagram in static SVG at `position` (reusing
/// `step_by_step.rs`: scrubber at the position, before/after differential
/// against the trace end with ghosts).
pub fn render_current_svg<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    root: NodeId,
    events: &[TraceEvent],
    position: usize,
) -> String {
    let mut scrubber = Scrubber::new(events.to_vec());
    scrubber.round_a(position);
    let diff = scrubber.diff_to(events.len());
    crate::step_by_step::render_svg_step_by_step(pool, root, &scrubber, &diff)
}

/// Recursive-descent arithmetic evaluator over the [`Counters`].
struct ExprParser<'a> {
    chars: Vec<char>,
    position: usize,
    counters: &'a Counters,
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.position += 1;
        }
    }

    fn parse_sum(&mut self) -> Result<i64, String> {
        let mut value = self.parse_product()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.position += 1;
                    value += self.parse_product()?;
                }
                Some('-') => {
                    self.position += 1;
                    value -= self.parse_product()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_product(&mut self) -> Result<i64, String> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.position += 1;
                    value *= self.parse_factor()?;
                }
                Some('/') => {
                    self.position += 1;
                    let divisor = self.parse_factor()?;
                    if divisor == 0 {
                        return Err("division by zero".to_string());
                    }
                    value /= divisor;
                }
                Some('%') => {
                    self.position += 1;
                    let divisor = self.parse_factor()?;
                    if divisor == 0 {
                        return Err("modulo by zero".to_string());
                    }
                    value %= divisor;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_factor(&mut self) -> Result<i64, String> {
        self.skip_ws();
        match self.peek() {
            Some('-') => {
                self.position += 1;
                Ok(-self.parse_factor()?)
            }
            Some('(') => {
                self.position += 1;
                let value = self.parse_sum()?;
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.position += 1;
                    Ok(value)
                } else {
                    Err("missing ')'".to_string())
                }
            }
            Some(c) if c.is_ascii_digit() => {
                let mut text = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        text.push(c);
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                text.parse::<i64>()
                    .map_err(|e| format!("invalid number '{text}': {e}"))
            }
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let mut name = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                self.counter(&name)
            }
            other => Err(format!("unexpected character {other:?}")),
        }
    }

    fn counter(&self, name: &str) -> Result<i64, String> {
        let counters = self.counters;
        match name {
            "position" => Ok(counters.position),
            "length" => Ok(counters.length),
            "live" => Ok(counters.live),
            "memo_live" => Ok(counters.memo_live),
            "memo_dead" => Ok(counters.memo_dead),
            "version" => Ok(counters.version),
            "rounds" => Ok(counters.rounds),
            "hits" => Ok(counters.hits),
            "misses" => Ok(counters.misses),
            _ => Err(format!("unknown counter '{name}'")),
        }
    }
}

/// Map of counter names for documentation/tests.
pub fn counter_names() -> &'static [&'static str] {
    &[
        "position",
        "length",
        "live",
        "memo_live",
        "memo_dead",
        "version",
        "rounds",
        "hits",
        "misses",
    ]
}

/// Observer that collects the typed events in memory (D3-3 scenario).
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

/// The standard scenario (D3-1) with the pool and the fixpoint root kept:
/// returns the traced events, the pool (for the `svg` rendering) and the
/// root node of the saturated diagram.
pub fn standard_scenario_full(n: u64) -> (Vec<TraceEvent>, Arena<ReferenceController>, NodeId) {
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
    let policy = PolicySaturation::Static(levels);
    let step = ChainStep {
        number_levels: schema.number_levels(),
    };
    let mut params = SaturationParameters {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        number_levels: schema.number_levels(),
        observer: Some(std::sync::Arc::new(collector.clone())),
    };
    let result = saturate(&mut params).expect("scenario saturation terminates");
    assert_eq!(
        result.iterations, n as u32,
        "the chain saturates in n rounds"
    );
    let events = collector.events.lock().expect("mutex poisoned").clone();
    (events, pool, result.node)
}
