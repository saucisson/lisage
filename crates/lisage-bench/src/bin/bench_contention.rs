//! P0-1 runner (issue #136): contention bench anchored on the philosophers
//! workload (golden thread) at n = 5, with saturation. Instruments the
//! existing metrics: intern calls per level (P0-1 counter in the pool),
//! R-share hit rate (pool metrics), memo hit rate (MemoHit/MemoMiss events),
//! saturation rounds, and per-phase wall times. Prints one JSON line.
//!
//! The saturation step is a synthetic contention step (emits one fixed
//! member, an illegal state absent from R_5): it exercises the saturation
//! machinery (rounds, memo, union) on the philosophers diagram — the real
//! take/release/hungry transition saturation belongs to the golden-thread
//! milestone, not to this bench.
//!
//! Usage: `bench_contention [n]` (default n = 5, the anchored criterion).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::{Observer, TraceEvent};
use lisage_core::pli::{count, union};
use lisage_core::saturate::{ParametresSaturation, PolicySaturation, saturer};
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::{Atom, Value};

/// Test anchor.
const ANCHOR: Anchor = Anchor(0);
/// `think`.
const THINK: u64 = 0;
/// `hungry`.
const HUNGRY: u64 = 1;
/// `eat`.
const EAT: u64 = 2;
/// Step of the interleaving keys: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const KEY_STRIDE: u64 = 2;
/// Offset of the `left_i` key after the `pc_i` key in the interleaving.
const LEFT_KEY_OFFSET: u64 = 1;
/// Fingerprint of the contention step transduction.
const STEP_FINGERPRINT: u64 = 6001;

type PoolRef = Arena<ReferenceController>;

/// Counting observer: memo hits/misses, rounds, node creations/shares.
#[derive(Default)]
struct CountingObserver {
    memo_hits: Mutex<u64>,
    memo_misses: Mutex<u64>,
    rounds: Mutex<u64>,
    creations: Mutex<u64>,
    shares: Mutex<u64>,
}

impl Observer for CountingObserver {
    fn on_event(&self, event: &TraceEvent) {
        match event {
            TraceEvent::MemoHit { .. } => *self.memo_hits.lock().unwrap() += 1,
            TraceEvent::MemoMiss { .. } => *self.memo_misses.lock().unwrap() += 1,
            TraceEvent::Round { .. } => *self.rounds.lock().unwrap() += 1,
            TraceEvent::NodeCreated { .. } => *self.creations.lock().unwrap() += 1,
            TraceEvent::NodeShared { .. } => *self.shares.lock().unwrap() += 1,
            _ => {}
        }
    }
}

/// Contention step transduction: always emits the fixed singleton member
/// (terminals → ⊥), like the PBT step of `saturate.rs`.
#[derive(Clone)]
struct EmitSingleton {
    singleton: NodeId,
}

impl Transduction for EmitSingleton {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        STEP_FINGERPRINT
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, _h: &(), _arc: u64) {}
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        _pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memory::EffetMemory<()>) {
        let _ = ctx;
        (self.singleton, lisage_core::memory::EffetMemory::new())
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        _payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(false)), ())
    }
}

/// Schema of the ring unrolled in interleaving (golden thread § 2).
fn ring_schema(n: u64) -> Schema {
    Schema::Map(
        (0..KEY_STRIDE * n).map(Atom).collect(),
        Domain::new(vec![Atom(THINK), Atom(HUNGRY), Atom(EAT)]),
    )
}

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn left_fork(pc: &[u64], n: usize, i: usize) -> u64 {
    const FREE: u64 = 0;
    const HELD: u64 = 1;
    const PREVIOUS: u64 = 2;
    if pc[i] == EAT {
        HELD
    } else if pc[(i + n - 1) % n] == EAT {
        PREVIOUS
    } else {
        FREE
    }
}

/// Enumerates the reachable circular words (no two adjacent eaters).
fn reachable_words(n: usize) -> Vec<Vec<u64>> {
    let mut result = Vec::new();
    let mut word = vec![THINK; n];
    loop {
        let circular_ok = (0..n).all(|i| !(word[i] == EAT && word[(i + 1) % n] == EAT));
        if circular_ok {
            result.push(word.clone());
        }
        let mut i = n;
        loop {
            if i == 0 {
                return result;
            }
            i -= 1;
            if word[i] < EAT {
                word[i] += 1;
                break;
            }
            word[i] = THINK;
        }
    }
}

/// Builds R_n (union of the maps compiled over the interleaved schema).
fn build_r_n(pool: &mut PoolRef, schema: &Schema, keys: &SchemaKeys, n: u64) -> NodeId {
    let mut root = pool.intern(ANCHOR, Form::Terminal(false));
    for word in reachable_words(n as usize) {
        let map: BTreeMap<Value, Value> = (0..n)
            .flat_map(|i| {
                let i = i as usize;
                [
                    (KEY_STRIDE * i as u64, word[i]),
                    (
                        KEY_STRIDE * i as u64 + LEFT_KEY_OFFSET,
                        left_fork(&word, n as usize, i),
                    ),
                ]
            })
            .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
            .collect();
        let member = compiler(pool, ANCHOR, schema, &Value::Map(map)).unwrap();
        root = union(pool, keys, ANCHOR, root, member).unwrap();
    }
    root
}

/// Compiles one extended state into a member map.
fn compile_member(pool: &mut PoolRef, schema: &Schema, word: &[u64], n: u64) -> NodeId {
    let map: BTreeMap<Value, Value> = (0..n)
        .flat_map(|i| {
            let i = i as usize;
            [
                (KEY_STRIDE * i as u64, word[i]),
                (
                    KEY_STRIDE * i as u64 + LEFT_KEY_OFFSET,
                    left_fork(word, n as usize, i),
                ),
            ]
        })
        .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
        .collect();
    compiler(pool, ANCHOR, schema, &Value::Map(map)).unwrap()
}

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "5".to_string())
        .parse()
        .expect("n is an integer");
    assert!((1..=8).contains(&n), "n in 1..=8");

    let schema = ring_schema(n);
    let keys = SchemaKeys::new(schema.clone());
    let observer = Arc::new(CountingObserver::default());
    struct Bridge(Arc<CountingObserver>);
    impl Observer for Bridge {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(Bridge(observer.clone())),
    );

    // Phase 1: build R_n (the anchored philosophers workload).
    let start = Instant::now();
    let root = build_r_n(&mut pool, &schema, &keys, n);
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Phase 2: saturation with the contention step (illegal state: two
    // adjacent eaters, absent from R_n — added in round 1, fixpoint in 2).
    let mut illegal_word = vec![THINK; n as usize];
    illegal_word[0] = EAT;
    illegal_word[1] = EAT;
    let singleton = compile_member(&mut pool, &schema, &illegal_word, n);
    let step = EmitSingleton { singleton };
    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy = PolicySaturation::Statique(levels);
    let start = Instant::now();
    let mut params = ParametresSaturation {
        transduction: &step,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor: ANCHOR,
        initial: root,
        number_levels: schema.number_levels(),
        observer: Some(observer.clone()),
    };
    let result = saturer(&mut params).expect("saturation terminates");
    let saturation_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Phase 3: cardinality of the fixpoint (verification).
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let start = Instant::now();
    let verified = count(&pool, &mut output, ANCHOR, &schema, result.node).unwrap();
    let count_ms = start.elapsed().as_secs_f64() * 1000.0;

    let metrics = pool.metriques();
    let memo_hits = *observer.memo_hits.lock().unwrap();
    let memo_misses = *observer.memo_misses.lock().unwrap();
    let memo_total = memo_hits + memo_misses;
    let memo_hit_rate = if memo_total > 0 {
        memo_hits as f64 / memo_total as f64
    } else {
        0.0
    };
    let interns_per_level: Vec<u64> = pool.interns_per_level().to_vec();
    let nodes = pool.live();
    println!(
        "{{\"tool\":\"lisage-contention\",\"n\":{n},\"nodes\":{nodes},\"verified\":{verified},\"rounds\":{},\"creations\":{},\"shares\":{},\"rshare_hit_rate\":{},\"memo_hits\":{memo_hits},\"memo_misses\":{memo_misses},\"memo_hit_rate\":{memo_hit_rate:.4},\"interns_per_level\":{:?},\"build_ms\":{build_ms:.3},\"saturation_ms\":{saturation_ms:.3},\"count_ms\":{count_ms:.3}}}",
        *observer.rounds.lock().unwrap(),
        metrics.creations,
        metrics.partages,
        metrics.rate_hit(),
        interns_per_level,
    );
}
