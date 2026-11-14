//! Attributed transduction engine (M1-4, spec § 3.1, statement L3):
//! the two channels — **inherited** attribute `H` (downward, `φ_disp`) and
//! **synthesized** `S` (upward, `φ_aggr`) — and the memo keyed by fingerprint
//! `(op, node, h)` (lemma L1). Rules T-TERM/T-NODE.
//!
//! Since M1-5 (#12), the transduction reads its input from a pool with
//! payload `bool` and writes into a pool with any payload `Out`
//! (`Δ_out` may differ from `Δ_in`, spec § 3.2): `psi` and `φ_out` write
//! themselves (spec: "φ_out is the only rule that writes the diagram"),
//! always by interning (A3: canonical result by construction).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, PolitiqueMemoire, Pool};

use crate::error::CoreError;
use crate::memoire::{EffetMemoire, MemoireNiveau};
use crate::observer::{CanalAttr, Observer, TraceEvent};

/// Memo key: (operation fingerprint, node, inherited attribute,
/// level-memory version) — lemma L1.
type CleMemo<H> = (u64, NodeId, H, u64);

/// Context of `φ_out` (T-NODE): groups the restructuring parameters
/// (spec § 3.1: `φ_out(ℓ, h, {v ↦ (D', s)}, M)`).
pub struct ContextePhiOut<'a, H, S, M: Clone + Eq + Hash + Debug> {
    /// Anchor of the root.
    pub anchor: Anchor,
    /// Level of the node being restructured.
    pub level: Level,
    /// Inherited attribute at the node.
    pub h: &'a H,
    /// Already-transduced children: `{v ↦ (D', s)}`.
    pub arcs: BTreeMap<u64, (NodeId, S)>,
    /// Level memory in read (M2-3).
    pub memoire: &'a MemoireNiveau<M>,
    /// Input node being visited (key of the level memory).
    pub noeud_entree: NodeId,
}

/// Attributed transduction (spec § 3.1): three partial functions of the
/// controller + initial element of the inherited channel + level memory `M`
/// (M2-3, T3). Input: pool with payload `bool` (set-theoretic reading);
/// output: pool with payload `Out`.
pub trait Transduction: Send + Sync {
    /// Inherited attribute (downward).
    type H: Clone + PartialEq + Eq + Hash + Debug + Send + Sync;
    /// Synthesized attribute (upward).
    type S: Clone + Debug + Send + Sync;
    /// Payload of the terminals of the output diagram (`Δ_out`, spec § 3.2).
    type Out: Clone + Eq + Hash + Debug + Send + Sync;
    /// Value stored in the level memory `M : (id(node), ℓ) ⇀ M`
    /// (M2-3). `()` for transductions without memory (M1).
    type M: Clone + Eq + Hash + Debug + Send + Sync;

    /// Fingerprint of the operation (parameters included): enters the
    /// memo key `(op, node, h, version(M))` (L1/L3).
    fn fingerprint(&self) -> u64;

    /// Initial element of the inherited channel.
    fn h0(&self) -> Self::H;

    /// (T-NODE) dispatch: inherited attribute along arc `v` of level `ℓ`.
    fn phi_disp(&self, level: Level, h: &Self::H, arc: u64) -> Self::H;

    /// (T-NODE) synthesis: aggregation of the children's attributes. Each arc
    /// carries the **level of the child** (`None` if the child is a terminal):
    /// needed by the A4 expansion-counting (the levels skipped between `ℓ`
    /// and the child were collapsed, their cardinality enters the fold).
    fn phi_aggr(
        &self,
        level: Level,
        h: &Self::H,
        arcs: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S;

    /// (T-NODE) restructuring: result node interned in the output pool from
    /// the already-transduced children `{v ↦ (D', s)}` (A3), with
    /// level-memory read and production of a memory effect
    /// `μ` (M2-3, spec § 3.1: `φ_out(ℓ, h, {v ↦ (D', s)}, M) ↦ (n', μ)`).
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffetMemoire<Self::M>);

    /// (T-TERM): interned result terminal node + synthesized attribute.
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S);
}

/// Runner of a transduction with its computed table (memo L1/L3) and its
/// level memory (M2-3).
pub struct Transducteur<T: Transduction> {
    transduction: T,
    memo: HashMap<CleMemo<T::H>, (NodeId, T::S)>,
    memoire: MemoireNiveau<T::M>,
    politique: PolitiqueMemoire,
    observateur: Option<Arc<dyn Observer>>,
}

impl<T: Transduction> Transducteur<T> {
    /// New runner for the given transduction, empty memory (version 0),
    /// policy `Aucune` (M1: no level memory), no observer.
    pub fn new(transduction: T) -> Self {
        Self::avec_politique(transduction, PolitiqueMemoire::Aucune)
    }

    /// Runner with a level-memory storage policy
    /// declared by the controller (M2-3, T3).
    pub fn avec_politique(transduction: T, politique: PolitiqueMemoire) -> Self {
        Transducteur {
            transduction,
            memo: HashMap::new(),
            memoire: MemoireNiveau::new(),
            politique,
            observateur: None,
        }
    }

    /// Runner with trace observer (M2-6): emits `MemoHit`/`MemoMiss`
    /// and the `Attr` of the three channels during execution.
    pub fn avec_observateur(
        transduction: T,
        politique: PolitiqueMemoire,
        observateur: Arc<dyn Observer>,
    ) -> Self {
        Transducteur {
            transduction,
            memo: HashMap::new(),
            memoire: MemoireNiveau::new(),
            politique,
            observateur: Some(observateur),
        }
    }

    /// Emits an event on the observer, if there is one.
    fn notifier(&self, evenement: TraceEvent) {
        if let Some(observateur) = &self.observateur {
            observateur.on_event(&evenement);
        }
    }

    /// Access to the embedded transduction (to inspect the state of a
    /// wrapper, e.g. levels modified by `RestreintAuxNiveaux`).
    pub fn transduction(&self) -> &T {
        &self.transduction
    }

    /// Number of memo entries (for tests: proof of hits).
    pub fn taille_memo(&self) -> usize {
        self.memo.len()
    }

    /// Current version of the level memory (L1).
    pub fn version_memoire(&self) -> u64 {
        self.memoire.version()
    }

    /// Access to the level memory (for tests: produced entries).
    pub fn memoire(&self) -> &MemoireNiveau<T::M> {
        &self.memoire
    }

    /// Runs the transduction at the root, with initial attribute `h0`.
    /// The input is read from `pool_entree` (payload `bool`), the result is
    /// written into `pool_sortie` (payload `Out`).
    pub fn run<PIn, POut>(
        &mut self,
        pool_entree: &PIn,
        pool_sortie: &mut POut,
        anchor: Anchor,
        noeud: NodeId,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        PIn: Pool<Key = u64, Payload = bool>,
        POut: Pool<Key = u64, Payload = T::Out>,
    {
        let h = self.transduction.h0();
        self.run_avec(pool_entree, pool_sortie, anchor, noeud, h)
    }

    /// Single-pool variant: reads and writes in the same pool (payload `Out`
    /// = `bool`). Used by `saturate` (M2-4) where the current iteration lives
    /// in the same pool as the result.
    pub fn run_meme_pool<P>(
        &mut self,
        pool: &mut P,
        anchor: Anchor,
        noeud: NodeId,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        P: Pool<Key = u64, Payload = bool>,
        T: Transduction<Out = bool>,
    {
        let h = self.transduction.h0();
        self.run_avec_meme_pool(pool, anchor, noeud, h)
    }

    fn run_avec<PIn, POut>(
        &mut self,
        pool_entree: &PIn,
        pool_sortie: &mut POut,
        anchor: Anchor,
        noeud: NodeId,
        h: T::H,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        PIn: Pool<Key = u64, Payload = bool>,
        POut: Pool<Key = u64, Payload = T::Out>,
    {
        let cle = (
            self.transduction.fingerprint(),
            noeud,
            h.clone(),
            self.memoire.version(),
        );
        if let Some((resultat, s)) = self.memo.get(&cle) {
            self.notifier(TraceEvent::MemoHit { noeud });
            return Ok((*resultat, s.clone()));
        }
        self.notifier(TraceEvent::MemoMiss { noeud });
        let forme = pool_entree.resolve(noeud)?.clone();
        let (resultat, s) = match forme {
            Form::Terminal(payload) => self.transduction.psi(pool_sortie, anchor, payload, &h),
            Form::Node { level, arcs } => {
                let mut enfants = BTreeMap::new();
                let mut s_arcs = BTreeMap::new();
                for (arc, cible) in arcs {
                    let h_arc = self.transduction.phi_disp(level, &h, arc);
                    self.notifier(TraceEvent::Attr {
                        canal: CanalAttr::Herite,
                        noeud,
                        valeur: format!("{h_arc:?}"),
                    });
                    let (cible_resultat, s_arc) =
                        self.run_avec(pool_entree, pool_sortie, anchor, cible, h_arc)?;
                    let niveau_cible = match pool_entree.resolve(cible) {
                        Ok(Form::Node { level, .. }) => Some(*level),
                        Ok(Form::Terminal(_)) => None,
                        Err(erreur) => return Err(erreur.into()),
                    };
                    enfants.insert(arc, (cible_resultat, s_arc.clone()));
                    s_arcs.insert(arc, (niveau_cible, s_arc));
                }
                let s = self.transduction.phi_aggr(level, &h, &s_arcs);
                self.notifier(TraceEvent::Attr {
                    canal: CanalAttr::Synthétise,
                    noeud,
                    valeur: format!("{s:?}"),
                });
                let ctx = ContextePhiOut {
                    anchor,
                    level,
                    h: &h,
                    arcs: enfants,
                    memoire: &self.memoire,
                    noeud_entree: noeud,
                };
                let (noeud_resultat, effet) = self.transduction.phi_out(pool_sortie, ctx);
                if self.politique == PolitiqueMemoire::Aucune && !effet.ecritures.is_empty() {
                    return Err(CoreError::MemoireNonAutorisee);
                }
                if !effet.ecritures.is_empty() {
                    self.notifier(TraceEvent::Attr {
                        canal: CanalAttr::Memoire,
                        noeud,
                        valeur: format!("{:?}", effet.ecritures),
                    });
                }
                effet.appliquer(&mut self.memoire);
                (noeud_resultat, s)
            }
        };
        self.memo.insert(cle, (resultat, s.clone()));
        Ok((resultat, s))
    }

    fn run_avec_meme_pool<P>(
        &mut self,
        pool: &mut P,
        anchor: Anchor,
        noeud: NodeId,
        h: T::H,
    ) -> Result<(NodeId, T::S), CoreError>
    where
        P: Pool<Key = u64, Payload = bool>,
        T: Transduction<Out = bool>,
    {
        let cle = (
            self.transduction.fingerprint(),
            noeud,
            h.clone(),
            self.memoire.version(),
        );
        if let Some((resultat, s)) = self.memo.get(&cle) {
            self.notifier(TraceEvent::MemoHit { noeud });
            return Ok((*resultat, s.clone()));
        }
        self.notifier(TraceEvent::MemoMiss { noeud });
        let forme = pool.resolve(noeud)?.clone();
        let (resultat, s) = match forme {
            Form::Terminal(payload) => self.transduction.psi(pool, anchor, payload, &h),
            Form::Node { level, arcs } => {
                let mut enfants = BTreeMap::new();
                let mut s_arcs = BTreeMap::new();
                for (arc, cible) in arcs {
                    let h_arc = self.transduction.phi_disp(level, &h, arc);
                    self.notifier(TraceEvent::Attr {
                        canal: CanalAttr::Herite,
                        noeud,
                        valeur: format!("{h_arc:?}"),
                    });
                    let (cible_resultat, s_arc) =
                        self.run_avec_meme_pool(pool, anchor, cible, h_arc)?;
                    let niveau_cible = match pool.resolve(cible) {
                        Ok(Form::Node { level, .. }) => Some(*level),
                        Ok(Form::Terminal(_)) => None,
                        Err(erreur) => return Err(erreur.into()),
                    };
                    enfants.insert(arc, (cible_resultat, s_arc.clone()));
                    s_arcs.insert(arc, (niveau_cible, s_arc));
                }
                let s = self.transduction.phi_aggr(level, &h, &s_arcs);
                self.notifier(TraceEvent::Attr {
                    canal: CanalAttr::Synthétise,
                    noeud,
                    valeur: format!("{s:?}"),
                });
                let ctx = ContextePhiOut {
                    anchor,
                    level,
                    h: &h,
                    arcs: enfants,
                    memoire: &self.memoire,
                    noeud_entree: noeud,
                };
                let (noeud_resultat, effet) = self.transduction.phi_out(pool, ctx);
                if self.politique == PolitiqueMemoire::Aucune && !effet.ecritures.is_empty() {
                    return Err(CoreError::MemoireNonAutorisee);
                }
                if !effet.ecritures.is_empty() {
                    self.notifier(TraceEvent::Attr {
                        canal: CanalAttr::Memoire,
                        noeud,
                        valeur: format!("{:?}", effet.ecritures),
                    });
                }
                effet.appliquer(&mut self.memoire);
                (noeud_resultat, s)
            }
        };
        self.memo.insert(cle, (resultat, s.clone()));
        Ok((resultat, s))
    }
}
