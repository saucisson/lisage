//! Tests des événements non locaux (M2-5, spec § 7, énoncé T3) : le
//! transfert `m(a) -= 1; m(b) += 1` compilé en transduction à mémoire de
//! niveau — jeton hérité entre niveaux non adjacents, enregistrement du
//! décrément dans `M`, idempotence, et terminaison sous `saturate`.

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

/// Schéma `Map([a, b, c])` sur le domaine `{0, 1, 2, 3}`.
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

/// Compile une map `clé ↦ valeur` (atomes) dans le pool.
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

/// Décompile un nœud en map `clé ↦ valeur` (atomes).
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
                _ => panic!("atomes attendus"),
            })
            .collect(),
        _ => panic!("map attendue"),
    }
}

/// Cardinal d'un diagramme (nombre de membres de la famille dénotée).
fn cardinal(pool: &PoolRef, schema: &Schema, noeud: lisage_algebra::interface::NodeId) -> u64 {
    let mut sortie = nouveau_pool(schema);
    compter(pool, &mut sortie, Anchor(0), schema, noeud).unwrap()
}

/// M2-5, T3 : le transfert non local `a -= 1; b += 1` entre deux clés non
/// adjacentes (a et b séparées par la clé c) s'applique en une passe.
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
        "m(a) -= 1 ; m(b) += 1, la clé intermédiaire c est inchangée"
    );
}

/// M2-5 : sans unité à transférer (m(a) = 0), la map est inchangée.
#[test]
fn test_transfert_sans_unite() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 0), (1, 1), (2, 2)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

    assert_eq!(resultat, initial, "m(a) = 0 : aucun transfert");
}

/// M2-5, T3 : la mémoire de niveau enregistre le décrément au nœud émetteur
/// (une entrée par nœud touché — surcoût O(w) par nœud), et la largeur de la
/// transition est bornée.
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

    // Le décrément a été produit : une entrée mémoire, version bumpée (L1).
    assert_eq!(runner.memoire().taille(), 1, "une entrée par nœud émetteur");
    assert!(
        runner.version_memoire() >= 1,
        "chaque écriture bump la version"
    );
}

/// M2-5, cas limite épinglé : destination au maximum du domaine — la pose
/// est abandonnée (arc conservé) mais le décrément de la source reste
/// appliqué (sémantique par arc, spec § 7 `φ_disp(val(a), h, v)`).
#[test]
fn test_transfert_destination_hors_domaine() {
    let schema = schema_map();
    let anchor = Anchor(0);
    let mut pool = nouveau_pool(&schema);
    // m(a) = 2, m(b) = 3 = max du domaine {0,1,2,3}.
    let initial = compiler_map(&mut pool, anchor, &schema, &[(0, 2), (1, 3)]);

    let transfert = TransfertNonLocal::new(&schema, 0, 1);
    let mut runner = Transducteur::avec_politique(transfert, PolitiqueMemoire::ParNiveau);
    let (resultat, _) = runner.run_meme_pool(&mut pool, anchor, initial).unwrap();

    let attendu = BTreeMap::from([(0, 1), (1, 3)]);
    assert_eq!(
        decompiler_map(&pool, &schema, resultat),
        attendu,
        "m(b) au max : pose abandonnée, décrément de m(a) appliqué"
    );
}

/// M2-5 : sous `saturate`, le transfert itère jusqu'au point fixe
/// ({a:2,b:0} → {a:1,b:1} → {a:0,b:2}) et termine (T2) — la famille
/// croît de 1 membre par itération.
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
    };
    let resultat = saturer(&mut params).unwrap();
    assert_eq!(resultat.iterations, 3, "a:2,b:0 → a:1,b:1 → a:0,b:2 → fixe");
    assert_eq!(
        cardinal(&pool, &schema, resultat.noeud),
        3,
        "famille des trois états atteints"
    );
}

/// M2-5, régression : collision de clés au remap `v → v-1` sur un nœud
/// `val(a)` multi-arcs (famille produite par union) — les deux sous-arbres
/// doivent être **unionés**, pas écrasés. Famille {{a:0,b:0}, {a:1,b:0}} :
/// le transfert donne {{a:0,b:0}, {a:0,b:1}} (cardinal 2, pas 1).
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
        "collision v→v-1 : les deux membres de la famille sont conservés"
    );
}

/// M2-5, régression : collision au remap `u → u+1` sur un nœud `val(b)`
/// multi-arcs — arc 2 → clé 3 et arc 3 (hors domaine, conservé) → clé 3.
/// Famille {{a:1,b:2}, {a:2,b:3}} → {{a:0,b:3}, {a:1,b:3}} (cardinal 2).
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
        "collision u→u+1 : les deux membres de la famille sont conservés"
    );
}

/// M2-5 : destination **absente** dans un membre d'une famille dont le nœud
/// de présence est multi-arcs — l'arc ABSENT devient PRESENT avec la valeur
/// transférée (unioné à l'arc PRESENT existant). Famille
/// {{a:1,b:0,c:2}, {a:1,c:2}} → {a:0,b:1,c:2} (les deux membres coïncident).
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
        "destination absente dans un membre : rendue présente avec l'unité transférée"
    );
}
