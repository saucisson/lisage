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
//!   (nombre de chemins racine → terminal) et la **taille dépliée** (nombre
//!   de nœuds de l'arbre déplié, chaque arc dupliquant le sous-arbre cible :
//!   ce que coûterait l'absence de hash-consing). Exemple fil rouge : R₃ tient en
//!   33 nœuds partagés contre une taille dépliée de 126, les 20 chemins
//!   étant un par état atteignable.
//!
//! R-share lui-même n'est pas commutable (c'est la table unique du pool) :
//! sa désactivation se **mesure** par la taille dépliée, pas par un pool
//! dégradé.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Level, NodeId, PolitiqueMemoire, Pool, PoolError,
};
use lisage_algebra::relatif::GroupeCles;

use crate::observer::{CanalAttr, Observer, TraceEvent};

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
    /// Nœuds de l'arbre déplié (chaque arc dupliquant le sous-arbre cible) :
    /// le coût de l'absence de hash-consing.
    pub taille_depliee: u64,
}

/// Mesure un diagramme : nœuds partagés, chemins et taille dépliée. Propage
/// l'erreur du pool (un `NodeId` inconnu est une erreur, pas une valeur
/// muette). `observateur` : canal de narration (ADR-0100) — un événement
/// `Attr`/`Synthétise` porte le rapport complet.
pub fn mesurer<P: Pool>(
    pool: &P,
    racine: NodeId,
    observateur: Option<&dyn Observer>,
) -> Result<RapportAtelier, PoolError> {
    let rapport = RapportAtelier {
        noeuds_partages: compter_noeuds(pool, racine)?,
        chemins: compter_chemins(pool, racine)?,
        taille_depliee: compter_taille_depliee(pool, racine)?,
    };
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine,
            valeur: format!(
                "noeuds_partages={} chemins={} taille_depliee={}",
                rapport.noeuds_partages, rapport.chemins, rapport.taille_depliee
            ),
        });
    }
    Ok(rapport)
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

/// Compte les chemins racine → terminal du DAG (mémoïsé par nœud, saturé).
/// Ce n'est pas la taille dépliée : celle-ci est la somme des longueurs de
/// chemins, calculée par [`compter_taille_depliee`].
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

/// Nombre de nœuds de l'arbre déplié (mémoïsé par nœud, saturé) : chaque
/// arc duplique le sous-arbre de sa cible, ce que coûterait l'absence de
/// R-share.
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

/// Niveau de la racine de la fenêtre glissante.
const NIVEAU_RACINE: u32 = 0;
/// Niveau du nœud fenêtre de la fenêtre glissante.
const NIVEAU_FENETRE: u32 = 1;

/// **Paramètre d'encodage** (atelier didactique, backlog § 9 : « jouer les
/// paramètres du contrôleur — ordre, encodage absolu/relatif, royaume,
/// ancre composée »).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encodage {
    /// Clés absolues : chaque fenêtre a ses propres clés, un nœud par base.
    Absolu,
    /// Clés relatives (section min = 0, M4-2) : une seule forme partagée,
    /// l'argument de position vit sur l'arc d'entrée.
    Relatif,
}

/// Comparaison didactique des deux encodages sur le même objet logique.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportComparaison {
    /// Métriques de l'encodage absolu.
    pub absolu: RapportAtelier,
    /// Métriques de l'encodage relatif.
    pub relatif: RapportAtelier,
}

/// Construit la **fenêtre glissante** en encodage absolu : pour chaque base
/// `b ∈ [0, nb_bases)`, une fenêtre propre `{b..b+taille-1}` (un nœud par
/// base), toutes reliées à la racine. Le nombre de nœuds croît avec
/// `nb_bases` — c'est le « l'infini de la fenêtre » du prototype plat.
pub fn construire_fenetre_absolue<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    nb_bases: u64,
) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let mut arcs = BTreeMap::new();
    for base in 0..nb_bases {
        let fenetre = pool.intern(
            anchor,
            Form::Node {
                level: Level(NIVEAU_FENETRE),
                arcs: (0..taille).map(|k| (base + k, vrai)).collect(),
            },
        );
        arcs.insert(base, fenetre);
    }
    pool.intern(
        anchor,
        Form::Node {
            level: Level(NIVEAU_RACINE),
            arcs,
        },
    )
}

/// Construit la même fenêtre glissante en encodage **relatif** (M4-2,
/// section min = 0) : une seule forme `{0..taille-1}` partagée par toutes les
/// bases (l'argument de position vit sur l'arc d'entrée). Le nombre de
/// nœuds est constant — c'est le « 4 contre l'infini » de la fenêtre.
pub fn construire_fenetre_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    nb_bases: u64,
) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let fenetre = pool.intern(
        anchor,
        Form::Node {
            level: Level(NIVEAU_FENETRE),
            arcs: (0..taille).map(|k| (k, vrai)).collect(),
        },
    );
    let arcs = (0..nb_bases).map(|base| (base, fenetre)).collect();
    pool.intern(
        anchor,
        Form::Node {
            level: Level(NIVEAU_RACINE),
            arcs,
        },
    )
}

/// Construit la fenêtre glissante dans l'`encodage` demandé : point
/// d'entrée unique du paramètre d'encodage.
pub fn construire_fenetre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    encodage: Encodage,
    taille: u64,
    nb_bases: u64,
) -> NodeId {
    match encodage {
        Encodage::Absolu => construire_fenetre_absolue(pool, anchor, taille, nb_bases),
        Encodage::Relatif => construire_fenetre_relative(pool, anchor, taille, nb_bases),
    }
}

/// Compare les deux encodages de la fenêtre glissante sur `nb_bases` bases :
/// mêmes chemins (même objet logique), nœuds partagés différents — le partage
/// relatif se voit à l'œil nu. `observateur` : narration ADR-0100.
pub fn comparer_encodages_fenetre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    nb_bases: u64,
    observateur: Option<&dyn Observer>,
) -> Result<RapportComparaison, PoolError> {
    let racine_absolue = construire_fenetre(pool, anchor, Encodage::Absolu, taille, nb_bases);
    let racine_relative = construire_fenetre(pool, anchor, Encodage::Relatif, taille, nb_bases);
    let absolu = mesurer(pool, racine_absolue, observateur)?;
    let relatif = mesurer(pool, racine_relative, observateur)?;
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine_relative,
            valeur: format!(
                "encodage=relatif_vs_absolu noeuds={} contre {}",
                relatif.noeuds_partages, absolu.noeuds_partages
            ),
        });
    }
    Ok(RapportComparaison { absolu, relatif })
}
