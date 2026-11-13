//! Golden-thread demo — philosophers end to end (M1-10, statement C8):
//! construction of the reachable space R_n for n = 3, 4, 5 in flat encoding
//! (unrolled ring: one key per cell, value = legal cell), counting
//! by fold `(ℕ,+,×)`, `map`/`filter`/`exists` operations, and verification of
//! the **7 forms** (5 internal signatures + 2 terminals) independently of n.
//!
//! Model (docs/golden-thread-example-n-philosophers-end-to-end § 1-2) in **interleaving**: each cell
//! i contributes two keys, `2i` = `pc_i` and `2i+1` = `left_i`, both with
//! domain {0, 1, 2} (`think/hungry/eat` and `free/me/prev`). The fork
//! `left_i` is determined by the word `pc`: `me` if i eats, `prev` if i-1
//! eats, `free` otherwise. A reachable state = circular word over {T, H, E}
//! with no two adjacent `E`.

use std::collections::BTreeMap;

use lisage_core::arena::Arena;
use lisage_core::fold::{count, union};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domain, ReferenceController, Schema, SchemaKeys, compiled};
use lisage_core::{Anchor, Form, NodeId, Pool};
use lisage_lang::value::{Atom, Value};

/// Value `think` / `hungry` / `eat` of a `pc`.
pub const PENSE: u64 = 0;
/// Value `hungry` of a `pc`.
pub const AFFAME: u64 = 1;
/// Value `eat` of a `pc`.
pub const EATS: u64 = 2;
/// Value `free` of a fork.
pub const LIBRE: u64 = 0;
/// Value `me` of a fork.
pub const TENUE: u64 = 1;
/// Value `prev` of a fork.
pub const PRECEDENTE: u64 = 2;

/// Schema of the unrolled ring in **interleaving** (docs/golden-thread-example-n-philosophers-end-to-end
/// § 2): `2n` keys, key `2i` = `pc_i`, key `2i+1` = `left_i`, domain
/// `{0, 1, 2}` for both.
pub fn schema_ring(n: u64) -> Schema {
    Schema::Map(
        (0..2 * n).map(Atom).collect(),
        Domain::new(vec![Atom(0), Atom(1), Atom(2)]),
    )
}

/// Fork `left_i` determined by the word `pc` (no two `E` adjacent
/// circularly): `me` if i eats, `prev` if i-1 eats, `free` otherwise.
pub fn fourchette(pc: &[u64], n: usize, i: usize) -> u64 {
    if pc[i] == EATS {
        TENUE
    } else if pc[(i + n - 1) % n] == EATS {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Enumerates the circular words of length `n` over {0,1,2} with no two `2`
/// adjacent circularly (reachable states, reference enumeration).
pub fn words_reachable(n: usize) -> Vec<Vec<u64>> {
    let mut result = Vec::new();
    let mut word = vec![0u64; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(word[i] == 2 && word[(i + 1) % n] == 2));
        if circulaire_ok {
            result.push(word.clone());
        }
        // Increment in base 3.
        let mut i = n;
        loop {
            if i == 0 {
                return result;
            }
            i -= 1;
            if word[i] < 2 {
                word[i] += 1;
                break;
            }
            word[i] = 0;
        }
    }
}

/// Builds R_n in the pool: union of the compiled maps, one per reachable
/// state. Generic over the pool: the didactic workshop wraps the
/// controller (`WorkshopController`) to commutes the reductions.
pub fn build_r_n<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    n: u64,
) -> NodeId {
    let schema = schema_ring(n);
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(anchor, Form::Terminal(false));
    for word in words_reachable(n as usize) {
        let map: BTreeMap<Value, Value> = (0..n)
            .flat_map(|i| {
                let i = i as usize;
                [
                    (2 * i as u64, word[i]),
                    (2 * i as u64 + 1, fourchette(&word, n as usize, i)),
                ]
            })
            .map(|(key, value)| (Value::Atom(Atom(key)), Value::Atom(Atom(value))))
            .collect();
        let membre = compiled(pool, anchor, &schema, &Value::Map(map))
            .expect("reachable state compilable (internal invariant)");
        root = union(pool, &keys, anchor, root, membre).expect("internal union");
    }
    root
}

/// Count |R_n| by fold `(ℕ,+,×)` (DD lane).
pub fn count_r_n(
    pool_input: &Arena<ReferenceController>,
    pool_output: &mut Arena<ReferenceController>,
    anchor: Anchor,
    n: u64,
    root: NodeId,
) -> u64 {
    let schema = schema_ring(n);
    count(pool_input, pool_output, anchor, &schema, root).expect("internal count")
}

/// The 5 legal cells (forms of the minimal automaton of the language of
/// legal cells, docs/golden-thread-example-n-philosophers-end-to-end § 3-4): with the inherited
/// p = « the previous one eats », p = 0 allows (think,free), (hungry,free),
/// (eat,me); p = 1 allows (think,prev), (hungry,prev). With the 2
/// terminals ⊥/⊤, these are the **7 forms** of the golden thread (automaton
/// level; the « 7 DD nodes » version is the relative encoding, milestone M4).
pub fn cells_legal() -> [(u64, u64); 5] {
    [
        (PENSE, LIBRE),
        (AFFAME, LIBRE),
        (EATS, TENUE),
        (PENSE, PRECEDENTE),
        (AFFAME, PRECEDENTE),
    ]
}

/// « eaters » schema for `filter`/`exists`: `n` keys, key `i` present iff
/// cell `i` eats (single value `0`). The presence predicates then
/// discriminate the states.
pub fn schema_eaters(n: u64) -> Schema {
    Schema::Map((0..n).map(Atom).collect(), Domain::new(vec![Atom(0)]))
}

/// Builds the set of eaters of R_n (key `i` present iff `pc_i =
/// eat`): projection of R_n usable by `filter`/`exists`.
pub fn build_eaters(pool: &mut Arena<ReferenceController>, anchor: Anchor, n: u64) -> NodeId {
    let schema = schema_eaters(n);
    let keys = SchemaKeys::new(schema.clone());
    let mut root = pool.intern(anchor, Form::Terminal(false));
    for word in words_reachable(n as usize) {
        let map: BTreeMap<Value, Value> = (0..n)
            .filter(|&i| word[i as usize] == EATS)
            .map(|i| (Value::Atom(Atom(i)), Value::Atom(Atom(0))))
            .collect();
        let membre = compiled(pool, anchor, &schema, &Value::Map(map))
            .expect("reachable state compilable (internal invariant)");
        root = union(pool, &keys, anchor, root, membre).expect("internal union");
    }
    root
}

/// New pool for the demo.
pub fn pool_nine(schema: &Schema) -> Arena<ReferenceController> {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}
