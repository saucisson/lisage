//! Tests du moteur de transductions (M1-4, spec § 3.1, L3) : les deux
//! canaux (hérité/synthétisé), l'agrégation φ_aggr et la mémo (op, nœud, h).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_core::transduction::{Transducteur, Transduction};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Transduction identité avec canal hérité = profondeur : H = u64.
struct IdentiteProfondeur;

impl Transduction for IdentiteProfondeur {
    type H = u64;
    type S = u64;
    type Out = bool;

    fn fingerprint(&self) -> u64 {
        1001
    }
    fn h0(&self) -> u64 {
        0
    }
    fn phi_disp(&self, _level: Level, h: &u64, _arc: u64) -> u64 {
        h + 1
    }
    fn phi_aggr(&self, _level: Level, _h: &u64, arcs: &BTreeMap<u64, u64>) -> u64 {
        arcs.values().copied().max().unwrap_or(0)
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        _h: &u64,
        arcs: BTreeMap<u64, (NodeId, u64)>,
    ) -> NodeId {
        pool.intern(
            anchor,
            Form::Node {
                level,
                arcs: arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
            },
        )
    }
    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), *h)
    }
}

/// Transduction comptage des chemins acceptants (canal synthétisé seul).
struct CompteChemins {
    appels: std::sync::Arc<AtomicU64>,
}

impl Transduction for CompteChemins {
    type H = ();
    type S = u64;
    type Out = bool;

    fn fingerprint(&self) -> u64 {
        1002
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), arcs: &BTreeMap<u64, u64>) -> u64 {
        arcs.values().sum()
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        _h: &(),
        arcs: BTreeMap<u64, (NodeId, u64)>,
    ) -> NodeId {
        self.appels.fetch_add(1, Ordering::SeqCst);
        pool.intern(
            anchor,
            Form::Node {
                level,
                arcs: arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
            },
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

/// Identité sans canal hérité, pour comparer les fingerprints de mémo.
struct Identite;

impl Transduction for Identite {
    type H = ();
    type S = ();
    type Out = bool;

    fn fingerprint(&self) -> u64 {
        1003
    }
    fn h0(&self) {}
    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }
    fn phi_aggr(&self, _level: Level, _h: &(), _arcs: &BTreeMap<u64, ()>) {}
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        _h: &(),
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> NodeId {
        pool.intern(
            anchor,
            Form::Node {
                level,
                arcs: arcs.into_iter().map(|(v, (n, _))| (v, n)).collect(),
            },
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

/// Construit un petit DAG dans le pool d'entrée : b = ν(1, {0 ↦ a, 1 ↦ ⊤}),
/// a = ν(0, {0 ↦ ⊤, 1 ↦ ⊥}).
fn dag(pool: &mut PoolRef, anchor: Anchor) -> (NodeId, NodeId) {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let faux = pool.intern(anchor, Form::Terminal(false));
    let a = pool.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let b = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, a), (1, vrai)]),
        },
    );
    (a, b)
}

/// Canal hérité : la transduction identité reconstruit les mêmes formes ;
/// la profondeur remontée vaut la hauteur du DAG.
#[test]
fn test_canal_herite_identite_et_profondeur() {
    let mut entree = pool();
    let mut sortie = pool();
    let anchor = Anchor(0);
    let (_, b) = dag(&mut entree, anchor);
    let mut runner = Transducteur::new(IdentiteProfondeur);
    let (resultat, profondeur) = runner.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(
        sortie.resolve(resultat).unwrap(),
        entree.resolve(b).unwrap(),
        "identité ⇒ mêmes formes"
    );
    assert_eq!(profondeur, 2, "profondeur max atteinte = hauteur du DAG");
}

/// Canal synthétisé : φ_aggr somme les poids des fils ; le comptage vaut le
/// nombre de chemins acceptants (2 dans le DAG : 0·0 et 1).
#[test]
fn test_canal_synthetise_comptage() {
    let mut entree = pool();
    let mut sortie = pool();
    let anchor = Anchor(0);
    let (_, b) = dag(&mut entree, anchor);
    let appels = std::sync::Arc::new(AtomicU64::new(0));
    let compte = CompteChemins {
        appels: appels.clone(),
    };
    let mut runner = Transducteur::new(compte);
    let (resultat, compte_chemins) = runner.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(
        sortie.resolve(resultat).unwrap(),
        entree.resolve(b).unwrap()
    );
    assert_eq!(compte_chemins, 2, "deux chemins mènent à ⊤");
    assert_eq!(
        appels.load(Ordering::SeqCst),
        2,
        "φ_out appelé une fois par nœud non terminal"
    );
}

/// Mémo (op, nœud, h) : une seconde exécution ne recalcule rien (hit mémo),
/// et deux opérations de fingerprints distincts ont des entrées séparées.
#[test]
fn test_memo_fingerprint_op_noeud() {
    let mut entree = pool();
    let mut sortie = pool();
    let anchor = Anchor(0);
    let (_, b) = dag(&mut entree, anchor);
    let mut runner = Transducteur::new(Identite);
    let (r1, _) = runner.run(&entree, &mut sortie, anchor, b).unwrap();
    let taille_apres_1 = runner.taille_memo();
    let (r2, _) = runner.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(r1, r2);
    assert_eq!(
        runner.taille_memo(),
        taille_apres_1,
        "seconde exécution : que des hits mémo"
    );
    // 4 paires (nœud, h) distinctes : b, a, ⊤, ⊥.
    assert_eq!(taille_apres_1, 4);

    // Une autre opération (fingerprint différent) sur le même nœud : mémo
    // séparée. Avec H = profondeur, ⊤ est visité à deux h distincts (1 via
    // l'arc 1 de b, 2 via l'arc 0 de a) : 5 entrées (b, a, ⊤×2, ⊥).
    let mut autre = Transducteur::new(IdentiteProfondeur);
    let (r3, _) = autre.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(sortie.resolve(r3).unwrap(), entree.resolve(b).unwrap());
    assert_eq!(
        autre.taille_memo(),
        5,
        "mémo propre à l'opération et à h : clé (op, nœud, h)"
    );
}

/// La transduction s'applique aussi à un diagramme compilé depuis une valeur
/// (schéma M1-3) : identité sur un ensemble compilé.
#[test]
fn test_transduction_sur_diagramme_compile() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let mut entree = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let mut sortie = Arena::new(
        ReferenceController::new(schema.clone()),
        Box::new(NoOpObserver),
    );
    let anchor = Anchor(0);
    let ensemble = Value::Set(std::collections::BTreeSet::from([
        Value::Atom(Atom(0)),
        Value::Atom(Atom(1)),
    ]));
    let racine = compiler(&mut entree, anchor, &schema, &ensemble).unwrap();
    let mut runner = Transducteur::new(Identite);
    let (resultat, _) = runner.run(&entree, &mut sortie, anchor, racine).unwrap();
    assert_eq!(
        sortie.resolve(resultat).unwrap(),
        entree.resolve(racine).unwrap()
    );
}
