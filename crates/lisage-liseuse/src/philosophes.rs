//! Golden-thread demo — end-to-end philosophers (M1-10, statement C8):
//! construction of the reachable space R_n for n = 3, 4, 5 in flat encoding
//! (unrolled ring: one key per cell, value = legal cell), counting
//! by fold `(ℕ,+,×)`, `map`/`filter`/`exists` operations, and verification of the
//! **7 forms** (5 internal signatures + 2 terminals) independently of n.
//!
//! Model (docs/golden-thread-example-n-philosophers-end-to-end § 1-2) in **interleaving**: each cell
//! i contributes two keys, `2i` = `pc_i` and `2i+1` = `left_i`, both with
//! domain {0, 1, 2} (`think/hungry/eat` and `free/me/prev`). The fork
//! `left_i` is determined by the word `pc`: `me` if i eats, `prev` if i-1
//! eats, `free` otherwise. A reachable state = circular word over {T, H, E}
//! with no two adjacent `E`.

use std::collections::BTreeMap;

use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::{Anchor, Form, NodeId, Pool};
use lisage_lang::value::{Atom, Value};

/// `think` / `hungry` / `eat` value of a `pc`.
pub const PENSE: u64 = 0;
/// `hungry` value of a `pc`.
pub const AFFAME: u64 = 1;
/// `eat` value of a `pc`.
pub const MANGE: u64 = 2;
/// `free` value of a fork.
pub const LIBRE: u64 = 0;
/// `me` value of a fork.
pub const TENUE: u64 = 1;
/// `prev` value of a fork.
pub const PRECEDENTE: u64 = 2;

/// Schema of the unrolled ring in **interleaving** (docs/golden-thread-example-n-philosophers-end-to-end
/// § 2): `2n` keys, key `2i` = `pc_i`, key `2i+1` = `left_i`, domain
/// `{0, 1, 2}` for both.
pub fn schema_anneau(n: u64) -> Schema {
    Schema::Map(
        (0..2 * n).map(Atom).collect(),
        Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]),
    )
}

/// Fork `left_i` determined by the word `pc` (no two `E` adjacent
/// circularly): `me` if i eats, `prev` if i-1 eats, `free` otherwise.
pub fn fourchette(pc: &[u64], n: usize, i: usize) -> u64 {
    if pc[i] == MANGE {
        TENUE
    } else if pc[(i + n - 1) % n] == MANGE {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Enumerates the circular words of length `n` over {0,1,2} with no two `2`
/// adjacent circularly (reachable states, reference enumeration).
pub fn mots_atteignables(n: usize) -> Vec<Vec<u64>> {
    let mut resultat = Vec::new();
    let mut mot = vec![0u64; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(mot[i] == 2 && mot[(i + 1) % n] == 2));
        if circulaire_ok {
            resultat.push(mot.clone());
        }
        // Increment in base 3.
        let mut i = n;
        loop {
            if i == 0 {
                return resultat;
            }
            i -= 1;
            if mot[i] < 2 {
                mot[i] += 1;
                break;
            }
            mot[i] = 0;
        }
    }
}

/// Builds R_n in the pool: union of the compiled maps, one per
/// reachable state. Generic over the pool: the didactic workshop wraps the
/// controller (`AtelierController`) to switch the reductions.
pub fn construire_r_n<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    n: u64,
) -> NodeId {
    let schema = schema_anneau(n);
    let keys = SchemaKeys::new(schema.clone());
    let mut racine = pool.intern(anchor, Form::Terminal(false));
    for mot in mots_atteignables(n as usize) {
        let map: BTreeMap<Value, Value> = (0..n)
            .flat_map(|i| {
                let i = i as usize;
                [
                    (2 * i as u64, mot[i]),
                    (2 * i as u64 + 1, fourchette(&mot, n as usize, i)),
                ]
            })
            .map(|(cle, valeur)| (Value::Atom(Atom(cle)), Value::Atom(Atom(valeur))))
            .collect();
        let membre = compiler(pool, anchor, &schema, &Value::Map(map))
            .expect("reachable state compilable (internal invariant)");
        racine = union(pool, &keys, anchor, racine, membre).expect("internal union");
    }
    racine
}

/// Counts |R_n| by fold `(ℕ,+,×)` (DD path).
pub fn compter_r_n(
    pool_entree: &Arena<ReferenceController>,
    pool_sortie: &mut Arena<ReferenceController>,
    anchor: Anchor,
    n: u64,
    racine: NodeId,
) -> u64 {
    let schema = schema_anneau(n);
    compter(pool_entree, pool_sortie, anchor, &schema, racine).expect("internal count")
}

/// The 5 legal cells (forms of the minimal automaton of the language of
/// legal cells, docs/golden-thread-example-n-philosophers-end-to-end § 3-4): with the inherited
/// p = « the previous eats », p = 0 allows (think,free), (hungry,free),
/// (eat,me); p = 1 allows (think,prev), (hungry,prev). With the 2
/// terminals ⊥/⊤, these are the **7 forms** of the golden thread (automaton level;
/// the « 7 DD nodes » version is the relative encoding, milestone M4).
pub fn cellules_legales() -> [(u64, u64); 5] {
    [
        (PENSE, LIBRE),
        (AFFAME, LIBRE),
        (MANGE, TENUE),
        (PENSE, PRECEDENTE),
        (AFFAME, PRECEDENTE),
    ]
}

/// « eaters » schema for `filter`/`exists`: `n` keys, key `i` present
/// iff cell `i` eats (single value `0`). The presence predicates
/// then discriminate the states.
pub fn schema_mangeurs(n: u64) -> Schema {
    Schema::Map((0..n).map(Atom).collect(), Domaine::nouveau(vec![Atom(0)]))
}

/// Builds the set of eaters of R_n (key `i` present iff `pc_i =
/// eat`): projection of R_n usable by `filter`/`exists`.
pub fn construire_mangeurs(
    pool: &mut Arena<ReferenceController>,
    anchor: Anchor,
    n: u64,
) -> NodeId {
    let schema = schema_mangeurs(n);
    let keys = SchemaKeys::new(schema.clone());
    let mut racine = pool.intern(anchor, Form::Terminal(false));
    for mot in mots_atteignables(n as usize) {
        let map: BTreeMap<Value, Value> = (0..n)
            .filter(|&i| mot[i as usize] == MANGE)
            .map(|i| (Value::Atom(Atom(i)), Value::Atom(Atom(0))))
            .collect();
        let membre = compiler(pool, anchor, &schema, &Value::Map(map))
            .expect("reachable state compilable (internal invariant)");
        racine = union(pool, &keys, anchor, racine, membre).expect("internal union");
    }
    racine
}

/// New pool for the demo.
pub fn pool_neuf(schema: &Schema) -> Arena<ReferenceController> {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}
