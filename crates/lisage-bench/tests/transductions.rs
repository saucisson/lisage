//! PBT du moteur de transductions (M1-4, L3) : identité (mêmes formes en
//! sortie), comptage des chemins acceptants (transduction vs comptage naïf
//! récursif), et sanité de la mémo (op, nœud).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema};
use lisage_core::transduction::{ContextePhiOut, Transducteur, Transduction};
use lisage_lang::value::Atom;
use proptest::prelude::*;

type PoolRef = Arena<ReferenceController>;

fn nouveau_pool() -> PoolRef {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

struct Identite;

impl Transduction for Identite {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        2001
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, (Option<Level>, ())>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memoire::EffetMemoire<Self::M>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memoire::EffetMemoire::new(),
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}

struct CompteChemins;

impl Transduction for CompteChemins {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        2002
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).sum()
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, lisage_core::memoire::EffetMemoire<Self::M>) {
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs: ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
                },
            ),
            lisage_core::memoire::EffetMemoire::new(),
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, u64) {
        (
            pool.intern(anchor, Form::Terminal(payload)),
            u64::from(payload),
        )
    }
}

/// Égalité structurelle de deux formes dans deux pools distincts (les NodeId
/// sont propres à chaque pool).
fn formes_egales(
    pool_a: &PoolRef,
    noeud_a: NodeId,
    pool_b: &PoolRef,
    noeud_b: NodeId,
    visites: &mut std::collections::HashMap<(NodeId, NodeId), bool>,
) -> bool {
    if let Some(&vu) = visites.get(&(noeud_a, noeud_b)) {
        return vu;
    }
    let egales = match (
        pool_a.resolve(noeud_a).unwrap(),
        pool_b.resolve(noeud_b).unwrap(),
    ) {
        (Form::Terminal(pa), Form::Terminal(pb)) => pa == pb,
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
            la == lb
                && aa.len() == ab.len()
                && aa.iter().zip(ab.iter()).all(|((va, na), (vb, nb))| {
                    va == vb && formes_egales(pool_a, *na, pool_b, *nb, visites)
                })
        }
        _ => false,
    };
    visites.insert((noeud_a, noeud_b), egales);
    egales
}

/// Comptage naïf récursif des chemins acceptants (oracle indépendant).
fn compte_naif(pool: &PoolRef, noeud: NodeId) -> u64 {
    match pool.resolve(noeud).unwrap() {
        Form::Terminal(payload) => u64::from(*payload),
        Form::Node { arcs, .. } => arcs.values().map(|&c| compte_naif(pool, c)).sum(),
    }
}

/// Construit un DAG aléatoire dans le pool d'entrée : terminaux puis nœuds
/// de niveaux 0..3, arcs vers des nœuds d'index strictement inférieur.
fn dag_aleatoire(pool: &mut PoolRef, anchor: Anchor, paires: Vec<(u8, u32)>) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let faux = pool.intern(anchor, Form::Terminal(false));
    let mut noeuds = vec![vrai, faux];
    for (niveau, cible) in paires {
        let cible = noeuds[usize::try_from(cible).unwrap() % noeuds.len()];
        let noeud = pool.intern(
            anchor,
            Form::Node {
                level: Level(u32::from(niveau % 4)),
                arcs: BTreeMap::from([(u64::from(niveau % 2), cible)]),
            },
        );
        noeuds.push(noeud);
    }
    *noeuds.last().unwrap()
}

proptest! {
    /// Identité : la transduction reconstruit les mêmes formes en sortie.
    #[test]
    fn test_identite_pbt(paires in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut entree = nouveau_pool();
        let mut sortie = nouveau_pool();
        let anchor = Anchor(0);
        let racine = dag_aleatoire(&mut entree, anchor, paires);
        let mut runner = Transducteur::new(Identite);
        let (resultat, _) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
        prop_assert!(formes_egales(
            &entree,
            racine,
            &sortie,
            resultat,
            &mut std::collections::HashMap::new()
        ));
    }

    /// Comptage : la transduction (φ_aggr = somme) égale le comptage naïf.
    #[test]
    fn test_comptage_pbt(paires in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut entree = nouveau_pool();
        let mut sortie = nouveau_pool();
        let anchor = Anchor(0);
        let racine = dag_aleatoire(&mut entree, anchor, paires);
        let attendu = compte_naif(&entree, racine);
        let mut runner = Transducteur::new(CompteChemins);
        let (resultat, compte) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
        prop_assert!(formes_egales(
            &entree,
            racine,
            &sortie,
            resultat,
            &mut std::collections::HashMap::new()
        ));
        prop_assert_eq!(compte, attendu);
        // Mémo saine : une seconde exécution ne change rien.
        let taille = runner.taille_memo();
        let (r2, c2) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
        prop_assert_eq!(r2, resultat);
        prop_assert_eq!(c2, compte);
        prop_assert_eq!(runner.taille_memo(), taille);
        prop_assert!(taille <= entree.node_count());
    }
}
