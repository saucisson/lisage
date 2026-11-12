//! **Atelier didactique** — tableau de bord des réductions (backlog § 9 de
//! la feuille de route, demande du 10 novembre 2026) : rendre manipulable la
//! thèse « le diagramme est une fonction du contrôleur » en
//! activant/désactivant chaque réduction une à une et en mesurant.
//!
//! Premier fragment (backend, sans front web) :
//!
//! - [`Reductions`] : commutateurs R-collapse / R-default ;
//! - [`AtelierController`] : contrôleur enveloppant qui les commute (le pool
//!   lit les hooks `collapse_actif` / `defaut_actif` à l'interning) ;
//! - [`mesurer`] : trois métriques didactiques — les **nœuds partagés**
//!   (R-share actif, compte de handles atteignables), les **chemins**
//!   (nombre de chemins racine → terminal) et la **taille dépliée** (somme
//!   des longueurs de chemins = nombre de nœuds du DAG déplié, ce que
//!   coûterait l'absence de hash-consing). Exemple fil rouge : R₃ tient en
//!   33 nœuds partagés contre une taille dépliée de 140 (20 états × 7
//!   niveaux), les 20 chemins étant un par état atteignable.
//!
//! R-share lui-même n'est pas commutable (c'est la table unique du pool) :
//! sa désactivation se **mesure** par la taille dépliée, pas par un pool
//! dégradé.

use std::collections::{BTreeSet, HashMap};

use lisage_algebra::interface::{
    Controller, ControllerId, Form, Level, NodeId, PolitiqueMemoire, Pool, PoolError,
};
use lisage_algebra::relatif::GroupeCles;

/// Commutateurs des réductions côté contrôleur (R-share exclu : mesuré).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reductions {
    /// R-collapse actif.
    pub collapse: bool,
    /// R-default actif (absorption de l'arc δ(ℓ) vers D_δ).
    pub defaut: bool,
}

impl Reductions {
    /// Toutes les réductions activées (configuration canonique).
    pub const TOUTES: Reductions = Reductions {
        collapse: true,
        defaut: true,
    };
    /// Aucune réduction côté contrôleur (R-share toujours actif, mesuré).
    pub const AUCUNE: Reductions = Reductions {
        collapse: false,
        defaut: false,
    };
    /// R-collapse commuté, R-default actif.
    pub fn sans_collapse() -> Self {
        Reductions {
            collapse: false,
            defaut: true,
        }
    }
    /// R-default commuté, R-collapse actif.
    pub fn sans_defaut() -> Self {
        Reductions {
            collapse: true,
            defaut: false,
        }
    }
}

/// Contrôleur enveloppant qui commute R-collapse et R-default selon
/// [`Reductions`] ; tout le reste est délégué au contrôleur interne.
pub struct AtelierController<C: Controller> {
    inner: C,
    reductions: Reductions,
}

impl<C: Controller> AtelierController<C> {
    /// Enveloppe `inner` avec les commutateurs `reductions`.
    pub fn nouveau(inner: C, reductions: Reductions) -> Self {
        AtelierController { inner, reductions }
    }

    /// Les commutateurs actifs.
    pub fn reductions(&self) -> Reductions {
        self.reductions
    }
}

impl<C: Controller> Controller for AtelierController<C> {
    type Key = C::Key;
    type Payload = C::Payload;
    type Keys = C::Keys;

    // Empreinte propre à l'enveloppe (hors plage des contrôleurs natifs 1-6).
    const FINGERPRINT: ControllerId = ControllerId(100);

    fn keys(&self) -> &Self::Keys {
        self.inner.keys()
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        if self.reductions.defaut {
            self.inner.default_payload(level)
        } else {
            None
        }
    }

    fn collapse_actif(&self) -> bool {
        self.reductions.collapse
    }

    fn defaut_actif(&self) -> bool {
        self.reductions.defaut
    }

    fn politique_memoire(&self) -> PolitiqueMemoire {
        self.inner.politique_memoire()
    }

    fn absorption_generalisee(&self) -> bool {
        self.reductions.defaut && self.inner.absorption_generalisee()
    }

    fn groupe_cles(&self) -> Option<&dyn GroupeCles<Cle = Self::Key, Argument = i64>> {
        self.inner.groupe_cles()
    }
}

/// Rapport de mesure didactique d'un diagramme.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportAtelier {
    /// Nœuds distincts atteignables (R-share actif) : le diagramme partagé.
    pub noeuds_partages: u64,
    /// Chemins racine → terminal dans le DAG.
    pub chemins: u64,
    /// Somme des longueurs des chemins = nœuds du DAG déplié (coût de
    /// l'absence de hash-consing).
    pub taille_depliee: u64,
}

/// Mesure un diagramme : nœuds partagés, chemins et taille dépliée. Propage
/// l'erreur du pool (un `NodeId` inconnu est une erreur, pas une valeur
/// muette).
pub fn mesurer<P: Pool>(pool: &P, racine: NodeId) -> Result<RapportAtelier, PoolError> {
    Ok(RapportAtelier {
        noeuds_partages: compter_noeuds(pool, racine)?,
        chemins: compter_chemins(pool, racine)?,
        taille_depliee: compter_taille_depliee(pool, racine)?,
    })
}

/// Compte les nœuds distincts atteignables depuis `racine`, terminaux compris.
pub fn compter_noeuds<P: Pool>(pool: &P, racine: NodeId) -> Result<u64, PoolError> {
    let mut visite = BTreeSet::new();
    let mut pile = vec![racine];
    while let Some(noeud) = pile.pop() {
        if !visite.insert(noeud) {
            continue;
        }
        if let Form::Node { arcs, .. } = pool.resolve(noeud)? {
            pile.extend(arcs.values().copied());
        }
    }
    Ok(visite.len() as u64)
}

/// Compte les chemins racine → terminal du DAG (mémoïsé par nœud, saturé) :
/// la taille du diagramme déplié sans R-share.
pub fn compter_chemins<P: Pool>(pool: &P, racine: NodeId) -> Result<u64, PoolError> {
    fn chemins<P: Pool>(
        pool: &P,
        noeud: NodeId,
        memo: &mut HashMap<NodeId, u64>,
    ) -> Result<u64, PoolError> {
        if let Some(total) = memo.get(&noeud) {
            return Ok(*total);
        }
        let total = match pool.resolve(noeud)? {
            Form::Terminal(_) => 1,
            Form::Node { arcs, .. } => {
                let mut somme = 0u64;
                for cible in arcs.values() {
                    somme = somme.saturating_add(chemins(pool, *cible, memo)?);
                }
                somme
            }
        };
        memo.insert(noeud, total);
        Ok(total)
    }
    let mut memo = HashMap::new();
    chemins(pool, racine, &mut memo)
}

/// Somme des longueurs des chemins racine → terminal (mémoïsée par nœud,
/// saturée) : le nombre de nœuds du DAG déplié, c'est-à-dire ce que coûterait
/// l'absence de R-share (un nœud par position de chemin).
pub fn compter_taille_depliee<P: Pool>(pool: &P, racine: NodeId) -> Result<u64, PoolError> {
    fn taille<P: Pool>(
        pool: &P,
        noeud: NodeId,
        memo: &mut HashMap<NodeId, u64>,
    ) -> Result<u64, PoolError> {
        if let Some(total) = memo.get(&noeud) {
            return Ok(*total);
        }
        let total = match pool.resolve(noeud)? {
            Form::Terminal(_) => 1,
            Form::Node { arcs, .. } => {
                let mut somme = 1u64;
                for cible in arcs.values() {
                    somme = somme.saturating_add(taille(pool, *cible, memo)?);
                }
                somme
            }
        };
        memo.insert(noeud, total);
        Ok(total)
    }
    let mut memo = HashMap::new();
    taille(pool, racine, &mut memo)
}
