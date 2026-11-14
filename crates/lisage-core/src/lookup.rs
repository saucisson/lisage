//! `lookup(k)` — the test operation (M1-5, spec § 6): transduction from
//! `DD⟨Δ_map, Bool⟩` to `DD⟨Δ_map, Option(V)⟩`.
//!
//! Two inherited modes (spec § 6.3): `Cherche` (above the key axis) and
//! `Extrait` (in the axis); the mode `Stop(v)` carries the extracted value.
//! The synthesized attribute is a **DD fragment** (not a scalar): the arcs of
//! the value level of the key, targets = terminals `ε(Some(v))` of the output
//! pool. When the level `val(k)` has been collapsed (R-collapse), `φ_out`
//! re-explicates it (T-EXPAND) and grafts the fragment onto each continuation
//! (spec § 6.2, § 6.4).
//!
//! The output diagram lives in a separate pool (`LookupController`, payload
//! `Option<u64>`; D_δ = `ε(None)`, the absent key).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Controller, ControllerId, Form, Level, NodeId, Pool};
use lisage_lang::value::Atom;

use crate::schema::{Domaine, PRESENT, Schema, SchemaKeys};
use crate::transduction::{ContextePhiOut, Transduction};

/// Base of the fingerprint namespace of the `lookup` family (key = base + value).
const FINGERPRINT_BASE_LOOKUP: u64 = 3000;

/// Inherited mode of `lookup` (spec § 6.3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Mode {
    /// Above the axis of the searched key.
    Cherche,
    /// In the axis of the searched key (the value level is next).
    Extrait,
    /// Value `v` extracted at level `val(k)`: the terminal is `Some(v)`.
    Stop(u64),
}

/// Controller of the `lookup` output pool: payload `Option<u64>` (`None` =
/// absent key), same levels as the map schema.
pub struct LookupController {
    schema: Schema,
    keys: SchemaKeys,
}

impl LookupController {
    /// Controller for the schema of the searched map.
    pub fn new(schema: Schema) -> Self {
        let keys = SchemaKeys::new(schema.clone());
        LookupController { schema, keys }
    }

    /// Schema of this instance.
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

/// Transduction `lookup(k)` on `Map(K, V)` (spec § 6.3).
pub struct Lookup {
    /// Searched key.
    cle: Atom,
    /// Index of the key in the canonical order of the map.
    index_cle: usize,
    /// Value domain (for T-EXPAND).
    domaine_valeurs: Domaine,
}

impl Lookup {
    /// `lookup(cle)` for the schema `Map(liste_cles, domaine_valeurs)`.
    pub fn new(cle: Atom, liste_cles: Vec<Atom>, domaine_valeurs: Domaine) -> Self {
        let index_cle = liste_cles
            .iter()
            .position(|c| *c == cle)
            .expect("the searched key must belong to the schema");
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

    /// Graft (spec § 6.4): replaces the terminals of the sub-diagram `noeud`
    /// with `ε(payload)`, internal nodes rebuilt identically.
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
    /// Synthesized DD fragment: arcs of the value level of the key, targets =
    /// terminals `ε(Some(v))` of the output pool.
    type S = BTreeMap<u64, NodeId>;
    type Out = Option<u64>;
    type M = ();

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
        // Canonical union of fragments (merge of the arcs of the value level).
        let mut fragment = BTreeMap::new();
        for (_, s) in arcs.values() {
            fragment.extend(s.iter().map(|(v, n)| (*v, *n)));
        }
        fragment
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
        let anchor = ctx.anchor;
        let level = ctx.level;
        let h = ctx.h;
        let arcs = ctx.arcs;
        // T-EXPAND (spec § 6.2) at the presence level of the searched key: if
        // the `present` child is not a `val(k)` node (collapsed level), we
        // re-explicate the value level and graft `Some(v)` onto each
        // continuation, for each value `v` of the domain.
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
                    return (
                        pool.intern(
                            anchor,
                            Form::Node {
                                level,
                                arcs: arcs_sortie,
                            },
                        ),
                        crate::memoire::EffetMemoire::new(),
                    );
                }
            }
        }
        // Standard restructuring: we re-emit the node with the transduced
        // children (the default arc leads to D_δ = ε(None), implicit).
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(anchor, Form::Node { level, arcs }),
            crate::memoire::EffetMemoire::new(),
        )
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
            // Cherche (key not yet reached) or Extrait (implicit val(k) level,
            // grafted above): not found ⇒ None.
            Mode::Cherche | Mode::Extrait => {
                let terminal = pool.intern(anchor, Form::Terminal(None));
                (terminal, BTreeMap::new())
            }
        }
    }
}
