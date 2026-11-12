//! Arena : pool hash-consé bi-plan, implémentation du port `Pool` (M1-1,
//! M4-1). Plans principal et d'ombre (spec § 10 Q7), GC mark-and-sweep avec
//! compaction profilée, métriques de cache.
//!
//! Hash-consing à clé (contrôleur, ancre, forme) : R-share est la table
//! unique. À la création du handle, les réductions du contrôleur s'appliquent
//! (M1-2, spec § 2.3) : R-collapse et R-default.

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_algebra::reduction::{
    Reduction, reduire, reduire_dabs as crate_reduction_dabs, reduire_sans_collapse,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::biplan::{EntreeNoeud, IndexNiveaux, MetriquesPool, Plan, StatsGc, seuil_atteint};
use crate::observer::{Observer, ReductionRule, TraceEvent};

/// Clé du hash-consing : (contrôleur, ancre, forme).
type CleInterning<Key, Payload> = (ControllerId, Anchor, Form<Key, Payload>);

/// Arena : stockage canonique aveugle des formes, adressé par contenu.
/// Détient l'instance de contrôleur : son algèbre de clés (paramétrée par le
/// schéma, M1-3) décide des formes normales, cardinalités et défauts.
pub struct Arena<C: Controller> {
    nodes: Vec<EntreeNoeud<C::Key, C::Payload>>,
    unique: HashMap<CleInterning<C::Key, C::Payload>, NodeId>,
    /// Index du plan d'ombre : niveau → nœuds vivants de ce niveau.
    index_niveaux: IndexNiveaux,
    metriques: MetriquesPool,
    /// Table de remapping de la dernière compaction (vide sinon).
    remap: Vec<Option<NodeId>>,
    observer: Box<dyn Observer>,
    controleur: C,
}

impl<C: Controller> Arena<C> {
    /// Arena neuve avec le contrôleur et l'observateur de trace donnés.
    pub fn new(controleur: C, observer: Box<dyn Observer>) -> Self {
        Arena {
            nodes: Vec::new(),
            unique: HashMap::new(),
            index_niveaux: IndexNiveaux::new(),
            metriques: MetriquesPool::default(),
            remap: Vec::new(),
            observer,
            controleur,
        }
    }

    /// Contrôleur propriétaire de cette arena.
    pub fn controleur(&self) -> &C {
        &self.controleur
    }

    /// Internement d'un terminal à payload (plan principal).
    pub fn terminal(&mut self, anchor: Anchor, payload: C::Payload) -> NodeId {
        self.intern(anchor, Form::Terminal(payload))
    }

    /// Internement dans un plan donné (principal par défaut via `intern`).
    /// Sur un hit du hash-consing (forme déjà présente), le handle existant
    /// est retourné et son plan est déplacé vers `plan`.
    pub fn intern_plan(
        &mut self,
        anchor: Anchor,
        form: Form<C::Key, C::Payload>,
        plan: Plan,
    ) -> NodeId {
        let node = self.intern(anchor, form);
        if let Some(entree) = self.nodes.get_mut(node.0 as usize) {
            entree.plan = plan;
        }
        node
    }

    /// Plan d'un nœud vivant.
    pub fn plan_de(&self, node: NodeId) -> Option<Plan> {
        self.nodes
            .get(node.0 as usize)?
            .forme
            .as_ref()
            .map(|_| self.nodes[node.0 as usize].plan)
    }

    /// Nœuds vivants d'un niveau (index du plan d'ombre).
    pub fn index_niveau(&self, level: Level) -> &BTreeSet<NodeId> {
        static VIDE: std::sync::LazyLock<BTreeSet<NodeId>> =
            std::sync::LazyLock::new(BTreeSet::new);
        self.index_niveaux.get(&level).unwrap_or(&VIDE)
    }

    /// Métriques du pool (cache + GC).
    pub fn metriques(&self) -> MetriquesPool {
        self.metriques
    }

    /// Table de remapping de la dernière compaction : `remap[ancien] =
    /// nouveau` (`None` si l'ancien nœud était une tombstone). Vide si la
    /// dernière GC n'a pas compacté.
    pub fn dernier_remap(&self) -> &[Option<NodeId>] {
        &self.remap
    }

    /// Nombre de nœuds vivants.
    pub fn vivants(&self) -> usize {
        self.nodes.iter().filter(|e| e.forme.is_some()).count()
    }

    /// GC mark-and-sweep depuis les racines données : les nœuds non atteints
    /// deviennent des tombstones (retirés de la table unique et de l'index) ;
    /// si le seuil de morts est atteint, compaction profilée avec table de
    /// remapping des `NodeId`.
    pub fn gc(&mut self, racines: &[NodeId]) -> StatsGc {
        let mut atteints = BTreeSet::new();
        let mut pile: Vec<NodeId> = racines.to_vec();
        while let Some(node) = pile.pop() {
            if !atteints.insert(node) {
                continue;
            }
            if let Some(EntreeNoeud {
                forme: Some(Form::Node { arcs, .. }),
                ..
            }) = self.nodes.get(node.0 as usize)
            {
                pile.extend(arcs.values().copied());
            }
        }
        let total = self.nodes.len() as u64;
        let mut collectes = 0u64;
        for (index, entree) in self.nodes.iter_mut().enumerate() {
            if entree.forme.is_some() && !atteints.contains(&NodeId(index as u32)) {
                entree.forme = None;
                collectes += 1;
            }
        }
        // Retrait des tombstones de la table unique et de l'index d'ombre.
        self.unique
            .retain(|_, node| self.nodes[node.0 as usize].forme.is_some());
        for ensemble in self.index_niveaux.values_mut() {
            ensemble.retain(|node| self.nodes[node.0 as usize].forme.is_some());
        }
        // Vivants comptés sur les entrées réellement vivantes (et non
        // dérivés du nombre de slots, qui inclut les tombstones d'un GC
        // précédent sans compaction).
        let vivants = self.nodes.iter().filter(|e| e.forme.is_some()).count() as u64;
        let compaction = seuil_atteint(collectes, total);
        if compaction {
            self.remap = self.compacter();
        } else {
            self.remap.clear();
        }
        self.metriques.gc += 1;
        self.metriques.vivants = vivants;
        self.metriques.morts += collectes;
        if compaction {
            self.metriques.compactions += 1;
        }
        let stats = StatsGc {
            vivants,
            collectes,
            compaction,
        };
        self.observer.on_event(&TraceEvent::Gc {
            vivants,
            collectes,
            compaction,
        });
        stats
    }

    /// Compaction : reconstruit le vecteur des nœuds sans tombstones et
    /// remappe les `NodeId` (table de remapping retournée). Réservée au
    /// chemin `gc`, qui nettoie d'abord les tombstones de la table unique :
    /// appel direct avec des tombstones laisserait des `NodeId` périmés
    /// dans `unique`.
    fn compacter(&mut self) -> Vec<Option<NodeId>> {
        self.remap.clear();
        let mut remap: Vec<Option<NodeId>> = Vec::with_capacity(self.nodes.len());
        let mut nouveaux: Vec<EntreeNoeud<C::Key, C::Payload>> = Vec::new();
        for entree in self.nodes.drain(..) {
            if entree.forme.is_some() {
                remap.push(Some(NodeId(nouveaux.len() as u32)));
                nouveaux.push(entree);
            } else {
                remap.push(None);
            }
        }
        self.nodes = nouveaux;
        for node in self.unique.values_mut() {
            if let Some(Some(nouveau)) = remap.get(node.0 as usize) {
                *node = *nouveau;
            }
        }
        for ensemble in self.index_niveaux.values_mut() {
            *ensemble = ensemble
                .iter()
                .filter_map(|node| remap.get(node.0 as usize).copied().flatten())
                .collect();
        }
        remap
    }

    /// Forme normalisée d'un nœud : les clés passent par la forme normale de
    /// l'algèbre de clés du contrôleur (sinon le hash-consing violerait C1).
    fn forme_normalisee(&self, form: Form<C::Key, C::Payload>) -> Form<C::Key, C::Payload> {
        match form {
            Form::Terminal(payload) => Form::Terminal(payload),
            Form::Node { level, arcs } => Form::Node {
                level,
                arcs: arcs
                    .into_iter()
                    .map(|(key, cible)| (self.controleur.keys().normal_form(&key), cible))
                    .collect(),
            },
        }
    }

    /// Réductions à la création du handle (spec § 2.3, M1-2) : R-collapse et
    /// R-default, le diagramme de défaut D_δ étant désigné par le contrôleur.
    /// Retourne soit la cible du collapse, soit la forme réduite à interniser.
    fn reduit(
        &self,
        anchor: Anchor,
        level: Level,
        arcs: BTreeMap<C::Key, NodeId>,
    ) -> Result<Form<C::Key, C::Payload>, NodeId> {
        // Atelier didactique (backlog § 9) : le contrôleur peut commuter
        // R-default et R-collapse une à une.
        let defaut_actif = self.controleur.defaut_actif();
        let cle_defaut = if defaut_actif {
            self.controleur.keys().default(level)
        } else {
            None
        };
        let cardinalite = self.controleur.keys().cardinality(level);
        let payload_defaut = if defaut_actif {
            self.controleur.default_payload(level)
        } else {
            None
        };
        let est_defaut = |node: NodeId| -> bool {
            defaut_actif
                && matches!(self.resolve(node), Ok(Form::Terminal(payload)) if payload_defaut.as_ref() == Some(payload))
        };
        // M4-3 : défaut généralisé (D-abs/D-trans) si le contrôleur l'active.
        let reduit = if !self.controleur.collapse_actif() {
            reduire_sans_collapse(arcs, cle_defaut, &est_defaut)
        } else if self.controleur.absorption_generalisee() {
            // NodeId de D^déf sous cette ancre, s'il est déjà interné (sert
            // au D-trans sur forme vide A = ∅).
            let defaut = payload_defaut.as_ref().and_then(|p| {
                self.unique
                    .get(&(Self::CONTROLLER, anchor, Form::Terminal(p.clone())))
                    .copied()
            });
            crate_reduction_dabs(arcs, cle_defaut, &est_defaut, cardinalite, defaut)
        } else {
            reduire(arcs, cle_defaut, &est_defaut, cardinalite)
        };
        match reduit {
            Reduction::Collapse(cible) => Err(cible),
            Reduction::Forme(arcs_reduits) => Ok(Form::Node {
                level,
                arcs: arcs_reduits,
            }),
        }
    }
}

impl<C: Controller> Pool for Arena<C> {
    type Key = C::Key;
    type Payload = C::Payload;

    const CONTROLLER: ControllerId = C::FINGERPRINT;

    fn intern(&mut self, anchor: Anchor, form: Form<Self::Key, Self::Payload>) -> NodeId {
        let forme = self.forme_normalisee(form);
        let forme = match forme {
            Form::Node { level, arcs } => {
                let taille_avant = arcs.len();
                match self.reduit(anchor, level, arcs) {
                    Err(cible) => {
                        self.observer.on_event(&TraceEvent::ReductionApplied {
                            rule: ReductionRule::Collapse,
                            level,
                        });
                        return cible;
                    }
                    Ok(Form::Node { arcs, .. }) => {
                        if arcs.len() < taille_avant {
                            self.observer.on_event(&TraceEvent::ReductionApplied {
                                rule: ReductionRule::Default,
                                level,
                            });
                        }
                        Form::Node { level, arcs }
                    }
                    Ok(terminal) => terminal,
                }
            }
            terminal => terminal,
        };
        let cle = (Self::CONTROLLER, anchor, forme.clone());
        if let Some(node) = self.unique.get(&cle) {
            self.metriques.partages += 1;
            self.observer
                .on_event(&TraceEvent::NodeShared { node: *node });
            return *node;
        }
        let node = NodeId(self.nodes.len() as u32);
        let level = match &forme {
            Form::Terminal(_) => None,
            Form::Node { level, .. } => Some(*level),
        };
        self.nodes.push(EntreeNoeud {
            plan: Plan::Principal,
            forme: Some(forme),
        });
        if let Some(level) = level {
            self.index_niveaux.entry(level).or_default().insert(node);
        }
        self.unique.insert(cle, node);
        self.metriques.creations += 1;
        self.observer
            .on_event(&TraceEvent::NodeCreated { node, level });
        node
    }

    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError> {
        match self.nodes.get(node.0 as usize) {
            Some(entree) => entree.forme.as_ref().ok_or(PoolError::UnknownNode(node)),
            None => Err(PoolError::UnknownNode(node)),
        }
    }

    fn handle(&self, node: NodeId, anchor: Anchor) -> Handle {
        Handle {
            controller: Self::CONTROLLER,
            anchor,
            node,
        }
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
