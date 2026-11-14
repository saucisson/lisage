//! Tests for non-local events (M2-5, spec § 7, statement T3): the transfer
//! `m(a) -= 1; m(b) += 1` compiled as a level-memory transduction — inherited
//! token between non-adjacent levels, recording of the decrement in `M`,
//! idempotence, and termination under `saturate`.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, PolitiqueMemoire};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::saturate::{ParametresSaturation, PolitiqueSaturation, saturer};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler, decompiler};
use lisage_core::transduction::Transducteur;
use lisage_core::transfert::{LARGEUR_TRANSFERT, TransfertNonLocal};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

/// Schema `Map([a, b, c])` over the domain `{0, 1, 2, 3}`.
fn schema_map() -> Schema {
    Schema::Map(
        vec![Atom(0), Atom(1), Atom(2)],
        Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2), Atom(3)]),
    )
}

fn nouveau_pool(schema: &Schema) -> PoolRef {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}

/// Compiles a map `key ↦ value` (atoms) in the pool.
fn compiler_map(
    pool: &mut PoolRef,
    anchor: Anchor,
    schema: &Schema,
    entrees: &[(u64, u64)],
) -> lisage_algebra::interface::NodeId {
    compiler(
        pool,
        anchor,
        schema,
        &Value::Map(
            entrees
                .iter()
                .map(|(k, v)| (Value::Atom(Atom(*k)), Value::Atom(Atom(*v))))
                .collect(),
        ),
    )
    .unwrap()
}

/// Decompiles a node into a map `key ↦ value` (atoms).
fn decompiler_map(
    pool: &PoolRef,
    schema: &Schema,
    noeud: lisage_algebra::interface::NodeId,
) -> BTreeMap<u64, u64> {
    match decompiler(pool, schema, noeud).unwrap() {
        Value::Map(map) => map
            .into_iter()
            .map(|(k, v)| match (k, v) {
                (Value::Atom(Atom(k)), Value::Atom(Atom(v))) => (k, v),
                _ => panic!("atoms expected"),
            })
            .collect(),
        _ => panic!("map expected"),
    }
}

/// Cardinality of a diagram (number of members of the denoted family).
fn cardinal(pool: &PoolRef, schema: &Schema, noeud: lisage_algebra::interface::NodeId) -> u64 {
    let mut sortie = nouveau_pool(schema);
    compter(pool, &mut sortie, Anchor(0), schema, noeud).unwrap()
}

/// M2-5, T3: the non-local transfer `a -= 1; b += 1` between two non-adjacent
/// keys (a and b separated by key c) applies in a single pass.
#[test]
fn test_transfert_a_vers_b_non_adjacent() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 2), (1, 0), (2, 1)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

    let attendu = BTreeMap::from([(0, 1), (1, 1), (2, 1)]);
    assert_eq!(
        decompiler_map(&pool, &schema, resultat),
        attendu,
        "m(a) -= 1 ; m(b) += 1, the intermediate key c is unchanged"
    );
}

/// M2-5: without a unit to transfer (m(a) = 0), the map is unchanged.
#[test]
fn test_transfert_sans_unite() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 0), (1, 1), (2, 2)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

    assert_eq!(resultat, initial, "m(a) = 0: no transfer");
}

/// M2-5, T3: the level memory records the decrement at the emitting node (one
/// entry per touched node — O(w) overhead per node), and the transition width
/// is bounded.
#[test]
fn test_transfert_memoire_et_largeur() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 2), (1, 0)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    assert_eq!(transfert.largeur(), LARGEUR_TRANSFERT);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let _ = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

    // The decrement was produced: one memory entry, version bumped (L1).
    assert_eq!(runner.memoire().taille(), 1, "one entry per emitting node");
    assert!(
        runner.version_memoire() >= 1,
        "each write bumps the version"
    );
}

/// M2-5, pinned edge case: destination at the domain maximum — the placement
/// is abandoned (arc kept) but the source's decrement is still applied
/// (per-arc semantics, spec § 7 `φ_disp(val(a), h, v)`).
#[test]
fn test_transfert_destination_hors_domaine() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    // m(a) = 2, m(b) = 3 = max of the domain {0,1,2,3}.
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 2), (1, 3)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

    let attendu = BTreeMap::from([(0, 1), (1, 3)]);
    assert_eq!(
        decompiler_map(&pool, &schema, resultat),
        attendu,
        "m(b) at max: placement abandoned, m(a) decrement applied"
    );
}

/// M2-5: under `saturate`, the transfer iterates to the fixed point
/// ({a:2,b:0} → {a:1,b:1} → {a:0,b:2}) and terminates (T2) — the family grows
/// by 1 member per iteration.
#[test]
fn test_transfert_sous_saturate_termine() {
    let schema = Schema::Map(
        vec![Atom(0), Atom(1)],
        Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 2), (1, 0)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let niveaux: Vec<lisage_algebra::interface::Level> = (0..schema.nombre_niveaux())
        .map(lisage_algebra::interface::Level)
        .collect();
    let politique = PolitiqueSaturation::Statique(niveaux);
    let mut params = ParametresSaturation {
        transduction: &transfert,
        politique: &politique,
        pool: &mut pool,
        keys: &keys,
        anchor,
        initial,
        nombre_niveaux: schema.nombre_niveaux(),
        observateur: None,
    };
    let resultat = saturer(&mut params).unwrap();
    assert_eq!(
        resultat.iterations, 3,
        "a:2,b:0 → a:1,b:1 → a:0,b:2 → fixed"
    );
    assert_eq!(
        cardinal(&pool, &schema, resultat.noeud),
        3,
        "family of the three reached states"
    );
}

/// M2-5, regression: key collision on the remap `v → v-1` over a multi-arc
/// `val(a)` node (family produced by union) — the two subtrees must be
/// **unioned**, not overwritten. Family {{a:0,b:0}, {a:1,b:0}}: the transfer
/// gives {{a:0,b:0}, {a:0,b:1}} (cardinality 2, not 1).
#[test]
fn test_transfert_collision_source_union() {
    let schema = Schema::Map(
        vec![Atom(0), Atom(1)],
        Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]),
    );
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let m1 = compiler_map(&mut pool, anchor, &schema, &[(0, 0), (1, 0)]);
    let m2 = compiler_map(&mut pool, anchor, &schema, &[(0, 1), (1, 0)]);
    let famille = union(&mut pool, &keys, anchor, m1, m2).unwrap();
    assert_eq!(cardinal(&pool, &schema, famille), 2);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, famille).unwrap();
    assert_eq!(
        cardinal(&pool, &schema, resultat),
        2,
        "collision v→v-1: both members of the family are kept"
    );
}

/// M2-5, regression: collision on the remap `u → u+1` over a multi-arc
/// `val(b)` node — arc 2 → key 3 and arc 3 (out of domain, kept) → key 3.
/// Family {{a:1,b:2}, {a:2,b:3}} → {{a:0,b:3}, {a:1,b:3}} (cardinality 2).
#[test]
fn test_transfert_collision_destination_union() {
    let schema = schema_map();
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let m1 = compiler_map(&mut pool, anchor, &schema, &[(0, 1), (1, 2)]);
    let m2 = compiler_map(&mut pool, anchor, &schema, &[(0, 2), (1, 3)]);
    let famille = union(&mut pool, &keys, anchor, m1, m2).unwrap();
    assert_eq!(cardinal(&pool, &schema, famille), 2);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, famille).unwrap();
    assert_eq!(
        cardinal(&pool, &schema, resultat),
        2,
        "collision u→u+1: both members of the family are kept"
    );
}

/// M2-5: destination **absent** in a member of a family whose presence node
/// is multi-arc — the ABSENT arc becomes PRESENT with the transferred value
/// (unioned with the existing PRESENT arc). Family {{a:1,b:0,c:2}, {a:1,c:2}}
/// → {a:0,b:1,c:2} (the two members coincide).
#[test]
fn test_transfert_destination_absente_dans_famille() {
    let schema = schema_map();
    let keys = SchemaKeys::new(schema.clone());
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let m1 = compiler_map(&mut pool, anchor, &schema, &[(0, 1), (1, 0), (2, 2)]);
    let m2 = compiler_map(&mut pool, anchor, &schema, &[(0, 1), (2, 2)]);
    let famille = union(&mut pool, &keys, anchor, m1, m2).unwrap();
    assert_eq!(cardinal(&pool, &schema, famille), 2);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, famille).unwrap();

    let attendu = compiler_map(&mut pool, anchor, &schema, &[(0, 0), (1, 1), (2, 2)]);
    assert_eq!(
        resultat, attendu,
        "destination absent in a member: made present with the transferred unit"
    );
}
