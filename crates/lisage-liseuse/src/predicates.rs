//! D3-2: predicate AST over (event, reconstructed state) and the shared
//! evaluator, used **identically in replay and at the gate** (issue #165).
//!
//! A predicate script is a small JSON document (hand-written parser, no
//! external dependency — same policy as the trace serializer):
//!
//! ```json
//! {"all": [
//!   {"event": "op-end"},
//!   {"state": {"memo_count_min": 1}},
//!   {"path": {"any": [[{"level": 0, "arc": 1}]]}}
//! ]}
//! ```
//!
//! Vocabulary: `all`/`any`/`not` (boolean combinators), `event` (an
//! [`EventKind`] name), `node` (node id of the event), `node_level`,
//! `state` (a [`StateCondition`], evaluated **only at `OpEnd`/`Round`
//! events**), `path` (a [`PathExpr`]: a union of prefix paths, each step
//! a `{"level", "arc"}` region). Arc keys are algebra-specific: a JSON
//! number for the reference instance (`u64`), a `{"dbm": ...}` object for
//! `DbmKeys`.
//!
//! The [`Evaluator`] advances an incremental automaton at each visit
//! opening (`VisitEntry`): the prefix path is the sequence of arcs from
//! the root to the visited node, resolved through an [`ArcResolver`]
//! (a node→form map in replay, the live pool at the gate). Membership of
//! an arc in a path region goes through the controller-side
//! [`PathPredicate`] port: arc equality for the reference instance,
//! region inclusion for `DbmKeys`. [`PredicateGate`] wraps the same
//! evaluator in a mutex and implements [`Observer`], so the gate and the
//! replay run the exact same `on_event` code path.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use lisage_core::dbm::Dbm;
use lisage_core::observer::{Observer, TraceEvent};
use lisage_core::{Form, Level, NodeId, PathPredicate, Pool};

use crate::debugger::EventKind;
use crate::mirror::PoolMirror;

/// Resolves the arc `(level, key)` from a parent node to a child node.
/// In replay: a node→form map snapshot. At the gate: the live pool (D4).
pub type ArcResolver<K> = Arc<dyn Fn(NodeId, NodeId) -> Option<(Level, K)> + Send + Sync>;

/// Builds an arc resolver from an owned node→form map (replay: the map is
/// a snapshot of the pool taken after the run).
pub fn resolver_from_forms<K, P>(forms: HashMap<NodeId, Form<K, P>>) -> ArcResolver<K>
where
    K: Clone + Ord + Hash + Debug + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    Arc::new(move |parent, child| match forms.get(&parent)? {
        Form::Node { level, arcs } => arcs
            .iter()
            .find(|(_, target)| **target == child)
            .map(|(key, _)| (*level, key.clone())),
        Form::Terminal(_) => None,
    })
}

/// Builds an arc resolver over a shared live pool (the gate, D4: the
/// engine runs in its own thread, the resolver locks the pool only to
/// resolve one arc at a visit opening).
pub fn resolver_from_pool<P: Pool + 'static>(pool: Arc<Mutex<P>>) -> ArcResolver<P::Key> {
    Arc::new(move |parent, child| {
        let pool = pool.lock().expect("mutex poisoned");
        match pool.resolve(parent).ok()? {
            Form::Node { level, arcs } => arcs
                .iter()
                .find(|(_, target)| **target == child)
                .map(|(key, _)| (level, key.clone())),
            Form::Terminal(_) => None,
        }
    })
}

/// State valuation condition (D3-2), evaluated at `OpEnd`/`Round` events
/// against the reconstructed [`PoolMirror`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateCondition {
    /// Current memory version equals this value.
    MemoryVersion(u64),
    /// Memo table holds at least this many entries.
    MemoCountAtLeast(usize),
    /// Live node count equals this value.
    LiveCountEq(usize),
}

impl StateCondition {
    fn matches(&self, mirror: &PoolMirror) -> bool {
        match self {
            StateCondition::MemoryVersion(version) => mirror.memory_version() == *version,
            StateCondition::MemoCountAtLeast(min) => mirror.memo_count() >= *min,
            StateCondition::LiveCountEq(count) => mirror.live_count() == *count,
        }
    }
}

/// One step of a path: the region expected at `level`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathStep<K> {
    /// Level of the step.
    pub level: Level,
    /// Region the arc must belong to (controller-side semantics).
    pub region: K,
}

/// Path expression (D3-2): a union of prefix paths ("or" of paths = union
/// of regions). A path matches the current prefix path iff it is a
/// prefix of it, step by step, through [`PathPredicate::arc_in_region`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathExpr<K> {
    /// The union of paths (each path a sequence of steps).
    pub paths: Vec<Vec<PathStep<K>>>,
}

impl<K: Clone + Ord + Hash + Debug + Send + Sync> PathExpr<K> {
    /// Prefix matching: some path of the union is a prefix of `path`
    /// (an unknown arc — `None` — matches no region).
    pub fn matches_prefix<A: PathPredicate<Key = K>>(
        &self,
        path: &[Option<(Level, K)>],
        algebra: &A,
    ) -> bool {
        self.paths.iter().any(|candidate| {
            candidate.len() <= path.len()
                && candidate
                    .iter()
                    .zip(path.iter())
                    .all(|(step, arc)| match arc {
                        Some((level, key)) => {
                            step.level == *level
                                && algebra.arc_in_region(step.level, &step.region, key)
                        }
                        None => false,
                    })
        })
    }
}

/// Predicate AST (D3-2) over (event, reconstructed state, prefix path).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Predicate<K> {
    /// Event-kind condition.
    EventKind(EventKind),
    /// The event carries this node id.
    NodeId(NodeId),
    /// The event's node is at this level.
    NodeLevel(u32),
    /// State valuation (evaluated only at `OpEnd`/`Round`).
    State(StateCondition),
    /// Prefix-path condition.
    Path(PathExpr<K>),
    /// Conjunction.
    And(Vec<Predicate<K>>),
    /// Disjunction.
    Or(Vec<Predicate<K>>),
    /// Negation.
    Not(Box<Predicate<K>>),
}

/// Node id carried by an event, if any.
fn event_node(event: &TraceEvent) -> Option<NodeId> {
    match event {
        TraceEvent::NodeCreated { node, .. }
        | TraceEvent::NodeShared { node, .. }
        | TraceEvent::MemoHit { node, .. }
        | TraceEvent::MemoMiss { node, .. }
        | TraceEvent::VisitEntry { node, .. }
        | TraceEvent::VisitExit { node, .. }
        | TraceEvent::MemoConsult { node, .. }
        | TraceEvent::MemoInsert { node, .. }
        | TraceEvent::MemoryRead { node, .. }
        | TraceEvent::StructureChange { node, .. } => Some(*node),
        _ => None,
    }
}

/// Level of the event's node, if any.
fn event_level(event: &TraceEvent) -> Option<u32> {
    match event {
        TraceEvent::NodeCreated { level, .. } => level.map(|level| level.0),
        // The level of the consulted memory key (ADR-0109 tracks).
        TraceEvent::MemoryRead { level, .. } => Some(*level),
        _ => None,
    }
}

impl<K: Clone + Ord + Hash + Debug + Send + Sync> Predicate<K> {
    /// Evaluates the predicate on one event, with the reconstructed state
    /// and the current prefix path.
    pub fn matches<A: PathPredicate<Key = K>>(
        &self,
        event: &TraceEvent,
        mirror: &PoolMirror,
        path: &[Option<(Level, K)>],
        algebra: &A,
    ) -> bool {
        match self {
            Predicate::EventKind(kind) => kind.matches(event),
            Predicate::NodeId(node) => event_node(event) == Some(*node),
            Predicate::NodeLevel(level) => event_level(event) == Some(*level),
            Predicate::State(condition) => match event {
                TraceEvent::OpEnd { .. } | TraceEvent::Round { .. } => condition.matches(mirror),
                _ => false,
            },
            Predicate::Path(expression) => expression.matches_prefix(path, algebra),
            Predicate::And(predicates) => predicates
                .iter()
                .all(|p| p.matches(event, mirror, path, algebra)),
            Predicate::Or(predicates) => predicates
                .iter()
                .any(|p| p.matches(event, mirror, path, algebra)),
            Predicate::Not(predicate) => !predicate.matches(event, mirror, path, algebra),
        }
    }
}

/// One frame of the visit stack: the visited node and the arc taken from
/// its parent (`None` at the root — the empty prefix path).
struct VisitFrame<K> {
    node: NodeId,
    arc: Option<(Level, K)>,
}

/// The shared evaluator (D3-2): feeds events one by one through
/// [`Evaluator::on_event`], whether they come from a recorded trace
/// (replay) or from a live pool (gate, via [`PredicateGate`]).
pub struct Evaluator<A: PathPredicate> {
    script: Predicate<A::Key>,
    algebra: A,
    resolver: ArcResolver<A::Key>,
    mirror: PoolMirror,
    visits: Vec<VisitFrame<A::Key>>,
    position: usize,
    matches: Vec<usize>,
}

impl<A: PathPredicate> Evaluator<A> {
    /// New evaluator over `script`, with the controller-side algebra and
    /// the arc resolver.
    pub fn new(script: Predicate<A::Key>, algebra: A, resolver: ArcResolver<A::Key>) -> Self {
        Evaluator {
            script,
            algebra,
            resolver,
            mirror: PoolMirror::default(),
            visits: Vec::new(),
            position: 0,
            matches: Vec::new(),
        }
    }

    /// Replay helper: builds the evaluator and feeds it the whole trace.
    pub fn replay(
        script: Predicate<A::Key>,
        algebra: A,
        resolver: ArcResolver<A::Key>,
        events: &[TraceEvent],
    ) -> Self {
        let mut evaluator = Self::new(script, algebra, resolver);
        for event in events {
            evaluator.on_event(event);
        }
        evaluator
    }

    /// Feeds one event: updates the mirror and the incremental path
    /// automaton (at each visit opening), evaluates the script, records
    /// the position when it matches. Returns whether it matched.
    pub fn on_event(&mut self, event: &TraceEvent) -> bool {
        self.mirror.apply(event);
        match event {
            TraceEvent::VisitEntry { node, .. } => {
                let arc = match self.visits.last() {
                    None => None,
                    Some(parent) => (self.resolver)(parent.node, *node),
                };
                self.visits.push(VisitFrame { node: *node, arc });
            }
            TraceEvent::VisitExit { .. } => {
                self.visits.pop();
            }
            _ => {}
        }
        let path: Vec<Option<(Level, A::Key)>> = self
            .visits
            .iter()
            .skip(1)
            .map(|frame| frame.arc.clone())
            .collect();
        let matched = self
            .script
            .matches(event, &self.mirror, &path, &self.algebra);
        if matched {
            self.matches.push(self.position);
        }
        self.position += 1;
        matched
    }

    /// Positions (in event order) where the script matched.
    pub fn matches(&self) -> &[usize] {
        &self.matches
    }

    /// Current event position (number of events fed so far).
    pub fn position(&self) -> usize {
        self.position
    }

    /// The reconstructed state.
    pub fn mirror(&self) -> &PoolMirror {
        &self.mirror
    }
}

/// The gate (D3-2): the same [`Evaluator`] behind a mutex, implementing
/// [`Observer`] so it can be plugged into a live pool — replay and gate
/// run the identical `on_event` code path.
pub struct PredicateGate<A: PathPredicate> {
    evaluator: Mutex<Evaluator<A>>,
}

impl<A: PathPredicate> PredicateGate<A> {
    /// New gate over `script`.
    pub fn new(script: Predicate<A::Key>, algebra: A, resolver: ArcResolver<A::Key>) -> Self {
        PredicateGate {
            evaluator: Mutex::new(Evaluator::new(script, algebra, resolver)),
        }
    }

    /// Positions where the script matched so far.
    pub fn matches(&self) -> Vec<usize> {
        self.evaluator
            .lock()
            .expect("mutex poisoned")
            .matches()
            .to_vec()
    }
}

impl<A: PathPredicate + Send + Sync> Observer for PredicateGate<A> {
    fn on_event(&self, event: &TraceEvent) {
        self.evaluator
            .lock()
            .expect("mutex poisoned")
            .on_event(event);
    }
}

/// Minimal JSON value (hand-written parser, no external dependency —
/// same policy as the trace serializer).
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// Boolean.
    Bool(bool),
    /// Integer (the predicate scripts carry no floats).
    Num(i64),
    /// String.
    Str(String),
    /// Array.
    Arr(Vec<Json>),
    /// Object (insertion order preserved).
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Object field lookup.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// Integer value.
    pub fn as_num(&self) -> Option<i64> {
        match self {
            Json::Num(value) => Some(*value),
            _ => None,
        }
    }

    /// String value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(value) => Some(value),
            _ => None,
        }
    }

    /// Array value.
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

/// Recursive-descent JSON parser over a char vector.
struct JsonParser {
    chars: Vec<char>,
    position: usize,
}

impl JsonParser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.position += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next() {
            Some(c) if c == expected => Ok(()),
            other => Err(format!("expected '{expected}', got {other:?}")),
        }
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => Ok(Json::Str(self.parse_string()?)),
            Some('t') => self.parse_literal("true", Json::Bool(true)),
            Some('f') => self.parse_literal("false", Json::Bool(false)),
            Some('n') => self.parse_literal("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            other => Err(format!("unexpected character {other:?}")),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: Json) -> Result<Json, String> {
        for expected in literal.chars() {
            self.expect(expected)?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Json, String> {
        let mut text = String::new();
        if self.peek() == Some('-') {
            text.push('-');
            self.position += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.position += 1;
            } else {
                break;
            }
        }
        if text.is_empty() || text == "-" {
            return Err("invalid number".to_string());
        }
        if matches!(self.peek(), Some('.' | 'e' | 'E')) {
            return Err("floats are not part of the predicate script format".to_string());
        }
        text.parse::<i64>()
            .map(Json::Num)
            .map_err(|error| format!("invalid number '{text}': {error}"))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut output = String::new();
        loop {
            match self.next() {
                None => return Err("unterminated string".to_string()),
                Some('"') => return Ok(output),
                Some('\\') => match self.next() {
                    Some('"') => output.push('"'),
                    Some('\\') => output.push('\\'),
                    Some('/') => output.push('/'),
                    Some('n') => output.push('\n'),
                    Some('t') => output.push('\t'),
                    Some('r') => output.push('\r'),
                    Some('b') => output.push('\u{0008}'),
                    Some('f') => output.push('\u{000C}'),
                    Some('u') => {
                        let mut code = 0u32;
                        for _ in 0..4 {
                            let digit = self
                                .next()
                                .and_then(|c| c.to_digit(16))
                                .ok_or("invalid \\u escape")?;
                            code = code * 16 + digit;
                        }
                        output.push(char::from_u32(code).ok_or("invalid \\u code point")?);
                    }
                    other => return Err(format!("invalid escape {other:?}")),
                },
                Some(c) => output.push(c),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.position += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.next() {
                Some(',') => {}
                Some(']') => return Ok(Json::Arr(items)),
                other => return Err(format!("expected ',' or ']', got {other:?}")),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.position += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.next() {
                Some(',') => {}
                Some('}') => return Ok(Json::Obj(entries)),
                other => return Err(format!("expected ',' or '}}', got {other:?}")),
            }
        }
    }
}

/// Parses a JSON document (integers only — the script format carries no
/// floats).
pub fn parse_json(text: &str) -> Result<Json, String> {
    let mut parser = JsonParser {
        chars: text.chars().collect(),
        position: 0,
    };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.position != parser.chars.len() {
        return Err("trailing characters after the JSON document".to_string());
    }
    Ok(value)
}

/// Parses a predicate script (JSON) into a [`Predicate`], with the
/// algebra-specific arc-key parser `parse_key` (a JSON number for the
/// reference instance, a `{"dbm": ...}` object for `DbmKeys`).
pub fn parse_script<K>(
    json_text: &str,
    parse_key: &dyn Fn(&Json) -> Option<K>,
) -> Result<Predicate<K>, String> {
    let json = parse_json(json_text)?;
    parse_predicate(&json, parse_key)
}

/// Parses a predicate script for the reference instance (`u64` arc keys:
/// JSON numbers).
pub fn parse_script_u64(json_text: &str) -> Result<Predicate<u64>, String> {
    parse_script(json_text, &|json| json.as_num().map(|n| n as u64))
}

/// Parses a predicate script for the DBM instance: an arc key is
/// `{"dbm": {"vars": n, "constraints": [[i, j, c], ...]}}` with
/// `x_i − x_j ≤ c`.
pub fn parse_script_dbm(json_text: &str) -> Result<Predicate<Dbm>, String> {
    parse_script(json_text, &parse_dbm_key)
}

/// Parses one DBM region from its JSON encoding.
pub fn parse_dbm_key(json: &Json) -> Option<Dbm> {
    let dbm = json.get("dbm")?;
    let vars = dbm.get("vars")?.as_num()? as usize;
    let constraints_json = dbm.get("constraints")?.as_arr()?;
    let mut constraints = Vec::new();
    for entry in constraints_json {
        let triple = entry.as_arr()?;
        if triple.len() != 3 {
            return None;
        }
        constraints.push((
            triple[0].as_num()? as usize,
            triple[1].as_num()? as usize,
            triple[2].as_num()?,
        ));
    }
    Some(Dbm::new(vars, &constraints))
}

fn parse_predicate<K>(
    json: &Json,
    parse_key: &dyn Fn(&Json) -> Option<K>,
) -> Result<Predicate<K>, String> {
    if let Some(value) = json.get("all") {
        let items = value
            .as_arr()
            .ok_or("'all' expects an array")?
            .iter()
            .map(|item| parse_predicate(item, parse_key))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Predicate::And(items));
    }
    if let Some(value) = json.get("any") {
        let items = value
            .as_arr()
            .ok_or("'any' expects an array")?
            .iter()
            .map(|item| parse_predicate(item, parse_key))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Predicate::Or(items));
    }
    if let Some(value) = json.get("not") {
        return Ok(Predicate::Not(Box::new(parse_predicate(value, parse_key)?)));
    }
    if let Some(value) = json.get("event") {
        let name = value.as_str().ok_or("'event' expects a string")?;
        let kind = EventKind::parse(name).ok_or_else(|| format!("unknown event kind '{name}'"))?;
        return Ok(Predicate::EventKind(kind));
    }
    if let Some(value) = json.get("node") {
        let id = value.as_num().ok_or("'node' expects a number")?;
        return Ok(Predicate::NodeId(NodeId(
            u32::try_from(id).map_err(|_| "node id out of range".to_string())?,
        )));
    }
    if let Some(value) = json.get("node_level") {
        let level = value.as_num().ok_or("'node_level' expects a number")?;
        return Ok(Predicate::NodeLevel(
            u32::try_from(level).map_err(|_| "level out of range".to_string())?,
        ));
    }
    if let Some(value) = json.get("state") {
        return Ok(Predicate::State(parse_state(value)?));
    }
    if let Some(value) = json.get("path") {
        return Ok(Predicate::Path(parse_path(value, parse_key)?));
    }
    Err(
        "unknown predicate: expected one of all/any/not/event/node/node_level/state/path"
            .to_string(),
    )
}

fn parse_state(json: &Json) -> Result<StateCondition, String> {
    if let Some(value) = json.get("memory_version") {
        let version = value.as_num().ok_or("'memory_version' expects a number")?;
        return Ok(StateCondition::MemoryVersion(
            u64::try_from(version).map_err(|_| "version out of range".to_string())?,
        ));
    }
    if let Some(value) = json.get("memo_count_min") {
        let min = value.as_num().ok_or("'memo_count_min' expects a number")?;
        return Ok(StateCondition::MemoCountAtLeast(
            usize::try_from(min).map_err(|_| "count out of range".to_string())?,
        ));
    }
    if let Some(value) = json.get("live_count") {
        let count = value.as_num().ok_or("'live_count' expects a number")?;
        return Ok(StateCondition::LiveCountEq(
            usize::try_from(count).map_err(|_| "count out of range".to_string())?,
        ));
    }
    Err("unknown state condition: expected memory_version/memo_count_min/live_count".to_string())
}

fn parse_path<K>(
    json: &Json,
    parse_key: &dyn Fn(&Json) -> Option<K>,
) -> Result<PathExpr<K>, String> {
    let union = json
        .get("any")
        .and_then(Json::as_arr)
        .ok_or("'path' expects {\"any\": [[step, ...], ...]}")?;
    let mut paths = Vec::new();
    for path_json in union {
        let steps_json = path_json
            .as_arr()
            .ok_or("each path expects an array of steps")?;
        let mut steps = Vec::new();
        for step_json in steps_json {
            let level = step_json
                .get("level")
                .and_then(Json::as_num)
                .ok_or("path step expects a 'level' number")?;
            let arc_json = step_json
                .get("arc")
                .ok_or("path step expects an 'arc' key")?;
            let region = parse_key(arc_json).ok_or("unparseable arc key")?;
            steps.push(PathStep {
                level: Level(u32::try_from(level).map_err(|_| "level out of range".to_string())?),
                region,
            });
        }
        paths.push(steps);
    }
    Ok(PathExpr { paths })
}
