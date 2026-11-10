//! Configuration du contrôleur en **BDD classique** (M1-8, énoncé C6) :
//! variables booléennes ordonnées, arcs `0`/`1` toujours explicites, aucune
//! clé par défaut désignée — R-default est inactif, seules les réductions
//! R-share (hash-consing) et R-collapse s'appliquent. C'est la configuration
//! « témoin » de référence pour la reproduction à l'identique (C6) : le nombre
//! de nœuds d'une fonction booléenne doit être identique chez tout témoin
//! BDD indépendant (mini-BDD interne, CUDD/Sylvan via FFI — voir
//! `docs/spike-ffi-cudd-sylvan.md`).

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

/// Cardinalité d'un niveau BDD : deux arcs, `0` (bas) et `1` (haut).
pub const CARDINALITE_BDD: u64 = 2;

/// Algèbre de clés d'un BDD classique sur `nombre_variables` variables :
/// le niveau ℓ est la variable ℓ, `Val(ℓ) = {0, 1}`, pas de δ(ℓ).
#[derive(Clone)]
pub struct BddKeys {
    nombre_variables: u32,
}

impl BddKeys {
    /// Algèbre de clés pour `nombre_variables` variables ordonnées 0..n-1.
    pub fn new(nombre_variables: u32) -> Self {
        BddKeys { nombre_variables }
    }

    /// Nombre de variables du BDD.
    pub fn nombre_variables(&self) -> u32 {
        self.nombre_variables
    }
}

impl KeyAlgebra for BddKeys {
    type Key = u64;

    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }

    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITE_BDD
    }

    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

/// Contrôleur BDD classique : payload booléen, aucun diagramme de défaut
/// désigné par niveau (R-default inactif), R-collapse seul.
pub struct BddController {
    keys: BddKeys,
}

impl BddController {
    /// Contrôleur pour `nombre_variables` variables ordonnées 0..n-1.
    pub fn new(nombre_variables: u32) -> Self {
        BddController {
            keys: BddKeys::new(nombre_variables),
        }
    }
}

impl Controller for BddController {
    type Key = u64;
    type Payload = bool;
    type Keys = BddKeys;

    const FINGERPRINT: ControllerId = ControllerId(3);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

/// Compile une table de vérité en BDD classique par expansion de Shannon
/// descendante : la racine est au niveau 0, le niveau ℓ décide la variable
/// ℓ. `table[a]` = valeur de la fonction sur l'affectation `a` (bit ℓ de
/// `a` = variable ℓ) ; la longueur doit être `2^n`.
pub fn compiler_bdd<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    keys: &BddKeys,
    table: &[bool],
) -> NodeId {
    let nombre_variables = keys.nombre_variables();
    assert_eq!(
        table.len(),
        1usize << nombre_variables,
        "table de vérité de taille 2^n (invariant interne)"
    );
    shannon(pool, anchor, Level(0), table)
}

/// Expansion de Shannon : `table` est indexée par les bits des variables
/// `level.0..n` (bit ℓ de l'indice = variable ℓ) ; les positions paires
/// (bit de la variable courante = 0) donnent l'arc 0, les impaires l'arc 1.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
) -> NodeId {
    if table.len() == 1 {
        return pool.intern(anchor, Form::Terminal(table[0]));
    }
    let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
    let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
    let bas = shannon(pool, anchor, Level(level.0 + 1), &bas_table);
    let haut = shannon(pool, anchor, Level(level.0 + 1), &haut_table);
    pool.intern(
        anchor,
        Form::Node {
            level,
            arcs: BTreeMap::from([(0, bas), (1, haut)]),
        },
    )
}

/// Compte les nœuds distincts atteignables depuis `racine`, terminaux compris.
pub fn compter_noeuds<P: Pool<Key = u64, Payload = bool>>(pool: &P, racine: NodeId) -> u64 {
    let mut visite = BTreeSet::new();
    let mut pile = vec![racine];
    while let Some(noeud) = pile.pop() {
        if !visite.insert(noeud) {
            continue;
        }
        if let Ok(Form::Node { arcs, .. }) = pool.resolve(noeud) {
            pile.extend(arcs.values().copied());
        }
    }
    visite.len() as u64
}

/// Évalue le BDD sur une affectation (bit ℓ = variable ℓ).
pub fn evaluer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    affectation: u64,
) -> Result<bool, PoolError> {
    let mut noeud = racine;
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(payload) => return Ok(*payload),
            Form::Node { level, arcs } => {
                let bit = (affectation >> level.0) & 1;
                noeud = *arcs.get(&bit).ok_or(PoolError::UnknownNode(noeud))?;
            }
        }
    }
}
