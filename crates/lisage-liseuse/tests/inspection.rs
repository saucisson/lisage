//! D3-3 tests — inspection views of the debugger: node, memo (L1
//! invalidation), memory, diff, eval, display, ops, and the current SVG
//! rendering. DoD: an L1 invalidation (version bump → dead memo entries)
//! visible in one `memo` command.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use lisage_core::arena::Arena;
use lisage_core::memory::EffetMemory;
use lisage_core::observer::{Observer, StructureChangeKind, TraceEvent};
use lisage_core::schema::{Domain, ReferenceController, Schema};
use lisage_core::transduction::{ContextePhiOut, Transducteur, Transduction};
use lisage_core::{Anchor, Form, Level, MemoryPolicy, NodeId, Pool};
use lisage_lang::value::Atom;
use lisage_liseuse::debugger::{
    Command, Debugger, parse_command, round_numbers, standard_scenario,
};
use lisage_liseuse::inspection::{
    Inspector, MemoTable, render_current_svg, standard_scenario_full,
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

/// Transduction that writes into the level memory at each node visit
/// (`M[id(n), ℓ] ← h`): each write bumps the L1 version, so memo entries
/// inserted at an older version die (DoD).
struct WriteMemory;

impl Transduction for WriteMemory {
    type H = u64;
    type S = u64;
    type Out = bool;
    type M = u64;

    fn fingerprint(&self) -> u64 {
        3001
    }
    fn h0(&self) -> u64 {
        0
    }
    fn phi_disp(&self, _level: Level, h: &u64, _arc: u64) -> u64 {
        h + 1
    }
    fn phi_aggr(&self, _level: Level, _h: &u64, arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).max().unwrap_or(0)
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffetMemory<u64>) {
        let effet = EffetMemory::one(ctx.node_input, ctx.level, *ctx.h);
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            effet,
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), 0)
    }
}

/// A real traced run with memory writes: two non-terminal nodes, each
/// writing the level memory (two L1 version bumps).
fn memory_scenario() -> Vec<TraceEvent> {
    let schema = Schema::Set(Domain::new(vec![Atom(0), Atom(1)]));
    let mut input = Arena::new(
        ReferenceController::new(schema),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let mut output = Arena::new(
        ReferenceController::new(Schema::Set(Domain::new(vec![Atom(0), Atom(1)]))),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let anchor = Anchor(0);
    let true_node = input.intern(anchor, Form::Terminal(true));
    let false_node = input.intern(anchor, Form::Terminal(false));
    let a = input.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, true_node), (1, false_node)]).into(),
        },
    );
    let b = input.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, a), (1, true_node)]).into(),
        },
    );
    let collector = Collector::default();
    let mut runner = Transducteur::with_observer(
        WriteMemory,
        MemoryPolicy::PerLevel,
        Arc::new(collector.clone()),
    );
    runner.run(&input, &mut output, anchor, b).expect("run ok");
    assert!(
        runner.version_memory() >= 2,
        "two writes -> at least two version bumps"
    );
    collector.events.lock().expect("mutex poisoned").clone()
}

/// The DoD: one `memo` command shows the L1 invalidation — memo entries
/// inserted before the version bump are dead.
#[test]
fn test_memo_l1_invalidation_one_command() {
    let events = memory_scenario();
    let writes = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::EcritureMemoire { .. }))
        .count();
    assert!(writes >= 2, "the scenario really bumps the version");
    let mut debugger = Debugger::new(events);
    debugger.run(Command::Continue);
    let output = debugger.run(Command::Memo);
    println!("{output}");
    assert!(
        output.contains("dead"),
        "the memo view reports dead entries"
    );
    let dead = output.lines().filter(|l| l.starts_with("dead:")).count();
    assert!(dead >= 1, "at least one entry invalidated by the bump");
    assert!(output.contains("version 2"), "current version is 2");
}

/// Precise memo split on a hand-built trace: insert at v0, bump to v1,
/// insert at v1 → 1 live, 1 dead.
#[test]
fn test_memo_table_live_dead_split() {
    let events = vec![
        TraceEvent::MemoInsert {
            fingerprint: 7,
            node: NodeId(3),
            h: lisage_core::observer::TraceVal::Elide,
            version: 0,
            resultat: NodeId(5),
            span: None,
        },
        TraceEvent::EcritureMemoire {
            version_avant: 0,
            version_apres: 1,
            span: None,
        },
        TraceEvent::MemoInsert {
            fingerprint: 7,
            node: NodeId(4),
            h: lisage_core::observer::TraceVal::Elide,
            version: 1,
            resultat: NodeId(6),
            span: None,
        },
    ];
    let table = MemoTable::replay(&events, events.len());
    assert_eq!(table.version, 1);
    assert_eq!(table.live().count(), 1);
    assert_eq!(table.dead().count(), 1);
    assert_eq!(table.dead().next().unwrap().node, NodeId(3));
    let inspector = Inspector::new(events);
    let view = inspector.memo_view(3);
    assert!(view.contains("version 1, live 1, dead 1"), "{view}");
    assert!(
        view.contains("dead: op 7 node NodeId(3) v0 -> NodeId(5)"),
        "{view}"
    );
}

/// `memory`: version history.
#[test]
fn test_memory_view() {
    let events = vec![
        TraceEvent::EcritureMemoire {
            version_avant: 0,
            version_apres: 1,
            span: None,
        },
        TraceEvent::EcritureMemoire {
            version_avant: 1,
            version_apres: 2,
            span: None,
        },
    ];
    let inspector = Inspector::new(events);
    let view = inspector.memory_view(2);
    assert!(view.contains("version 2"), "{view}");
    assert!(
        view.contains("0->1 @ 0") && view.contains("1->2 @ 1"),
        "{view}"
    );
}

/// `node <id>`: creation level/form and index ranges on the real scenario.
#[test]
fn test_node_view() {
    let events = standard_scenario(4);
    let node = events
        .iter()
        .find_map(|e| match e {
            TraceEvent::VisiteEntree { node, .. } => Some(*node),
            _ => None,
        })
        .expect("the scenario visits nodes");
    let inspector = Inspector::new(events);
    let view = inspector.node_view(10, node);
    assert!(view.contains("node"), "{view}");
    assert!(view.contains("ranges ["), "{view}");
    // Unknown node: no creation.
    let view = inspector.node_view(10, NodeId(999_999));
    assert!(view.contains("not created"), "{view}");
}

/// `diff`: structure-change fragments at (or after) the cursor.
#[test]
fn test_diff_view() {
    let events = vec![
        TraceEvent::OpStart {
            fingerprint: 1,
            span: None,
        },
        TraceEvent::StructureChange {
            node: NodeId(7),
            classification: StructureChangeKind::Collapse,
            arcs_avant: vec![(0, NodeId(3)), (1, NodeId(4))],
            arcs_apres: vec![(1, NodeId(4))],
            span: None,
        },
    ];
    let inspector = Inspector::new(events);
    let view = inspector.diff_view(0);
    assert!(view.contains("node NodeId(7)"), "{view}");
    assert!(view.contains("Collapse"), "{view}");
    assert!(
        view.contains("avant: [0->NodeId(3), 1->NodeId(4)]"),
        "{view}"
    );
    assert!(view.contains("apres: [1->NodeId(4)]"), "{view}");
    // No StructureChange after the cursor: explicit message.
    let view = inspector.diff_view(2);
    assert!(view.contains("no StructureChange"), "{view}");
}

/// `ops`: the operations of the standard scenario, current one marked.
#[test]
fn test_ops_view() {
    let events = standard_scenario(4);
    let inspector = Inspector::new(events.clone());
    let view = inspector.ops_view(0);
    assert!(view.contains("4 ops"), "{view}");
    assert!(view.contains("fingerprint"), "{view}");
    // Inside the first op: marked current.
    let first_op_start = events
        .iter()
        .position(|e| matches!(e, TraceEvent::OpStart { .. }))
        .unwrap();
    let view = inspector.ops_view(first_op_start);
    assert!(view.contains("(current)"), "{view}");
}

/// `display`: dashboard line with the counters.
#[test]
fn test_display() {
    let events = standard_scenario(4);
    let inspector = Inspector::new(events);
    let view = inspector.display(10, 2, 1);
    assert!(view.contains("display @ pos 10/"), "{view}");
    assert!(view.contains("spans 2 visits 1"), "{view}");
    assert!(view.contains("rounds"), "{view}");
}

/// `eval <valuation>`: arithmetic over the counters, and error paths.
#[test]
fn test_eval() {
    let events = standard_scenario(4);
    let inspector = Inspector::new(events);
    assert_eq!(inspector.eval(10, "1 + 2 * 3"), Ok(7));
    assert_eq!(inspector.eval(10, "(1 + 2) * 3"), Ok(9));
    assert_eq!(inspector.eval(10, "-4 + 10 / 2"), Ok(1));
    assert_eq!(inspector.eval(10, "10 % 3"), Ok(1));
    assert_eq!(inspector.eval(10, "position + 0"), Ok(10));
    assert_eq!(
        inspector.eval(10, "length - position"),
        Ok(inspector.length() as i64 - 10)
    );
    assert!(
        inspector.eval(10, "live + memo_dead >= 0").is_err(),
        "no comparison operators"
    );
    assert!(inspector.eval(10, "1 / 0").is_err());
    assert!(inspector.eval(10, "nope + 1").is_err());
    assert!(inspector.eval(10, "1 +").is_err());
    assert!(inspector.eval(10, "1 2").is_err());
}

/// Command parsing of the D3-3 inspection commands.
#[test]
fn test_parse_command_inspection() {
    assert_eq!(parse_command("node 7"), Some(Command::Node(NodeId(7))));
    assert_eq!(parse_command("memo"), Some(Command::Memo));
    assert_eq!(parse_command("memory"), Some(Command::Memory));
    assert_eq!(parse_command("diff"), Some(Command::Diff));
    assert_eq!(parse_command("display"), Some(Command::Display));
    assert_eq!(parse_command("ops"), Some(Command::Ops));
    assert_eq!(
        parse_command("eval live + 2 * version"),
        Some(Command::Eval("live + 2 * version".to_string()))
    );
    assert_eq!(parse_command("node"), None);
    assert_eq!(parse_command("node x"), None);
}

/// The standard scenario with pool and root kept (for the `svg` command).
#[test]
fn test_standard_scenario_full() {
    let (events, pool, root) = standard_scenario_full(4);
    assert_eq!(round_numbers(&events).len(), 4);
    assert!(
        pool.resolve(root).is_ok(),
        "the fixpoint root is in the pool"
    );
}

/// `svg`: the current diagram rendering reuses step_by_step (SVG with
/// timeline and nodes).
#[test]
fn test_render_current_svg() {
    let (events, pool, root) = standard_scenario_full(4);
    let svg = render_current_svg(&pool, root, &events, 10);
    assert!(svg.starts_with("<svg"), "{svg:.80}");
    assert!(svg.contains("</svg>"));
    assert!(svg.contains("step by step"), "the step-by-step header");
    // At the trace end: no ghosts, full diagram.
    let svg_end = render_current_svg(&pool, root, &events, events.len());
    assert!(svg_end.contains("<svg"));
}

/// End-to-end through the command bus: navigation + inspection commands.
#[test]
fn test_debugger_inspection_commands_end_to_end() {
    let events = standard_scenario(4);
    let mut debugger = Debugger::new(events);
    debugger.run(Command::NextRound);
    debugger.run(Command::Step); // step past the Round event so it is in the prefix
    let display = debugger.run(Command::Display);
    assert!(display.contains("display @ pos"), "{display}");
    let ops = debugger.run(Command::Ops);
    assert!(ops.contains("ops @ pos"), "{ops}");
    let eval = debugger.run(Command::Eval("rounds * 10".to_string()));
    assert_eq!(eval, "10", "one round seen -> rounds=1");
    let memo = debugger.run(Command::Memo);
    assert!(memo.contains("memo @ pos"), "{memo}");
}
