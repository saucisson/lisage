//! PBT du moteur de transductions (M1-4, L3) : identité (résultat = entrée),
//! comptage des chemins acceptants (transduction vs comptage naïf récursif),
//! et sanité de la mémo (op, nœud).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema};
use lisage_core::transduction::{Transducteur, Transduction};
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

    fn fingerprint(&self) -> u64 {
        2001
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, ()>) {}
    fn phi_out(&self, level: Level, _h: &(), arcs: BTreeMap<u64, (NodeId, ())>) -> Form<u64, bool> {
        Form::Node {
            level,
            arcs: arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
        }
    }
    fn psi(&self, payload: bool, _h: &()) -> (bool, ()) {
        (payload, ())
    }
}

struct CompteChemins;

impl Transduction for CompteChemins {
    type H = ();
    type S = u64;

    fn fingerprint(&self) -> u64 {
        2002
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), arcs: &BTreeMap<u64, u64>) -> u64 {
        arcs.values().sum()
    }
    fn phi_out(
        &self,
        level: Level,
        _h: &(),
        arcs: BTreeMap<u64, (NodeId, u64)>,
    ) -> Form<u64, bool> {
        Form::Node {
            level,
            arcs: arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
        }
    }
    fn psi(&self, payload: bool, _h: &()) -> (bool, u64) {
        (payload, u64::from(payload))
    }
}

/// Comptage naïf récursif des chemins acceptants (oracle indépendant).
fn compte_naif(pool: &PoolRef, noeud: NodeId) -> u64 {
    match pool.resolve(noeud).unwrap() {
        Form::Terminal(payload) => u64::from(*payload),
        Form::Node { arcs, .. } => arcs.values().map(|&c| compte_naif(pool, c)).sum(),
    }
}

/// Construit un DAG aléatoire : terminaux puis nœuds de niveaux 0..3, arcs
/// vers des nœuds d'index strictement inférieur (DAG acyclique).
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
    /// Identité : la transduction reconstruit les mêmes formes ⇒ même nœud.
    #[test]
    fn test_identite_pbt(paires in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut pool = nouveau_pool();
        let anchor = Anchor(0);
        let racine = dag_aleatoire(&mut pool, anchor, paires);
        let mut runner = Transducteur::new(Identite);
        let (resultat, _) = runner.run(&mut pool, anchor, racine).unwrap();
        prop_assert_eq!(resultat, racine);
    }

    /// Comptage : la transduction (φ_aggr = somme) égale le comptage naïf.
    #[test]
    fn test_comptage_pbt(paires in prop::collection::vec((0u8..4, 0u32..8), 0..8)) {
        let mut pool = nouveau_pool();
        let anchor = Anchor(0);
        let racine = dag_aleatoire(&mut pool, anchor, paires);
        let attendu = compte_naif(&pool, racine);
        let mut runner = Transducteur::new(CompteChemins);
        let (resultat, compte) = runner.run(&mut pool, anchor, racine).unwrap();
        prop_assert_eq!(resultat, racine);
        prop_assert_eq!(compte, attendu);
        // Mémo saine : une seconde exécution ne change rien.
        let taille = runner.taille_memo();
        let (r2, c2) = runner.run(&mut pool, anchor, racine).unwrap();
        prop_assert_eq!(r2, resultat);
        prop_assert_eq!(c2, compte);
        prop_assert_eq!(runner.taille_memo(), taille);
        prop_assert!(taille <= pool.node_count());
    }
}
