//! Tests for the transduction engine (M1-4, spec § 3.1, L3): the two channels
//! (inherited/synthesized), the φ_aggr aggregation and the memo (op, node, h).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema, compiler};
use lisage_core::transduction::{ContextePhiOut, Transducteur, Transduction};
use lisage_lang::value::{Atom, Value};

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// Identity transduction with inherited channel = depth: H = u64.
struct IdentiteProfondeur;

impl Transduction for IdentiteProfondeur {
    type H = u64;
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        1001
    }
    fn h0(&self) -> u64 {
        0
    }
    fn phi_disp(&self, _level: Level, h: &u64, _arc: u64) -> u64 {
        h + 1
    }
    fn phi_aggr(&self, _level: Level, _h: &u64, arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).max().unwrap_or(0)
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
        h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), *h)
    }
}

/// Transduction counting accepting paths (synthesized channel only).
struct CompteChemins {
    appels: std::sync::Arc<AtomicU64>,
}

impl Transduction for CompteChemins {
    type H = ();
    type S = u64;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        1002
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
        self.appels.fetch_add(1, Ordering::SeqCst);
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

/// Identity without inherited channel, to compare memo fingerprints.
struct Identite;

impl Transduction for Identite {
    type H = ();
    type S = ();
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        1003
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

/// Builds a small DAG in the input pool: b = ν(1, {0 ↦ a, 1 ↦ ⊤}),
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

/// Inherited channel: the identity transduction rebuilds the same forms; the
/// returned depth equals the DAG's height.
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
        "identity ⇒ same forms"
    );
    assert_eq!(profondeur, 2, "max depth reached = DAG height");
}

/// Synthesized channel: φ_aggr sums the children's weights; the count is the
/// number of accepting paths (2 in the DAG: 0·0 and 1).
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
    assert_eq!(compte_chemins, 2, "two paths lead to ⊤");
    assert_eq!(
        appels.load(Ordering::SeqCst),
        2,
        "φ_out called once per non-terminal node"
    );
}

/// Memo (op, node, h): a second run recomputes nothing (memo hit), and two
/// operations with distinct fingerprints have separate entries.
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
        "second run: only memo hits"
    );
    // 4 distinct (node, h) pairs: b, a, ⊤, ⊥.
    assert_eq!(taille_apres_1, 4);

    // Another operation (different fingerprint) on the same node: separate
    // memo. With H = depth, ⊤ is visited at two distinct h (1 via b's arc 1,
    // 2 via a's arc 0): 5 entries (b, a, ⊤×2, ⊥).
    let mut autre = Transducteur::new(IdentiteProfondeur);
    let (r3, _) = autre.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(sortie.resolve(r3).unwrap(), entree.resolve(b).unwrap());
    assert_eq!(
        autre.taille_memo(),
        5,
        "memo specific to the operation and to h: key (op, node, h)"
    );
}

/// The transduction also applies to a diagram compiled from a value (M1-3
/// schema): identity over a compiled set.
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
