//! `lookup(k)` — l'opération-test (M1-5, spec § 6) : transduction de
//! `DD⟨Δ_map, Bool⟩` vers `DD⟨Δ_map, Option(V)⟩`.
//!
//! Deux modes hérités (spec § 6.3) : `Cherche` (au-dessus de l'axe de la
//! clé) et `Extrait` (dans l'axe) ; le mode `Stop(v)` porte la valeur extraite.
//! L'attribut synthétisé est un **fragment de DD** (pas un scalaire) : les
//! arcs du niveau valeur de la clé, cibles = terminaux `ε(Some(v))` du pool
//! de sortie. Quand le niveau `val(k)` a été collapsé (R-collapse), `φ_out`
//! le ré-explicite (T-EXPAND) et greffe le fragment sur chaque continuation
//! (spec § 6.2, § 6.4).
//!
//! Le diagramme de sortie vit dans un pool à part (`LookupController`,
//! payload `Option<u64>` ; D_δ = `ε(None)`, la clé absente).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Controller, ControllerId, Form, Level, NodeId, Pool};
use lisage_lang::value::Atom;

use crate::schema::{Domaine, PRESENT, Schema, SchemaKeys};
use crate::transduction::Transduction;

/// Base du namespace de fingerprints de la famille `lookup` (clé = base + valeur).
const FINGERPRINT_BASE_LOOKUP: u64 = 3000;

/// Mode hérité de `lookup` (spec § 6.3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Mode {
    /// Au-dessus de l'axe de la clé cherchée.
    Cherche,
    /// Dans l'axe de la clé cherchée (le niveau valeur vient).
    Extrait,
    /// Valeur `v` extraite au niveau `val(k)` : le terminal vaut `Some(v)`.
    Stop(u64),
}

/// Contrôleur du pool de sortie de `lookup` : payload `Option<u64>`
/// (`None` = clé absente), mêmes niveaux que le schéma de la map.
pub struct LookupController {
    schema: Schema,
    keys: SchemaKeys,
}

impl LookupController {
    /// Contrôleur pour le schéma de la map cherchée.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        LookupController { schema, keys }
    }

    /// Schéma de cette instance.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

impl Controller for LookupController {
    type Key = u64;
    type Payload = Option<u64>;
    type Keys = SchemaKeys;

    const FINGERPRINT: ControllerId = ControllerId(2);

    fn keys(&self) -> &Self::Keys {
        &self.keys
    }

    fn default_payload(&self, level: Level) -> Option<Self::Payload> {
        match &self.schema {
            Schema::Atom(domaine) => domaine.defaut.map(|_| None),
            Schema::Set(_) => Some(None),
            Schema::Map(_, _) => {
                if level.0.is_multiple_of(2) {
                    Some(None)
                } else {
                    None
                }
            }
            Schema::Seq(_, _) => None,
            Schema::Tag(_, _) => None,
        }
    }
}

/// Transduction `lookup(k)` sur `Map(K, V)` (spec § 6.3).
pub struct Lookup {
    /// Clé cherchée.
    cle: Atom,
    /// Index de la clé dans l'ordre canonique de la map.
    index_cle: usize,
    /// Domaine des valeurs (pour le T-EXPAND).
    domaine_valeurs: Domaine,
}

impl Lookup {
    /// `lookup(cle)` pour le schéma `Map(liste_cles, domaine_valeurs)`.
    pub fn new(cle: Atom, liste_cles: Vec<Atom>, domaine_valeurs: Domaine) -> Self {
        let index_cle = liste_cles
            .iter()
            .position(|c| *c == cle)
            .expect("la clé cherchée doit appartenir au schéma");
        Lookup {
            cle,
            index_cle,
            domaine_valeurs,
        }
    }

    fn niveau_presence(&self) -> Level {
        Schema::niveau_presence(self.index_cle)
    }

    fn niveau_valeur(&self) -> Level {
        Schema::niveau_valeur(self.index_cle)
    }

    /// Greffe (spec § 6.4) : remplace les terminaux du sous-diagramme
    /// `noeud` par `ε(payload)`, nœuds internes reconstruits à l'identique.
    fn greffer<P: Pool<Key = u64, Payload = Option<u64>>>(
        pool: &mut P,
        anchor: Anchor,
        noeud: NodeId,
        payload: Option<u64>,
        memo: &mut BTreeMap<NodeId, NodeId>,
    ) -> NodeId {
        if let Some(&n) = memo.get(&noeud) {
            return n;
        }
        let forme = pool.resolve(noeud).unwrap().clone();
        let resultat = match forme {
            Form::Terminal(_) => pool.intern(anchor, Form::Terminal(payload)),
            Form::Node { level, arcs } => {
                let arcs = arcs
                    .iter()
                    .map(|(v, c)| (*v, Self::greffer(pool, anchor, *c, payload, memo)))
                    .collect();
                pool.intern(anchor, Form::Node { level, arcs })
            }
        };
        memo.insert(noeud, resultat);
        resultat
    }
}

impl Transduction for Lookup {
    type H = Mode;
    /// Fragment de DD synthétisé : arcs du niveau valeur de la clé,
    /// cibles = terminaux `ε(Some(v))` du pool de sortie.
    type S = BTreeMap<u64, NodeId>;
    type Out = Option<u64>;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_LOOKUP + self.cle.0
    }

    fn h0(&self) -> Mode {
        Mode::Cherche
    }

    fn phi_disp(&self, level: Level, h: &Mode, arc: u64) -> Mode {
        match h {
            Mode::Cherche => {
                if level == self.niveau_presence() && arc == PRESENT {
                    Mode::Extrait
                } else {
                    Mode::Cherche
                }
            }
            Mode::Extrait => {
                if level == self.niveau_valeur() {
                    Mode::Stop(arc)
                } else {
                    Mode::Extrait
                }
            }
            Mode::Stop(v) => Mode::Stop(*v),
        }
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &Mode,
        arcs: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S {
        // Union canonique des fragments (fusion des arcs du niveau valeur).
        let mut fragment = BTreeMap::new();
        for (_, s) in arcs.values() {
            fragment.extend(s.iter().map(|(v, n)| (*v, *n)));
        }
        fragment
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        level: Level,
        h: &Mode,
        arcs: BTreeMap<u64, (NodeId, Self::S)>,
    ) -> NodeId {
        // T-EXPAND (spec § 6.2) au niveau de présence de la clé cherchée :
        // si le fils `présent` n'est pas un nœud `val(k)` (niveau collapsé),
        // on ré-explicite le niveau valeur et on greffe `Some(v)` sur chaque
        // continuation, pour chaque valeur `v` du domaine.
        if *h == Mode::Cherche && level == self.niveau_presence() {
            let enfant_present = arcs.get(&PRESENT).map(|(n, _)| *n);
            if let Some(enfant) = enfant_present {
                let enfant_est_niveau_valeur = matches!(
                    pool.resolve(enfant),
                    Ok(Form::Node { level: l, .. }) if *l == self.niveau_valeur()
                );
                if !enfant_est_niveau_valeur {
                    let mut greffon = BTreeMap::new();
                    for v in &self.domaine_valeurs.valeurs {
                        let mut memo = BTreeMap::new();
                        let branche = Self::greffer(pool, anchor, enfant, Some(v.0), &mut memo);
                        greffon.insert(v.0, branche);
                    }
                    let noeud_valeur = pool.intern(
                        anchor,
                        Form::Node {
                            level: self.niveau_valeur(),
                            arcs: greffon,
                        },
                    );
                    let mut arcs_sortie: BTreeMap<u64, NodeId> = arcs
                        .into_iter()
                        .filter(|(v, _)| *v != PRESENT)
                        .map(|(v, (n, _))| (v, n))
                        .collect();
                    arcs_sortie.insert(PRESENT, noeud_valeur);
                    return pool.intern(
                        anchor,
                        Form::Node {
                            level,
                            arcs: arcs_sortie,
                        },
                    );
                }
            }
        }
        // Restructuration standard : on ré-émet le nœud avec les fils
        // transduits (l'arc par défaut mène à D_δ = ε(None), implicite).
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        pool.intern(anchor, Form::Node { level, arcs })
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        _payload: bool,
        h: &Mode,
    ) -> (NodeId, Self::S) {
        match h {
            Mode::Stop(v) => {
                let terminal = pool.intern(anchor, Form::Terminal(Some(*v)));
                (terminal, BTreeMap::from([(*v, terminal)]))
            }
            // Cherche (clé pas encore atteinte) ou Extrait (niveau val(k)
            // implicite, greffé plus haut) : pas trouvé ⇒ None.
            Mode::Cherche | Mode::Extrait => {
                let terminal = pool.intern(anchor, Form::Terminal(None));
                (terminal, BTreeMap::new())
            }
        }
    }
}
