//! Tests of the HSDD spike (parent issue #382). The falsifiable claims:
//! (1) the tower stores the ring in O(1) forms + one edge per n while the
//! flat prototype grows with n; (2) the structural fold reproduces |R_n|
//! exactly (fold == enumeration == golden thread); (3) C1 holds on the
//! module (hash-consing hit, same forms across pools). Any contrary
//! measurement falsifies the spike's mirroring of the HSDD gains.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_algebra::relative::{InputArc, ShiftGroup};
use lisage_core::arena::Arena;
use lisage_core::fold::union;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_lang::value::{Atom, Value};

use lisage_bench::hsdd::{
    RING_COUNTS, build_module, build_ring, count_ring, count_words_reference, coupling, fork_state,
    pool_tower, read_cells, read_ring,
};

/// Test anchor of the tower.
const ANCHOR_TOWER: Anchor = Anchor(0);
/// Flat prototype interleaving step: key `2i` = `pc_i`, `2i+1` = `left_i`.
const STEP_KEY_INTERLEAVING: u64 = 2;
/// Live nodes of the flat prototype (union of maps) per n — pinned edge
/// (`philosophers_bench`), growing with n.
const FLAT_LIVE: &[(u64, usize)] = &[(3, 206), (4, 646), (5, 1850), (6, 5198), (7, 14362)];
/// Live nodes of the tower after the module: 1 true terminal + 2 left
/// nodes + 1 pc node.
const TOWER_MODULE_LIVE: usize = 4;

/// `left` fork derived from the `pc` word (golden thread, oracle side).
fn fork_of_word(word: &[u64], i: usize) -> u64 {
    let n = word.len();
    fork_state(word[(i + n - 1) % n], word[i])
}

/// Builds the flat prototype of R_n: union of the compiled interleaved
/// maps (the encoding the tower replaces).
fn build_flat(n: u64) -> (Arena<ReferenceController>, NodeId, Schema) {
    let schema = Schema::Map(
        (0..STEP_KEY_INTERLEAVING * n).map(Atom).collect(),
        Domain::new((0..3).map(Atom).collect()),
    );
    let keys = SchemaKeys::new(schema.clone());
    let mut pool = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut root = pool.intern(ANCHOR_TOWER, Form::Terminal(false));
    for word in lisage_bench::hsdd::words_reference(n) {
        let map: BTreeMap<Value, Value> = (0..n as usize)
            .flat_map(|i| {
                [
                    (STEP_KEY_INTERLEAVING * i as u64, word[i]),
                    (STEP_KEY_INTERLEAVING * i as u64 + 1, fork_of_word(&word, i)),
                ]
            })
            .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
            .collect();
        let member = compiler(&mut pool, ANCHOR_TOWER, &schema, &Value::Map(map)).unwrap();
        root = union(&mut pool, &keys, ANCHOR_TOWER, root, member).unwrap();
    }
    (pool, root, schema)
}

/// (1) Storage: one pool, the module built once, one ring per n. The live
/// count is the module plus exactly one node per distinct n — the flat
/// prototype of the same sets grows with n. Explicit falsification on
/// every asserted number.
#[test]
fn test_tower_storage_constant_forms_and_edge() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    assert_eq!(pool.live(), TOWER_MODULE_LIVE, "the module is stored once");
    let mut tower_live = Vec::new();
    for &(n, _) in RING_COUNTS {
        let ring = build_ring(&mut pool, ANCHOR_TOWER, module, n).unwrap();
        let (positions, target) = read_ring(&pool, ring).unwrap();
        assert_eq!(target, module, "every position set points at the module");
        assert_eq!(
            positions.count_ones(),
            n as u32,
            "the mask holds n positions"
        );
        assert_eq!(positions, (1u64 << n) - 1);
        tower_live.push(pool.live());
    }
    // The module + exactly one edge node per distinct n.
    for (index, &live) in tower_live.iter().enumerate() {
        assert_eq!(
            live,
            TOWER_MODULE_LIVE + index + 1,
            "one instantiation node per distinct n, module shared"
        );
    }
    // Contrast with the flat prototype on the same n series.
    for &(n, expected) in FLAT_LIVE {
        let (pool_flat, _root, _schema) = build_flat(n);
        assert_eq!(pool_flat.live(), expected, "n={n}: flat prototype edge");
        assert!(
            pool_flat.live() > tower_live[(n - 3) as usize],
            "n={n}: the tower stores the same set with fewer live nodes"
        );
    }
}

/// (2) Semantics: |Ring_n| by structural fold over the tower (cells read
/// from the module, adjacency coupling, circular trace) == enumeration
/// oracle == golden thread. Explicit falsification.
#[test]
fn test_tower_semantics_fold_enumeration_golden() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let cells = read_cells(&pool, module).unwrap();
    assert_eq!(cells.len(), 5, "the module carries the 5 legal cells");
    let matrix = coupling(&cells);
    for &(n, expected) in RING_COUNTS {
        let folded = count_ring(&matrix, n);
        let reference = count_words_reference(n);
        assert_eq!(
            folded, reference,
            "n={n}: tower fold == enumeration for |R_{n}|"
        );
        assert_eq!(
            folded, expected,
            "n={n}: |R_{n}| = {expected} (golden thread)"
        );
    }
}

/// (3) C1: re-interning the module is a hash-consing hit (no new node),
/// and a second pool rebuilds the same forms.
#[test]
fn test_module_canonicity_hit_and_cross_pool() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let before = pool.live();
    let again = build_module(&mut pool, ANCHOR_TOWER);
    assert_eq!(again, module, "same anchor, same form: hash-consing hit");
    assert_eq!(pool.live(), before, "no node created by the rebuild");
    // Cross-pool: a fresh pool rebuilds the same forms (C1 between pools).
    let mut other = pool_tower();
    let module_other = build_module(&mut other, ANCHOR_TOWER);
    assert_eq!(
        other.live(),
        before,
        "the second pool stores the same forms"
    );
    assert!(forms_equal(&pool, module, &other, module_other));
}

/// Structural form equality across pools (C1 between pools, the transductions
/// convention of `par_equivalence`: forms, never bare ids).
fn forms_equal(
    left: &Arena<lisage_bench::hsdd::TowerController>,
    left_root: NodeId,
    right: &Arena<lisage_bench::hsdd::TowerController>,
    right_root: NodeId,
) -> bool {
    let form_left = left.resolve(left_root).unwrap();
    let form_right = right.resolve(right_root).unwrap();
    match (form_left, form_right) {
        (Form::Terminal(a), Form::Terminal(b)) => a == b,
        (
            Form::Node {
                level: la,
                arcs: aa,
            },
            Form::Node {
                level: lb,
                arcs: ab,
            },
        ) => {
            if la != lb || aa.len() != ab.len() {
                return false;
            }
            for ((ka, na), (kb, nb)) in aa.iter().zip(ab.iter()) {
                if ka != kb || !forms_equal(left, *na, right, *nb) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

/// (4) The instantiation reading of M4-2: n positions read the **same**
/// module handle through entry arcs carrying the position argument — one
/// handle, n readings, no new node (work sharing, O(1) + edge).
#[test]
fn test_instance_reading_one_handle_all_positions() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let handle = pool.handle(module, ANCHOR_TOWER);
    let locked = pool.live();
    for position in 0..7 {
        let arc = InputArc::new(position, handle);
        for key in 0..3 {
            // Absolute reading: the local key k at position i reads k + shift.
            assert_eq!(
                arc.read_key(&ShiftGroup, &key),
                (key as i64 + position) as u64
            );
        }
    }
    assert_eq!(pool.live(), locked, "reading never stores");
}

/// Confirm the store contract used by the tests: an untouched tower level
/// number is opaque to the pool (the reading convention is ours).
#[test]
fn test_tower_levels_are_pool_opaque() {
    let mut pool = pool_tower();
    let module = build_module(&mut pool, ANCHOR_TOWER);
    let ring = build_ring(&mut pool, ANCHOR_TOWER, module, 5).unwrap();
    let Form::Node { level, .. } = pool.resolve(ring).unwrap() else {
        panic!("the ring is a node");
    };
    assert_eq!(level, Level(lisage_bench::hsdd::LEVEL_INSTANCE));
    let (positions, target) = read_ring(&pool, ring).unwrap();
    assert_eq!(target, module);
    assert_eq!(positions, 0b1_1111);
}
