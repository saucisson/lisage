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
const PENSE: u64 = 0;
/// `hungry`.
const AFFAME: u64 = 1;
/// `eat`.
const MANGE: u64 = 2;
/// Step of the interleaving keys: key `2i` = `pc_i`, key `2i+1` = `left_i`.
const PAS_CLE_INTERCALAGE: u64 = 2;
/// Offset of the `left_i` key after the `pc_i` key in the interleaving.
const DECALAGE_CLE_GAUCHE: u64 = 1;
/// Fingerprint of the contention step transduction.
const FINGERPRINT_ETAPE: u64 = 6001;

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
struct EmetSingleton {
    singleton: NodeId,
}

impl Transduction for EmetSingleton {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_ETAPE
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
fn schema_anneau(n: u64) -> Schema {
    Schema::Map(
        (0..PAS_CLE_INTERCALAGE * n).map(Atom).collect(),
        Domain::new(vec![Atom(PENSE), Atom(AFFAME), Atom(MANGE)]),
    )
}

/// `left_i` fork derived from the `pc` word (golden thread § 3).
fn fourchette(pc: &[u64], n: usize, i: usize) -> u64 {
    const LIBRE: u64 = 0;
    const TENUE: u64 = 1;
    const PRECEDENTE: u64 = 2;
    if pc[i] == MANGE {
        TENUE
    } else if pc[(i + n - 1) % n] == MANGE {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Enumerates the reachable circular words (no two adjacent eaters).
fn mots_atteignables(n: usize) -> Vec<Vec<u64>> {
    let mut result = Vec::new();
    let mut mot = vec![PENSE; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(mot[i] == MANGE && mot[(i + 1) % n] == MANGE));
        if circulaire_ok {
            result.push(mot.clone());
        }
        let mut i = n;
        loop {
            if i == 0 {
                return result;
            }
            i -= 1;
            if mot[i] < MANGE {
                mot[i] += 1;
                break;
            }
            mot[i] = PENSE;
        }
    }
}

/// Builds R_n (union of the maps compiled over the interleaved schema).
fn build_r_n(pool: &mut PoolRef, schema: &Schema, keys: &SchemaKeys, n: u64) -> NodeId {
    let mut root = pool.intern(ANCHOR, Form::Terminal(false));
    for mot in mots_atteignables(n as usize) {
        let map: BTreeMap<Value, Value> = (0..n)
            .flat_map(|i| {
                let i = i as usize;
                [
                    (PAS_CLE_INTERCALAGE * i as u64, mot[i]),
                    (
                        PAS_CLE_INTERCALAGE * i as u64 + DECALAGE_CLE_GAUCHE,
                        fourchette(&mot, n as usize, i),
                    ),
                ]
            })
            .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
            .collect();
        let membre = compiler(pool, ANCHOR, schema, &Value::Map(map)).unwrap();
        root = union(pool, keys, ANCHOR, root, membre).unwrap();
    }
    root
}

/// Compiles one extended state into a member map.
fn compiler_membre(pool: &mut PoolRef, schema: &Schema, mot: &[u64], n: u64) -> NodeId {
    let map: BTreeMap<Value, Value> = (0..n)
        .flat_map(|i| {
            let i = i as usize;
            [
                (PAS_CLE_INTERCALAGE * i as u64, mot[i]),
                (
                    PAS_CLE_INTERCALAGE * i as u64 + DECALAGE_CLE_GAUCHE,
                    fourchette(mot, n as usize, i),
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

    let schema = schema_anneau(n);
    let keys = SchemaKeys::new(schema.clone());
    let observer = Arc::new(CountingObserver::default());
    struct Pont(Arc<CountingObserver>);
    impl Observer for Pont {
        fn on_event(&self, event: &TraceEvent) {
            self.0.on_event(event);
        }
    }
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(Pont(observer.clone())),
    );

    // Phase 1: build R_n (the anchored philosophers workload).
    let debut = Instant::now();
    let root = build_r_n(&mut pool, &schema, &keys, n);
    let temps_build_ms = debut.elapsed().as_secs_f64() * 1000.0;

    // Phase 2: saturation with the contention step (illegal state: two
    // adjacent eaters, absent from R_n — added in round 1, fixpoint in 2).
    let mut mot_illegal = vec![PENSE; n as usize];
    mot_illegal[0] = MANGE;
    mot_illegal[1] = MANGE;
    let singleton = compiler_membre(&mut pool, &schema, &mot_illegal, n);
    let etape = EmetSingleton { singleton };
    let levels: Vec<Level> = (0..schema.number_levels()).map(Level).collect();
    let policy = PolicySaturation::Statique(levels);
    let debut = Instant::now();
    let mut params = ParametresSaturation {
        transduction: &etape,
        policy: &policy,
        pool: &mut pool,
        keys: &keys,
        anchor: ANCHOR,
        initial: root,
        number_levels: schema.number_levels(),
        observer: Some(observer.clone()),
    };
    let result = saturer(&mut params).expect("saturation terminates");
    let temps_saturation_ms = debut.elapsed().as_secs_f64() * 1000.0;

    // Phase 3: cardinality of the fixpoint (verification).
    let mut output = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(lisage_core::observer::NoOpObserver),
    );
    let debut = Instant::now();
    let verifie = count(&pool, &mut output, ANCHOR, &schema, result.node).unwrap();
    let temps_count_ms = debut.elapsed().as_secs_f64() * 1000.0;

    let metriques = pool.metriques();
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
        "{{\"outil\":\"lisage-contention\",\"n\":{n},\"noeuds\":{nodes},\"verifie\":{verifie},\"rounds\":{},\"creations\":{},\"partages\":{},\"taux_hit_rshare\":{},\"memo_hits\":{memo_hits},\"memo_misses\":{memo_misses},\"taux_hit_memo\":{memo_hit_rate:.4},\"interns_par_niveau\":{:?},\"temps_build_ms\":{temps_build_ms:.3},\"temps_saturation_ms\":{temps_saturation_ms:.3},\"temps_count_ms\":{temps_count_ms:.3}}}",
        *observer.rounds.lock().unwrap(),
        metriques.creations,
        metriques.partages,
        metriques.rate_hit(),
        interns_per_level,
    );
}
