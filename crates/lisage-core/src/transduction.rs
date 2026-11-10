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

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;

/// Clé de la mémo : (fingerprint de l'opération, nœud, attribut hérité).
type CleMemo<H> = (u64, NodeId, H);

/// Transduction attribuée (spec § 3.1) : trois fonctions partielles du
/// contrôleur + élément initial du canal hérité. Entrée : pool à payload
/// `bool` (lecture ensembliste) ; sortie : pool à payload `Out`.
pub trait Transduction: Send + Sync {
    /// Attribut hérité (descendant).
    type H: Clone + PartialEq + Eq + Hash + Debug + Send + Sync;
    /// Attribut synthétisé (remontant).
    type S: Clone + Debug + Send + Sync;
    /// Payload des terminaux du diagramme de sortie (`Δ_out`, spec § 3.2).
    type Out: Clone + Eq + Hash + Debug + Send + Sync;

    /// Fingerprint de l'opération (paramètres compris) : entre dans la clé
    /// de mémo `(op, nœud, h)` (L3).
    fn fingerprint(&self) -> u64;

    /// Élément initial du canal hérité.
    fn h0(&self) -> Self::H;

    /// (T-NODE) dispatch : attribut hérité le long de l'arc `v` du niveau `ℓ`.
    fn phi_disp(&self, level: Level, h: &Self::H, arc: u64) -> Self::H;

    /// (T-NODE) synthèse : agrégation des attributs des fils.
    fn phi_aggr(&self, level: Level, h: &Self::H, arcs: &BTreeMap<u64, Self::S>) -> Self::S;

    /// (T-NODE) restructuration : nœud résultat interné dans le pool de
    /// sortie à partir des fils déjà transduits `{v ↦ (D', s)}` (A3).
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        h: &Self::H,
        arcs: BTreeMap<u64, (NodeId, Self::S)>,
    ) -> NodeId;

    /// (T-TERM) : nœud terminal résultat interné + attribut synthétisé.
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S);
}

/// Runner d'une transduction avec sa computed table (mémo L1/L3).
pub struct Transducteur<T: Transduction> {
    transduction: T,
    memo: HashMap<CleMemo<T::H>, (NodeId, T::S)>,
}

impl<T: Transduction> Transducteur<T> {
    /// Runner neuf pour la transduction donnée.
    pub fn new(transduction: T) -> Self {
        Transducteur {
            transduction,
            memo: HashMap::new(),
        }
    }

    /// Nombre d'entrées de la mémo (pour les tests : preuve des hits).
    pub fn taille_memo(&self) -> usize {
        self.memo.len()
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
        let cle = (self.transduction.fingerprint(), noeud, h.clone());
        if let Some((resultat, s)) = self.memo.get(&cle) {
            return Ok((*resultat, s.clone()));
        }
        let forme = pool_entree.resolve(noeud)?.clone();
        let (resultat, s) = match forme {
            Form::Terminal(payload) => self.transduction.psi(pool_sortie, anchor, payload, &h),
            Form::Node { level, arcs } => {
                let mut enfants = BTreeMap::new();
                let mut s_arcs = BTreeMap::new();
                for (arc, cible) in arcs {
                    let h_arc = self.transduction.phi_disp(level, &h, arc);
                    let (cible_resultat, s_arc) =
                        self.run_avec(pool_entree, pool_sortie, anchor, cible, h_arc)?;
                    enfants.insert(arc, (cible_resultat, s_arc.clone()));
                    s_arcs.insert(arc, s_arc);
                }
                let s = self.transduction.phi_aggr(level, &h, &s_arcs);
                let noeud_resultat =
                    self.transduction
                        .phi_out(pool_sortie, anchor, level, &h, enfants);
                (noeud_resultat, s)
            }
        };
        self.memo.insert(cle, (resultat, s.clone()));
        Ok((resultat, s))
    }
}
