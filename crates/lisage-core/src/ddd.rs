//! Instance-test **DDD-différence** (backlog § 9 de la feuille de route,
//! question ouverte 9 de la spec, veine #83 après les fragments intervalles
//! et DBM) : les *Difference Decision Diagrams* de Møller & Andersen (~2002)
//! — contraintes de différence symboliques (DBM) pour les automates
//! temporisés — comme instance vivante de l'algèbre de clés-régions.
//!
//! Le contrat de clés (forme normale, couverture, intersection) est déclaré
//! côté contrôleur (`Dbm`, `DbmKeys`) ; le pool ne change pas. L'ordre O
//! s'entend sur le treillis des régions : un nœud partitionne la boîte en
//! régions convexes, et l'arc absent vaut l'« absent » continu δ(ℓ) = région
//! vide → payload par défaut. Le balayage d'univers (énumération des points
//! du domaine) ne compile que si l'univers est borné à régions énumérables
//! (spec Q9) : la construction du diagramme passe par les régions
//! explicites, jamais par l'énumération des points — royaume libre DDD
//! assumé sur le continu.

use std::collections::{BTreeMap, BTreeSet};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool, PoolError};

use crate::dbm::Dbm;

/// Évalue un DDD-différence sur un point du domaine : à chaque nœud, suit
/// l'arc dont la région contient le point. `Ok(None)` si aucun arc ne
/// couvre le point : l'arc absent vaut δ(ℓ), l'« absent » continu.
pub fn evaluer<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    point: &[u64],
) -> Result<Option<bool>, PoolError> {
    let mut noeud = racine;
    loop {
        match pool.resolve(noeud)? {
            Form::Terminal(payload) => return Ok(Some(*payload)),
            Form::Node { arcs, .. } => {
                match arcs
                    .iter()
                    .find(|(region, _)| region.contient(point))
                    .map(|(_, cible)| *cible)
                {
                    Some(cible) => noeud = cible,
                    None => return Ok(None),
                }
            }
        }
    }
}

/// Compte les nœuds distincts atteignables depuis `racine`, terminaux compris.
pub fn compter_noeuds<P: Pool<Key = Dbm, Payload = bool>>(pool: &P, racine: NodeId) -> u64 {
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

/// Instance-test : la bande `x − y ≤ 1 ∧ y − x ≤ 2` sur deux horloges,
/// compilée en deux niveaux par régions explicites (aucun balayage
/// d'univers à la construction) :
///
/// - niveau 0 : `x − y ≤ 1` mène au niveau 1, le complémentaire
///   `y − x ≤ −2` mène à `false` (les deux arcs couvrent la boîte) ;
/// - niveau 1 : `y − x ≤ 2` mène à `true` ; hors de cette région, l'arc
///   absent vaut δ(ℓ) → `false`.
pub fn compiler_bande<P: Pool<Key = Dbm, Payload = bool>>(pool: &mut P, anchor: Anchor) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let faux = pool.intern(anchor, Form::Terminal(false));
    let niveau_1 = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(Dbm::nouveau(2, &[(1, 0, 2)]), vrai)]),
        },
    );
    pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([
                (Dbm::nouveau(2, &[(0, 1, 1)]), niveau_1),
                (Dbm::nouveau(2, &[(1, 0, -2)]), faux),
            ]),
        },
    )
}
