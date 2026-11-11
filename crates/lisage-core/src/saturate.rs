//! `saturate` — point fixe croissant d'une transduction d'étape (M2-4,
//! spec § 7, énoncés T2/T3) : `saturate(τ, D) = νX. D ⊕ τ(X, h₀)`.
//!
//! Deux politiques de saturation, deux instances d'une même sémantique du
//! point fixe (même résultat final, stratégie d'itération différente) :
//! - [`PolitiqueSaturation::Statique`] (à la Ciardo) : `τ` n'est appliquée
//!   qu'aux niveaux déclarés ;
//! - [`PolitiqueSaturation::AutoDetectee`] (à la Hamez) : tous les niveaux
//!   sont actifs au départ ; un niveau dont l'itération courante n'a rien
//!   changé est retiré de l'ensemble actif.
//!
//! Le merge de construction `⊕` est l'union idempotente (Bool, A5).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::pli::union;
use crate::schema::SchemaKeys;
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Borne de sécurité de la boucle de point fixe (T2 garantit la terminaison
/// sur schéma borné ; la borne ne sert qu'à détecter une régression).
pub const BORNE_ITERATIONS: u32 = 1_000;

/// Sel du fingerprint du wrapper [`RestreintAuxNiveaux`] : sépare
/// domain-ement le fingerprint du wrapper de celui de la transduction
/// interne.
const SEL_FINGERPRINT_WRAPPER: u64 = 0x5A7A_0000_0000_0000;

/// Politique de saturation (M2-4) : deux instances d'une même sémantique
/// du point fixe. Les deux politiques produisent le même point fixe lorsque
/// `Statique` déclare tous les niveaux ; seule la stratégie d'itération
/// diffère.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolitiqueSaturation {
    /// Statique par niveaux (à la Ciardo) : seuls les niveaux listés sont
    /// saturés à chaque itération.
    Statique(Vec<Level>),
    /// Auto-détectée (à la Hamez) : tous les niveaux sont actifs au départ ;
    /// un niveau est retiré dès qu'une itération ne change rien à ce niveau.
    AutoDetectee,
}

/// Paramètres d'une saturation (record, ADR-0003).
pub struct ParametresSaturation<'a, T, P> {
    /// Transduction d'étape `τ` (cloneable pour les itérations).
    pub transduction: &'a T,
    /// Politique de saturation.
    pub politique: &'a PolitiqueSaturation,
    /// Pool unique (lecture + écriture, payload `bool`) : l'itération
    /// courante et le résultat vivent dans le même pool.
    pub pool: &'a mut P,
    /// Algèbre de clés du schéma (pour l'union).
    pub keys: &'a SchemaKeys,
    /// Ancre de la racine.
    pub anchor: Anchor,
    /// Nœud initial `D`.
    pub initial: NodeId,
    /// Nombre de niveaux du schéma (pour AutoDetectee : tous les niveaux).
    pub nombre_niveaux: u32,
}

/// Résultat d'une saturation : nœud du point fixe + nombre d'itérations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultatSaturation {
    /// Nœud du point fixe atteint.
    pub noeud: NodeId,
    /// Nombre d'itérations de la boucle de point fixe.
    pub iterations: u32,
}

/// Transduction wrapper qui restreint une transduction d'étape à un
/// ensemble de niveaux actifs : aux niveaux actifs, elle délègue à la
/// transduction interne et enregistre les niveaux modifiés ; aux autres,
/// copie identité.
pub struct RestreintAuxNiveaux<T> {
    inner: T,
    niveaux_actifs: Vec<Level>,
    niveaux_modifies: Mutex<BTreeSet<Level>>,
}

impl<T: Clone> Clone for RestreintAuxNiveaux<T> {
    fn clone(&self) -> Self {
        // Le clone repart d'un suivi vierge : `saturer` crée de toute façon
        // un wrapper neuf à chaque itération.
        RestreintAuxNiveaux {
            inner: self.inner.clone(),
            niveaux_actifs: self.niveaux_actifs.clone(),
            niveaux_modifies: Mutex::new(BTreeSet::new()),
        }
    }
}

impl<T> RestreintAuxNiveaux<T> {
    /// Wrapper neuf : `inner` restreinte aux `niveaux_actifs`.
    pub fn new(inner: T, niveaux_actifs: Vec<Level>) -> Self {
        RestreintAuxNiveaux {
            inner,
            niveaux_actifs,
            niveaux_modifies: Mutex::new(BTreeSet::new()),
        }
    }

    /// Niveaux auxquels `φ_out` a produit un nœud différent de l'entrée
    /// depuis la création du wrapper (politique AutoDetectee).
    pub fn niveaux_modifies(&self) -> BTreeSet<Level> {
        self.niveaux_modifies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl<T: Transduction> Transduction for RestreintAuxNiveaux<T> {
    type H = T::H;
    type S = T::S;
    type Out = T::Out;
    type M = T::M;

    fn fingerprint(&self) -> u64 {
        self.inner.fingerprint() ^ SEL_FINGERPRINT_WRAPPER
    }
    fn h0(&self) -> Self::H {
        self.inner.h0()
    }
    fn phi_disp(&self, level: Level, h: &Self::H, arc: u64) -> Self::H {
        self.inner.phi_disp(level, h, arc)
    }
    fn phi_aggr(
        &self,
        level: Level,
        h: &Self::H,
        arcs: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S {
        self.inner.phi_aggr(level, h, arcs)
    }
    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
        if self.niveaux_actifs.contains(&ctx.level) {
            let noeud_entree = ctx.noeud_entree;
            let level = ctx.level;
            let (noeud, effet) = self.inner.phi_out(pool, ctx);
            if noeud != noeud_entree {
                self.niveaux_modifies
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(level);
            }
            (noeud, effet)
        } else {
            let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
            (
                pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: ctx.level,
                        arcs,
                    },
                ),
                crate::memoire::EffetMemoire::new(),
            )
        }
    }
    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        h: &Self::H,
    ) -> (NodeId, Self::S) {
        self.inner.psi(pool, anchor, payload, h)
    }
}

/// Point fixe croissant `saturate(τ, D)` sur le pool (spec § 7, T2) :
/// itère `X ← X ⊕ τ(X)` jusqu'à stabilisation, en appliquant `τ` selon la
/// politique (restreinte aux niveaux actifs ; AutoDetectee retire les
/// niveaux stabilisés de l'ensemble actif).
///
/// La terminaison (T2) est garantie sur schéma borné : l'ensemble des
/// formes canoniques est fini et la chaîne est croissante pour l'inclusion
/// des dénotations. La [`BORNE_ITERATIONS`] ne sert que de filet de
/// sécurité contre une régression.
pub fn saturer<T, P>(
    params: &mut ParametresSaturation<'_, T, P>,
) -> Result<ResultatSaturation, CoreError>
where
    T: Transduction<Out = bool> + Clone,
    P: Pool<Key = u64, Payload = bool>,
{
    // Niveaux actifs selon la politique.
    let mut niveaux_actifs: Vec<Level> = match params.politique {
        PolitiqueSaturation::Statique(niveaux) => niveaux.clone(),
        PolitiqueSaturation::AutoDetectee => (0..params.nombre_niveaux).map(Level).collect(),
    };

    let mut courant = params.initial;
    let mut iterations = 0u32;
    loop {
        iterations += 1;
        if iterations > BORNE_ITERATIONS {
            return Err(CoreError::NonTerminaison {
                borne: BORNE_ITERATIONS,
            });
        }
        let etape = RestreintAuxNiveaux::new(params.transduction.clone(), niveaux_actifs.clone());
        // Spec § 7 : les étapes de saturate peuvent produire un effet mémoire
        // (T3, M2-5) — politique ParNiveau. La mémoire vit le temps d'une
        // passe : le runner (et sa mémoire) est recréé à chaque itération.
        let mut runner = Transducteur::avec_politique(
            etape,
            lisage_algebra::interface::PolitiqueMemoire::ParNiveau,
        );
        let (image, _) = runner.run_meme_pool(params.pool, params.anchor, courant)?;
        let suivant = union(params.pool, params.keys, params.anchor, courant, image)?;
        if suivant == courant {
            return Ok(ResultatSaturation {
                noeud: courant,
                iterations,
            });
        }
        if matches!(params.politique, PolitiqueSaturation::AutoDetectee) {
            // Politique Hamez : un niveau non modifié par l'itération est
            // retiré de l'ensemble actif.
            let modifies = runner.transduction().niveaux_modifies();
            niveaux_actifs.retain(|niveau| modifies.contains(niveau));
            if niveaux_actifs.is_empty() {
                return Ok(ResultatSaturation {
                    noeud: suivant,
                    iterations,
                });
            }
        }
        courant = suivant;
    }
}
