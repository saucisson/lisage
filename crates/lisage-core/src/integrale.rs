//! **Intégrale au pli** (backlog § 9 de la feuille de route, question
//! ouverte 9 de la spec) : sur les clés-régions 1D (intervalles), le pli
//! discret — somme des cardinalités — devient **intégrale de Lebesgue** :
//! la mesure d'un intervalle `[a, b]` dans l'interprétation continue est
//! sa longueur `b − a` (le point isolé a mesure nulle), et l'intégrale
//! d'une densité constante 1 sur le diagramme est la mesure de l'ensemble
//! dénoté.
//!
//! L'intégrale se calcule avec la région courante comme contexte :
//! `intégrale(nœud, région) = Σ_{arcs I → fils} intégrale(fils, région ∩ I)`,
//! le terminal `⊤` rendant la mesure de la région courante, `⊥` rendant 0.
//! Sur un diagramme à un seul niveau, c'est la somme des longueurs des
//! intervalles menant à `⊤` — le différentiel contre le pli cardinal
//! (somme des tailles) montre le passage count → mesure.
//!
//! Variante **DBM** (spec Q9 (ii), « régions polyédriques ») : sur les
//! clés-régions DBM à 2 variables, la mesure de Lebesgue est l'**aire** de
//! la zone ∩ boîte (`DbmKeys::mesure_zone`, rendue doublée en `i128`), et
//! l'intégrale de densité constante 1 sur le diagramme est l'aire de
//! l'ensemble dénoté — `intégrale(nœud, région) = Σ_{arcs Z → fils}
//! intégrale(fils, région ∩ Z)`.
//!
//! Narration (ADR-0100 décision 6) : chaque appel récursif émet un
//! événement `Attr` (canal `Synthétise`) vers l'observateur optionnel,
//! avec la mesure calculée ; le test épingle ces événements.

use std::collections::{BTreeMap, HashMap};

use lisage_algebra::interface::{Form, NodeId, Pool, PoolError};

use crate::dbm::{Dbm, DbmKeys};
use crate::observer::{CanalAttr, Observer, TraceEvent};
use crate::region::Intervalle;

/// Mesure de Lebesgue d'un intervalle dans l'interprétation continue :
/// `longueur([a, b]) = b − a` ; l'intervalle vide et le point isolé ont
/// mesure nulle.
pub fn mesure_intervalle(intervalle: &Intervalle) -> u64 {
    if intervalle.est_vide() {
        0
    } else {
        intervalle
            .borne_sup()
            .saturating_sub(intervalle.borne_inf())
    }
}

/// Intersection de deux intervalles (meet ensembliste) : l'intervalle
/// vide si disjoints.
pub fn intersection(a: &Intervalle, b: &Intervalle) -> Intervalle {
    if a.est_vide() || b.est_vide() {
        return crate::region::VIDE;
    }
    Intervalle::nouveau(
        a.borne_inf().max(b.borne_inf()),
        a.borne_sup().min(b.borne_sup()),
    )
}

/// Intégrale de la densité constante 1 sur l'ensemble dénoté par le
/// diagramme à clés-régions 1D, restreint à la `region` courante
/// (interprétation continue : mesure de Lebesgue, en `u64` saturé).
///
/// `observateur` : canal de narration (ADR-0100) — chaque nœud visité émet
/// un événement `Attr`/`Synthétise` avec la mesure calculée.
pub fn integrale_constante<P: Pool<Key = Intervalle, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    region: &Intervalle,
    observateur: Option<&dyn Observer>,
) -> Result<u64, PoolError> {
    let mut memo: HashMap<(NodeId, Intervalle), u64> = HashMap::new();
    integrale(pool, racine, region, observateur, &mut memo)
}

fn integrale<P: Pool<Key = Intervalle, Payload = bool>>(
    pool: &P,
    noeud: NodeId,
    region: &Intervalle,
    observateur: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Intervalle), u64>,
) -> Result<u64, PoolError> {
    if let Some(total) = memo.get(&(noeud, *region)) {
        if let Some(obs) = observateur {
            obs.on_event(&TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                noeud,
                valeur: format!("mesure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(noeud)?.clone() {
        Form::Terminal(true) => mesure_intervalle(region),
        Form::Terminal(false) => 0,
        Form::Node { arcs, .. } => {
            let mut somme = 0u64;
            for (intervalle, cible) in &arcs {
                let sous_region = intersection(region, intervalle);
                if sous_region.est_vide() {
                    continue;
                }
                let contribution = integrale(pool, *cible, &sous_region, observateur, memo)?;
                somme = somme.saturating_add(contribution);
            }
            somme
        }
    };
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud,
            valeur: format!("mesure={total}"),
        });
    }
    memo.insert((noeud, *region), total);
    Ok(total)
}

/// Intégrale de la densité constante 1 sur l'ensemble dénoté par le
/// diagramme à clés-régions DBM (2 variables), restreint à la `region`
/// courante, intersectée à la boîte bornée de `keys`. La mesure est
/// l'aire **doublée** (`i128`, exacte) — voir [`DbmKeys::mesure_zone`].
///
/// `observateur` : canal de narration (ADR-0100) — chaque nœud visité émet
/// un événement `Attr`/`Synthétise` avec la mesure calculée.
pub fn integrale_dbm_constante<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    racine: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observateur: Option<&dyn Observer>,
) -> Result<i128, PoolError> {
    let mut memo: HashMap<(NodeId, Dbm), i128> = HashMap::new();
    integrale_dbm(pool, racine, region, keys, observateur, &mut memo)
}

fn integrale_dbm<P: Pool<Key = Dbm, Payload = bool>>(
    pool: &P,
    noeud: NodeId,
    region: &Dbm,
    keys: &DbmKeys,
    observateur: Option<&dyn Observer>,
    memo: &mut HashMap<(NodeId, Dbm), i128>,
) -> Result<i128, PoolError> {
    if let Some(total) = memo.get(&(noeud, region.clone())) {
        if let Some(obs) = observateur {
            obs.on_event(&TraceEvent::Attr {
                canal: CanalAttr::Synthétise,
                noeud,
                valeur: format!("mesure_memo={total}"),
            });
        }
        return Ok(*total);
    }
    let total = match pool.resolve(noeud)?.clone() {
        Form::Terminal(true) => keys.mesure_zone(region),
        Form::Terminal(false) => 0,
        Form::Node { arcs, .. } => {
            let mut somme = 0i128;
            for (zone, cible) in &arcs {
                let sous_region = region.intersection(zone);
                if sous_region.est_vide() {
                    continue;
                }
                let contribution =
                    integrale_dbm(pool, *cible, &sous_region, keys, observateur, memo)?;
                somme += contribution;
            }
            somme
        }
    };
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud,
            valeur: format!("mesure={total}"),
        });
    }
    memo.insert((noeud, region.clone()), total);
    Ok(total)
}

/// Construit un diagramme à un seul niveau dénotant l'union des
/// `intervalles` donnés (arcs vers `⊤`), le complémentaire allant vers
/// `⊥` — instance-test de l'intégrale : la mesure de l'union est la somme
/// des longueurs. **Précondition** : les intervalles sont deux à deux
/// disjoints (assertion de debug) — sinon l'intégrale sommerait les
/// longueurs individuelles et sur-compterait la mesure de l'union.
pub fn compiler_union<P: Pool<Key = Intervalle, Payload = bool>>(
    pool: &mut P,
    anchor: lisage_algebra::interface::Anchor,
    intervalles: &[Intervalle],
) -> NodeId {
    debug_assert!(
        intervalles.windows(2).all(|pair| {
            let a = &pair[0];
            let b = &pair[1];
            a.est_vide() || b.est_vide() || a.borne_sup() < b.borne_inf()
        }),
        "compiler_union exige des intervalles disjoints (précondition)"
    );
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let arcs: BTreeMap<Intervalle, NodeId> = intervalles
        .iter()
        .map(|intervalle| (*intervalle, vrai))
        .collect();
    pool.intern(
        anchor,
        Form::Node {
            level: lisage_algebra::interface::Level(0),
            arcs,
        },
    )
}
