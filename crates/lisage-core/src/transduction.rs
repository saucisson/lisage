//! Moteur de transductions attribuées (M1-4, spec § 3.1, énoncé L3) :
//! les deux canaux — attribut **hérité** `H` (descendant, `φ_disp`) et
//! **synthétisé** `S` (remontant, `φ_aggr`) — et la mémo à fingerprint
//! `(op, nœud, h)` (lemme L1). Règles T-TERM/T-NODE.
//!
//! Depuis M1-5 (#12), la transduction lit son entrée dans un pool à
//! payload `bool` et écrit dans un pool à payload `Out` quelconque
//! (`Δ_out` peut différer de `Δ_in`, spec § 3.2) : `psi` et `φ_out` écrivent
//! eux-mêmes (spec : « φ_out est la seule règle qui écrit le diagramme »),
//! toujours par interning (A3 : résultat canonique par construction).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, PolitiqueMemoire, Pool};

use crate::error::CoreError;
use crate::memoire::{EffetMemoire, MemoireNiveau};
use crate::observer::{CanalAttr, Observer, TraceEvent};

/// Clé de la mémo : (fingerprint de l'opération, nœud, attribut hérité,
/// version de la mémoire de niveau) — lemme L1.
type CleMemo<H> = (u64, NodeId, H, u64);

/// Contexte de `φ_out` (T-NODE) : regroupe les paramètres de la
/// restructuration (spec § 3.1 : `φ_out(ℓ, h, {v ↦ (D', s)}, M)`).
pub struct ContextePhiOut<'a, H, S, M: Clone + Eq + Hash + Debug> {
    /// Ancre de la racine.
    pub anchor: Anchor,
    /// Niveau du nœud en cours de restructuration.
    pub level: Level,
    /// Attribut hérité au nœud.
    pub h: &'a H,
    /// Fils déjà transduits : `{v ↦ (D', s)}`.
    pub arcs: BTreeMap<u64, (NodeId, S)>,
    /// Mémoire de niveau en lecture (M2-3).
    pub memoire: &'a MemoireNiveau<M>,
    /// Nœud d'entrée en cours de visite (clé de la mémoire de niveau).
    pub noeud_entree: NodeId,
}

/// Transduction attribuée (spec § 3.1) : trois fonctions partielles du
/// contrôleur + élément initial du canal hérité + mémoire de niveau `M`
/// (M2-3, T3). Entrée : pool à payload `bool` (lecture ensembliste) ;
/// sortie : pool à payload `Out`.
pub trait Transduction: Send + Sync {
    /// Attribut hérité (descendant).
    type H: Clone + PartialEq + Eq + Hash + Debug + Send + Sync;
    /// Attribut synthétisé (remontant).
    type S: Clone + Debug + Send + Sync;
    /// Payload des terminaux du diagramme de sortie (`Δ_out`, spec § 3.2).
    type Out: Clone + Eq + Hash + Debug + Send + Sync;
    /// Valeur stockée dans la mémoire de niveau `M : (id(nœud), ℓ) ⇀ M`
    /// (M2-3). `()` pour les transductions sans mémoire (M1).
    type M: Clone + Eq + Hash + Debug + Send + Sync;

    /// Fingerprint de l'opération (paramètres compris) : entre dans la clé
    /// de mémo `(op, nœud, h, version(M))` (L1/L3).
    fn fingerprint(&self) -> u64;

    /// Élément initial du canal hérité.
    fn h0(&self) -> Self::H;

    /// (T-NODE) dispatch : attribut hérité le long de l'arc `v` du niveau `ℓ`.
    fn phi_disp(&self, level: Level, h: &Self::H, arc: u64) -> Self::H;

    /// (T-NODE) synthèse : agrégation des attributs des fils. Chaque arc
    /// porte le **niveau du fils** (`None` si le fils est un terminal) :
    /// nécessaire à l'expansion-comptage A4 (les niveaux sautés entre `ℓ`
    /// et le fils ont été collapsés, leur cardinalité entre dans le pli).
    fn phi_aggr(
        &self,
        level: Level,
        h: &Self::H,
        arcs: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S;

    /// (T-NODE) restructuration : nœud résultat interné dans le pool de
    /// sortie à partir des fils déjà transduits `{v ↦ (D', s)}` (A3), avec
    /// lecture de la mémoire de niveau et production d'un effet mémoire
    /// `μ` (M2-3, spec § 3.1 : `φ_out(ℓ, h, {v ↦ (D', s)}, M) ↦ (n', μ)`).
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffetMemoire<Self::M>);

    /// (T-TERM) : nœud terminal résultat interné + attribut synthétisé.
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S);
}

/// Runner d'une transduction avec sa computed table (mémo L1/L3) et sa
/// mémoire de niveau (M2-3).
pub struct Transducteur<T: Transduction> {
    transduction: T,
    memo: HashMap<CleMemo<T::H>, (NodeId, T::S)>,
    memoire: MemoireNiveau<T::M>,
    politique: PolitiqueMemoire,
    observateur: Option<Arc<dyn Observer>>,
}

impl<T: Transduction> Transducteur<T> {
    /// Runner neuf pour la transduction donnée, mémoire vide (version 0),
    /// politique `Aucune` (M1 : pas de mémoire de niveau), sans observateur.
    pub fn new(transduction: T) -> Self {
        Self::avec_politique(transduction, PolitiqueMemoire::Aucune)
    }

    /// Runner avec une politique de stockage de la mémoire de niveau
    /// déclarée par le contrôleur (M2-3, T3).
    pub fn avec_politique(transduction: T, politique: PolitiqueMemoire) -> Self {
        Transducteur {
            transduction,
            memo: HashMap::new(),
            memoire: MemoireNiveau::new(),
            politique,
            observateur: None,
        }
    }

    /// Runner avec observateur de trace (M2-6) : émet `MemoHit`/`MemoMiss`
    /// et les `Attr` des trois canaux pendant l'exécution.
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

    /// Émet un événement sur l'observateur, s'il y en a un.
    fn notifier(&self, evenement: TraceEvent) {
        if let Some(observateur) = &self.observateur {
            observateur.on_event(&evenement);
        }
    }

    /// Accès à la transduction embarquée (pour inspecter l'état d'un
    /// wrapper, ex. niveaux modifiés par `RestreintAuxNiveaux`).
    pub fn transduction(&self) -> &T {
        &self.transduction
    }

    /// Nombre d'entrées de la mémo (pour les tests : preuve des hits).
    pub fn taille_memo(&self) -> usize {
        self.memo.len()
    }

    /// Version courante de la mémoire de niveau (L1).
    pub fn version_memoire(&self) -> u64 {
        self.memoire.version()
    }

    /// Accès à la mémoire de niveau (pour les tests : entrées produites).
    pub fn memoire(&self) -> &MemoireNiveau<T::M> {
        &self.memoire
    }

    /// Exécute la transduction à la racine, avec l'attribut initial `h0`.
    /// L'entrée est lue dans `pool_entree` (payload `bool`), le résultat est
    /// écrit dans `pool_sortie` (payload `Out`).
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

    /// Variante mono-pool : lit et écrit dans le même pool (payload `Out`
    /// = `bool`). Utilisée par `saturate` (M2-4) où l'itération courante vit
    /// dans le même pool que le résultat.
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
