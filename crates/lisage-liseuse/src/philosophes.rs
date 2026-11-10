//! Démo fil rouge — philosophes de bout en bout (M1-10, énoncé C8) :
//! construction de l'espace atteignable R_n pour n = 3, 4, 5 en encodage plat
//! (anneau déroulé : une clé par cellule, valeur = cellule légale), comptage
//! par pli `(ℕ,+,×)`, opérations `map`/`filter`/`exists`, et vérification des
//! **7 formes** (5 signatures internes + 2 terminaux) indépendamment de n.
//!
//! Modèle (docs/exemple-fil-rouge § 1) : chaque cellule i a un état
//! `pc_i ∈ {think, hungry, eat}` ; la fourchette `left_i` est déterminée :
//! `me` si i mange, `prev` si i-1 mange, `free` sinon. Un état atteignable =
//! mot circulaire sur {T, H, E} sans deux `E` adjacents. La cellule légale
//! (pc, left) est encodée par un atome de valeur :
//! 0 = (think, free), 1 = (hungry, free), 2 = (eat, me),
//! 3 = (think, prev), 4 = (hungry, prev).

use std::collections::BTreeMap;

use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::pli::{compter, union};
use lisage_core::schema::{Domaine, ReferenceController, Schema, SchemaKeys, compiler};
use lisage_core::{Anchor, Form, NodeId, Pool};
use lisage_lang::value::{Atom, Value};

/// Valeur `think` / `hungry` / `eat` d'un `pc`.
pub const PENSE: u64 = 0;
/// Valeur `hungry` d'un `pc`.
pub const AFFAME: u64 = 1;
/// Valeur `eat` d'un `pc`.
pub const MANGE: u64 = 2;
/// Valeur `free` d'une fourchette.
pub const LIBRE: u64 = 0;
/// Valeur `me` d'une fourchette.
pub const TENUE: u64 = 1;
/// Valeur `prev` d'une fourchette.
pub const PRECEDENTE: u64 = 2;

/// Schéma de l'anneau déroulé en **intercalage** (docs/exemple-fil-rouge
/// § 2) : `2n` clés, clé `2i` = `pc_i`, clé `2i+1` = `left_i`, domaine
/// `{0, 1, 2}` pour les deux.
pub fn schema_anneau(n: u64) -> Schema {
    Schema::Map(
        (0..2 * n).map(Atom).collect(),
        Domaine::nouveau(vec![Atom(0), Atom(1), Atom(2)]),
    )
}

/// Fourchette `left_i` déterminée par le mot `pc` (sans deux `E` adjacents
/// circulairement) : `me` si i mange, `prev` si i-1 mange, `free` sinon.
pub fn fourchette(pc: &[u64], n: usize, i: usize) -> u64 {
    if pc[i] == MANGE {
        TENUE
    } else if pc[(i + n - 1) % n] == MANGE {
        PRECEDENTE
    } else {
        LIBRE
    }
}

/// Énumère les mots circulaires de longueur `n` sur {0,1,2} sans deux `2`
/// adjacents circulairement (états atteignables, énumération de référence).
pub fn mots_atteignables(n: usize) -> Vec<Vec<u64>> {
    let mut resultat = Vec::new();
    let mut mot = vec![0u64; n];
    loop {
        let circulaire_ok = (0..n).all(|i| !(mot[i] == 2 && mot[(i + 1) % n] == 2));
        if circulaire_ok {
            resultat.push(mot.clone());
        }
        // Incrémentation en base 3.
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

/// Construit R_n dans le pool : union des maps compilées, une par état
/// atteignable.
pub fn construire_r_n(pool: &mut Arena<ReferenceController>, anchor: Anchor, n: u64) -> NodeId {
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
            .expect("état atteignable compilable (invariant interne)");
        racine = union(pool, &keys, anchor, racine, membre).expect("union interne");
    }
    racine
}

/// Compte |R_n| par pli `(ℕ,+,×)` (voie DD).
pub fn compter_r_n(
    pool_entree: &Arena<ReferenceController>,
    pool_sortie: &mut Arena<ReferenceController>,
    anchor: Anchor,
    n: u64,
    racine: NodeId,
) -> u64 {
    let schema = schema_anneau(n);
    compter(pool_entree, pool_sortie, anchor, &schema, racine).expect("comptage interne")
}

/// Les 5 cellules légales (formes de l'automate minimal du langage des
/// cellules légales, docs/exemple-fil-rouge § 3-4) : avec l'hérité
/// p = « le précédent mange », p = 0 autorise (think,free), (hungry,free),
/// (eat,me) ; p = 1 autorise (think,prev), (hungry,prev). Avec les 2
/// terminaux ⊥/⊤, ce sont les **7 formes** du fil rouge (niveau automate ;
/// la version « 7 nœuds du DD » est l'encodage relatif, jalon M4).
pub fn cellules_legales() -> [(u64, u64); 5] {
    [
        (PENSE, LIBRE),
        (AFFAME, LIBRE),
        (MANGE, TENUE),
        (PENSE, PRECEDENTE),
        (AFFAME, PRECEDENTE),
    ]
}

/// Schéma « mangeurs » pour `filter`/`exists` : `n` clés, clé `i` présente
/// ssi la cellule `i` mange (valeur unique `0`). Les prédicats de présence
/// discriminent alors les états.
pub fn schema_mangeurs(n: u64) -> Schema {
    Schema::Map((0..n).map(Atom).collect(), Domaine::nouveau(vec![Atom(0)]))
}

/// Construit l'ensemble des mangeurs de R_n (clé `i` présente ssi `pc_i =
/// eat`) : projection de R_n utilisable par `filter`/`exists`.
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
            .expect("état atteignable compilable (invariant interne)");
        racine = union(pool, &keys, anchor, racine, membre).expect("union interne");
    }
    racine
}

/// Pool neuf pour la démo.
pub fn pool_neuf(schema: &Schema) -> Arena<ReferenceController> {
    Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    )
}
