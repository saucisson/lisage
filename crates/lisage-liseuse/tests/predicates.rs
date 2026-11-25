//! D3-2 tests — predicate AST over (event, reconstructed state), the
//! shared evaluator (replay == gate), the controller-side `PathPredicate`
//! port (arc equality on the reference instance, region inclusion on
//! DbmKeys), and the JSON script parser.
//!
//! DoD: the same predicate script runs in replay and at the gate, and a
//! path condition is verified on the DBM instance.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use lisage_core::arena::Arena;
use lisage_core::dbm::{Dbm, DbmController, DbmKeys};
use lisage_core::ddd::compiler_band;
use lisage_core::observer::{Observer, TraceEvent};
use lisage_core::schema::{ABSENT, PRESENT, SchemaKeys};
use lisage_core::{Anchor, Form, Level, NodeId, PathPredicate, Pool};
use lisage_liseuse::debugger::{
    run_standard_scenario, standard_scenario, standard_scenario_with_pool,
};
use lisage_liseuse::mirror::PoolMirror;
use lisage_liseuse::predicates::{
    Evaluator, Json, Predicate, PredicateGate, StateCondition, parse_json, parse_script_dbm,
    parse_script_u64, resolver_from_forms, resolver_from_pool,
};

/// Collecting observer.
#[derive(Clone, Default)]
struct Collector {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Observer for Collector {
    fn on_event(&self, event: &TraceEvent) {
        self.events
            .lock()
            .expect("mutex poisoned")
            .push(event.clone());
    }
}

/// Node→form map of every node visited in the trace, resolved through
/// the pool (replay resolver).
fn forms_of_visited_nodes<P: Pool>(
    pool: &P,
    events: &[TraceEvent],
) -> HashMap<NodeId, Form<P::Key, P::Payload>> {
    let visited: BTreeSet<NodeId> = events
        .iter()
        .filter_map(|event| match event {
            TraceEvent::VisitEntry { node, .. } | TraceEvent::VisitExit { node, .. } => Some(*node),
            _ => None,
        })
        .collect();
    visited
        .into_iter()
        .map(|node| {
            let form = pool.resolve(node).expect("visited node resolvable");
            (node, form)
        })
        .collect()
}

/// The DoD, first half: one script, replay and gate, identical matches.
/// The script exercises all three condition families (event, state,
/// path) on the standard scenario n=4.
#[test]
fn test_same_script_replay_and_gate() {
    let script_text = r#"{
        "all": [
            {"any": [{"event": "op-end"}, {"event": "round"}, {"event": "memo-hit"}]},
            {"any": [
                {"state": {"memo_count_min": 1}},
                {"path": {"any": [[{"level": 0, "arc": 1}]]}}
            ]}
        ]
    }"#;
    let script = parse_script_u64(script_text).expect("script parses");

    // Replay: recorded trace + resolver built from the pool's forms.
    let (events, pool) = standard_scenario_with_pool(4);
    let keys = SchemaKeys::new(lisage_core::schema::Schema::Set(
        lisage_core::schema::Domain::new((0..4).map(lisage_lang::value::Atom).collect()),
    ));
    let resolver = resolver_from_forms(forms_of_visited_nodes(&pool, &events));
    let replayed = Evaluator::replay(script.clone(), keys.clone(), resolver, &events);
    let replay_matches = replayed.matches().to_vec();
    assert!(
        !replay_matches.is_empty(),
        "the script matches somewhere in the n=4 scenario"
    );

    // Gate: the same evaluator plugged live into a fresh run.
    let gate = PredicateGate::new(
        script,
        keys,
        resolver_from_forms(forms_of_visited_nodes(&pool, &events)),
    );
    let gate = Arc::new(gate);
    let _pool = run_standard_scenario(4, gate.clone());
    let gate_matches = gate.matches();

    assert_eq!(
        replay_matches, gate_matches,
        "the same script matches the same positions in replay and at the gate"
    );
}

/// Path condition on the reference instance: arc equality through the
/// port, union of paths, prefix semantics.
#[test]
fn test_path_condition_reference_instance() {
    let (events, pool) = standard_scenario_with_pool(4);
    let keys = SchemaKeys::new(lisage_core::schema::Schema::Set(
        lisage_core::schema::Domain::new((0..4).map(lisage_lang::value::Atom).collect()),
    ));
    let resolver = resolver_from_forms(forms_of_visited_nodes(&pool, &events));

    // Union of both arcs at level 0: every visit below the root matches.
    let both = parse_script_u64(
        r#"{"path": {"any": [[{"level": 0, "arc": 1}], [{"level": 0, "arc": 0}]]}}"#,
    )
    .expect("parses");
    let all_visits = Evaluator::replay(both, keys.clone(), resolver.clone(), &events);
    // Depth of each visit opening (the stack of Visit spans): the root
    // visit of each operation has depth 0 and an empty prefix path.
    let mut depths: Vec<(usize, usize)> = Vec::new();
    let mut stack: Vec<()> = Vec::new();
    for (position, event) in events.iter().enumerate() {
        match event {
            TraceEvent::VisitEntry { .. } => {
                depths.push((position, stack.len()));
                stack.push(());
            }
            TraceEvent::VisitExit { .. } => {
                stack.pop();
            }
            _ => {}
        }
    }
    // The union of both arcs at level 0 matches exactly the visit
    // openings at depth >= 1 (the automaton advances at each visit
    // opening and holds until the visit closes).
    for &(position, depth) in &depths {
        assert_eq!(
            all_visits.matches().contains(&position),
            depth >= 1,
            "visit opening at {position} (depth {depth})"
        );
    }

    // A single path: present arc only — a strict subset of the matches.
    let present_only =
        parse_script_u64(r#"{"path": {"any": [[{"level": 0, "arc": 1}]]}}"#).expect("parses");
    let present_visits = Evaluator::replay(present_only, keys.clone(), resolver.clone(), &events);
    assert!(!present_visits.matches().is_empty());
    assert!(present_visits.matches().len() < all_visits.matches().len());

    // Port semantics: arc equality on the reference instance.
    assert!(keys.arc_in_region(Level(0), &PRESENT, &PRESENT));
    assert!(!keys.arc_in_region(Level(0), &PRESENT, &ABSENT));
}

/// The DoD, second half: a path condition verified on the DBM instance,
/// with region inclusion through the port — same script in replay and
/// at the gate.
#[test]
fn test_path_condition_dbm_instance() {
    const SIZE_DOMAIN: u64 = 4;
    let anchor = Anchor(0);
    let collector = Collector::default();
    let mut pool = Arena::new(
        DbmController::new(DbmKeys::new(SIZE_DOMAIN)),
        Box::new(collector.clone()),
    );
    let root = compiler_band(&mut pool, anchor);
    let events = collector.events.lock().expect("mutex poisoned").clone();

    // Synthetic visit trace: properly nested DFS from the root (each
    // visit opens, recurses into the arcs, then closes).
    fn dfs<P: Pool>(
        pool: &P,
        node: NodeId,
        events: &mut Vec<TraceEvent>,
        visited: &mut BTreeSet<NodeId>,
    ) {
        if !visited.insert(node) {
            return;
        }
        events.push(TraceEvent::VisitEntry { node, span: None });
        if let Form::Node { arcs, .. } = pool.resolve(node).expect("node resolvable") {
            for (_, target) in arcs.iter() {
                dfs(pool, *target, events, visited);
            }
        }
        events.push(TraceEvent::VisitExit {
            node,
            result: node,
            span: None,
        });
    }
    let mut visit_events: Vec<TraceEvent> = Vec::new();
    dfs(&pool, root, &mut visit_events, &mut BTreeSet::new());
    // The pool-level events (NodeCreated) plus the visit trace.
    let mut trace = events.clone();
    trace.extend(visit_events);

    let keys = DbmKeys::new(SIZE_DOMAIN);
    let forms = forms_of_visited_nodes(&pool, &trace);
    let resolver = resolver_from_forms(forms);

    // Full box region: includes every arc — the path matches.
    let full_box =
        r#"{"path": {"any": [[{"level": 0, "arc": {"dbm": {"vars": 2, "constraints": []}}}]]}}"#;
    let script_full = parse_script_dbm(full_box).expect("parses");
    let replayed = Evaluator::replay(script_full.clone(), keys.clone(), resolver.clone(), &trace);
    assert!(
        !replayed.matches().is_empty(),
        "the full-box region includes the band arc at level 0"
    );

    // Same script at the gate: identical matches.
    let gate = Arc::new(PredicateGate::new(
        script_full,
        keys.clone(),
        resolver.clone(),
    ));
    for event in &trace {
        gate.on_event(event);
    }
    assert_eq!(
        replayed.matches(),
        gate.matches().as_slice(),
        "same script, same matches: replay == gate on the DBM instance"
    );

    // Diagonal region x−y ≤ 0 ∧ y−x ≤ 0: the band arc x−y ≤ 1 is NOT
    // included — no match.
    let diagonal = r#"{"path": {"any": [[{"level": 0, "arc": {"dbm": {"vars": 2, "constraints": [[0, 1, 0], [1, 0, 0]]}}}]]}}"#;
    let script_diagonal = parse_script_dbm(diagonal).expect("parses");
    let rejected = Evaluator::replay(script_diagonal, keys.clone(), resolver, &trace);
    assert!(
        rejected.matches().is_empty(),
        "region inclusion rejects the band arc against the diagonal"
    );

    // Port semantics directly: inclusion, not equality.
    let band = Dbm::new(2, &[(0, 1, 1)]);
    let full = Dbm::new(2, &[]);
    let diagonal_region = Dbm::new(2, &[(0, 1, 0), (1, 0, 0)]);
    assert!(
        keys.arc_in_region(Level(0), &full, &band),
        "band ⊆ full box"
    );
    assert!(keys.arc_in_region(Level(0), &band, &band), "band ⊆ band");
    assert!(
        !keys.arc_in_region(Level(0), &diagonal_region, &band),
        "band ⊄ diagonal"
    );
}

/// State valuation is evaluated only at OpEnd/Round events.
#[test]
fn test_state_valuation_only_at_op_end_and_round() {
    let events = standard_scenario(4);
    let keys = SchemaKeys::new(lisage_core::schema::Schema::Set(
        lisage_core::schema::Domain::new((0..4).map(lisage_lang::value::Atom).collect()),
    ));
    let resolver = resolver_from_forms::<u64, bool>(HashMap::new());

    let script = parse_script_u64(r#"{"state": {"memo_count_min": 0}}"#).expect("parses");
    let evaluated = Evaluator::replay(script, keys, resolver, &events);
    assert!(!evaluated.matches().is_empty());
    for &position in evaluated.matches() {
        assert!(
            matches!(
                events[position],
                TraceEvent::OpEnd { .. } | TraceEvent::Round { .. }
            ),
            "state conditions evaluate only at OpEnd/Round, got {:?} at {position}",
            std::mem::discriminant(&events[position])
        );
    }

    // Combined with an event kind: memo_count_min 1 at OpEnd only.
    let script =
        parse_script_u64(r#"{"all": [{"event": "op-end"}, {"state": {"memo_count_min": 1}}]}"#)
            .expect("parses");
    let keys = SchemaKeys::new(lisage_core::schema::Schema::Set(
        lisage_core::schema::Domain::new((0..4).map(lisage_lang::value::Atom).collect()),
    ));
    let evaluated = Evaluator::replay(
        script,
        keys,
        resolver_from_forms::<u64, bool>(HashMap::new()),
        &events,
    );
    for &position in evaluated.matches() {
        assert!(matches!(events[position], TraceEvent::OpEnd { .. }));
    }
}

/// The live-pool resolver (the gate's resolver, D4) resolves one arc.
#[test]
fn test_resolver_from_pool() {
    const SIZE_DOMAIN: u64 = 4;
    let anchor = Anchor(0);
    let mut pool = Arena::new(
        DbmController::new(DbmKeys::new(SIZE_DOMAIN)),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let root = compiler_band(&mut pool, anchor);
    let shared = Arc::new(Mutex::new(pool));
    let resolver = resolver_from_pool(shared.clone());
    let (level, arc_key, target) = {
        let pool = shared.lock().expect("mutex poisoned");
        let Form::Node { level, arcs } = pool.resolve(root).expect("root resolvable") else {
            panic!("the root is a node")
        };
        let (arc_key, target) = arcs.iter().next().expect("the root has arcs");
        (level, arc_key.clone(), *target)
    };
    let resolved = resolver(root, target).expect("arc resolved");
    assert_eq!(resolved, (level, arc_key));
}

/// JSON mini-parser: values, escapes, and the strictness rules (integers
/// only, no trailing characters).
#[test]
fn test_parse_json() {
    let json =
        parse_json(r#"{"a": [1, -2, "x\ny\"z", true, null], "b": {"c": []}}"#).expect("parses");
    assert_eq!(
        json.get("a").and_then(Json::as_arr).map(<[_]>::len),
        Some(5)
    );
    assert_eq!(
        json.get("a").and_then(Json::as_arr).map(|a| a[2].clone()),
        Some(Json::Str("x\ny\"z".to_string()))
    );
    assert_eq!(
        json.get("b")
            .and_then(|b| b.get("c"))
            .and_then(Json::as_arr)
            .map(<[_]>::len),
        Some(0)
    );
    assert!(parse_json("[1.5]").is_err(), "floats rejected");
    assert!(parse_json("[1] x").is_err(), "trailing characters rejected");
    assert!(parse_json("{\"a\":}").is_err());
}

/// Script parsing: AST shape and error paths.
#[test]
fn test_parse_script() {
    let script = parse_script_u64(
        r#"{"all": [{"not": {"event": "round"}}, {"node": 7}, {"node_level": 2}]}"#,
    )
    .expect("parses");
    assert_eq!(
        script,
        Predicate::And(vec![
            Predicate::Not(Box::new(Predicate::EventKind(
                lisage_liseuse::debugger::EventKind::Round
            ))),
            Predicate::NodeId(NodeId(7)),
            Predicate::NodeLevel(2),
        ])
    );
    assert!(parse_script_u64(r#"{"state": {"memo_count_min": 1}}"#).is_ok());
    assert!(matches!(
        parse_script_u64(r#"{"state": {"memo_count_min": 1}}"#).unwrap(),
        Predicate::State(StateCondition::MemoCountAtLeast(1))
    ));
    assert!(parse_script_u64(r#"{"unknown": 1}"#).is_err());
    assert!(parse_script_u64(r#"{"event": "nope"}"#).is_err());
    assert!(parse_script_u64(r#"{"path": {"any": [[{"level": 0, "arc": "x"}]]}}"#).is_err());
}

/// The evaluator's mirror tracks the reconstructed state along the trace.
#[test]
fn test_evaluator_mirror_tracks_state() {
    let events = standard_scenario(4);
    let keys = SchemaKeys::new(lisage_core::schema::Schema::Set(
        lisage_core::schema::Domain::new((0..4).map(lisage_lang::value::Atom).collect()),
    ));
    let script = parse_script_u64(r#"{"event": "round"}"#).expect("parses");
    let evaluated = Evaluator::replay(
        script,
        keys,
        resolver_from_forms::<u64, bool>(HashMap::new()),
        &events,
    );
    assert_eq!(evaluated.matches().len(), 4, "n=4 -> 4 rounds");
    assert_eq!(evaluated.position(), events.len());
    let mirror = evaluated.mirror();
    let expected = PoolMirror::build(&events);
    assert_eq!(mirror.memo_count(), expected.memo_count());
    assert_eq!(mirror.memory_version(), expected.memory_version());
}
