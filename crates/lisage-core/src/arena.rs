//! Arena : pool hash-consé, implémentation du port `Pool` (M1-1).
//!
//! Hash-consing à clé (contrôleur, ancre, forme) : R-share est la table
//! unique. À la création du handle, les réductions du contrôleur s'appliquent
//! (M1-2, spec § 2.3) : R-collapse et R-default.

use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};
use lisage_algebra::reduction::{Reduction, reduire};

use crate::observer::{Observer, ReductionRule, TraceEvent};

/// Clé du hash-consing : (contrôleur, ancre, forme).
type CleInterning<Key, Payload> = (ControllerId, Anchor, Form<Key, Payload>);

/// Arena : stockage canonique aveugle des formes, adressé par contenu.
pub struct Arena<C: Controller> {
    nodes: Vec<Form<C::Key, C::Payload>>,
    unique: HashMap<CleInterning<C::Key, C::Payload>, NodeId>,
    observer: Box<dyn Observer>,
    controleur: PhantomData<C>,
}

impl<C: Controller> Arena<C> {
    /// Arena neuve avec l'observateur de trace donné.
    pub fn new(observer: Box<dyn Observer>) -> Self {
        Arena {
            nodes: Vec::new(),
            unique: HashMap::new(),
            observer,
            controleur: PhantomData,
        }
    }

    /// Internement d'un terminal à payload.
    pub fn terminal(&mut self, anchor: Anchor, payload: C::Payload) -> NodeId {
        self.intern(anchor, Form::Terminal(payload))
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
                    .map(|(key, cible)| (C::Keys::normal_form(&key), cible))
                    .collect(),
            },
        }
    }

    /// Réductions à la création du handle (spec § 2.3, M1-2) : R-collapse et
    /// R-default, le diagramme de défaut D_δ étant désigné par le contrôleur.
    /// Retourne soit la cible du collapse, soit la forme réduite à interniser.
    fn reduit(
        &self,
        level: Level,
        arcs: BTreeMap<C::Key, NodeId>,
    ) -> Result<Form<C::Key, C::Payload>, NodeId> {
        let cle_defaut = C::Keys::default(level);
        let cardinalite = C::Keys::cardinality(level);
        let payload_defaut = C::default_payload(level);
        let est_defaut = |node: NodeId| -> bool {
            matches!(self.resolve(node), Ok(Form::Terminal(payload)) if payload_defaut.as_ref() == Some(payload))
        };
        match reduire(arcs, cle_defaut, &est_defaut, cardinalite) {
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
                match self.reduit(level, arcs) {
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
            self.observer
                .on_event(&TraceEvent::NodeShared { node: *node });
            return *node;
        }
        let node = NodeId(self.nodes.len() as u32);
        let level = match &forme {
            Form::Terminal(_) => None,
            Form::Node { level, .. } => Some(*level),
        };
        self.nodes.push(forme);
        self.unique.insert(cle, node);
        self.observer
            .on_event(&TraceEvent::NodeCreated { node, level });
        node
    }

    fn resolve(&self, node: NodeId) -> Result<&Form<Self::Key, Self::Payload>, PoolError> {
        self.nodes
            .get(node.0 as usize)
            .ok_or(PoolError::UnknownNode(node))
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
