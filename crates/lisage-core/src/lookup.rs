//! `lookup(k)` — the test operation (M1-5, spec § 6): transduction from
//! `DD⟨Δ_map, Bool⟩` to `DD⟨Δ_map, Option(V)⟩`.
//!
//! Two inherited modes (spec § 6.3): `Cherche` (above the axis of the
//! key) and `Extrait` (in the axis); the `Stop(v)` mode carries the extracted value.
//! The synthesized attribute is a **DD fragment** (not a scalar): the
//! edges of the value level of the key, targets = terminals `ε(Some(v))` of the
//! output pool. When the level `val(k)` has been collapsed (R-collapse), `φ_out`
//! re-explicits it (T-EXPAND) and grafts the fragment on each continuation
//! (spec § 6.2, § 6.4).
//!
//! The output diagram lives in a separate pool (`LookupController`,
//! payload `Option<u64>`; D_δ = `ε(None)`, the absent key).

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Controller, ControllerId, Form, Level, NodeId, Pool};
use lisage_lang::value::Atom;

use crate::schema::{Domain, PRESENT, Schema, SchemaKeys};
use crate::transduction::{ContextPhiOut, Transduction};

/// Base of the fingerprint namespace of the `lookup` family (key = base + value).
const FINGERPRINT_BASE_LOOKUP: u64 = 3000;

/// Inherited mode of `lookup` (spec § 6.3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Mode {
    /// Above the axis of the sought key.
    Cherche,
    /// In the axis of the sought key (the value level is coming).
    Extrait,
    /// Value `v` extracted at level `val(k)`: the terminal is `Some(v)`.
    Stop(u64),
}

/// Controller of the `lookup` output pool: payload `Option<u64>`
/// (`None` = absent key), same levels as the map schema.
pub struct LookupController {
    schema: Schema,
    keys: SchemaKeys,
}

impl LookupController {
    /// Controller for the schema of the sought map.
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
            Schema::Atom(domain) => domain.default.map(|_| None),
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
    /// Sought key.
    key: Atom,
    /// Index of the key in the canonical order of the map.
    index_key: usize,
    /// Domain of the values (for T-EXPAND).
    domain_values: Domain,
}

impl Lookup {
    /// `lookup(key)` for the schema `Map(list_keys, domain_values)`.
    pub fn new(key: Atom, list_keys: Vec<Atom>, domain_values: Domain) -> Self {
        let index_key = list_keys
            .iter()
            .position(|c| *c == key)
            .expect("the sought key must belong to the schema");
        Lookup {
            key,
            index_key,
            domain_values,
        }
    }

    fn level_presence(&self) -> Level {
        Schema::level_presence(self.index_key)
    }

    fn level_value(&self) -> Level {
        Schema::level_value(self.index_key)
    }

    /// Graft (spec § 6.4): replaces the terminals of the sub-diagram
    /// `node` by `ε(payload)`, internal nodes rebuilt identically.
    fn greffer<P: Pool<Key = u64, Payload = Option<u64>>>(
        pool: &mut P,
        anchor: Anchor,
        node: NodeId,
        payload: Option<u64>,
        memo: &mut BTreeMap<NodeId, NodeId>,
    ) -> NodeId {
        if let Some(&n) = memo.get(&node) {
            return n;
        }
        let form = pool.resolve(node).unwrap().clone();
        let result = match form {
            Form::Terminal(_) => pool.intern(anchor, Form::Terminal(payload)),
            Form::Node { level, edges } => {
                let edges = edges
                    .iter()
                    .map(|(v, c)| (*v, Self::greffer(pool, anchor, *c, payload, memo)))
                    .collect();
                pool.intern(anchor, Form::Node { level, edges })
            }
        };
        memo.insert(node, result);
        result
    }
}

impl Transduction for Lookup {
    type H = Mode;
    /// Synthesized DD fragment: edges of the value level of the key,
    /// targets = `ε(Some(v))` terminals of the output pool.
    type S = BTreeMap<u64, NodeId>;
    type Out = Option<u64>;
    type M = ();

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_BASE_LOOKUP + self.key.0
    }

    fn h0(&self) -> Mode {
        Mode::Cherche
    }

    fn phi_dispatch(&self, level: Level, h: &Mode, edge: u64) -> Mode {
        match h {
            Mode::Cherche => {
                if level == self.level_presence() && edge == PRESENT {
                    Mode::Extrait
                } else {
                    Mode::Cherche
                }
            }
            Mode::Extrait => {
                if level == self.level_value() {
                    Mode::Stop(edge)
                } else {
                    Mode::Extrait
                }
            }
            Mode::Stop(v) => Mode::Stop(*v),
        }
    }

    fn phi_aggregate(
        &self,
        _level: Level,
        _h: &Mode,
        edges: &BTreeMap<u64, (Option<Level>, Self::S)>,
    ) -> Self::S {
        // Canonical union of the fragments (merge of the value-level edges).
        let mut fragment = BTreeMap::new();
        for (_, s) in edges.values() {
            fragment.extend(s.iter().map(|(v, n)| (*v, *n)));
        }
        fragment
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextPhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memory::EffectMemory<Self::M>) {
        let anchor = ctx.anchor;
        let level = ctx.level;
        let h = ctx.h;
        let edges = ctx.edges;
        // T-EXPAND (spec § 6.2) at the presence level of the sought key:
        // if the `present` child is not a `val(k)` node (collapsed level),
        // we re-explicit the value level and graft `Some(v)` on each
        // continuation, for each value `v` of the domain.
        if *h == Mode::Cherche && level == self.level_presence() {
            let child_present = edges.get(&PRESENT).map(|(n, _)| *n);
            if let Some(child) = child_present {
                let child_is_level_value = matches!(
                    pool.resolve(child),
                    Ok(Form::Node { level: l, .. }) if *l == self.level_value()
                );
                if !child_is_level_value {
                    let mut greffon = BTreeMap::new();
                    for v in &self.domain_values.values {
                        let mut memo = BTreeMap::new();
                        let branche = Self::greffer(pool, anchor, child, Some(v.0), &mut memo);
                        greffon.insert(v.0, branche);
                    }
                    let node_value = pool.intern(
                        anchor,
                        Form::Node {
                            level: self.level_value(),
                            edges: greffon,
                        },
                    );
                    let mut edges_output: BTreeMap<u64, NodeId> = edges
                        .into_iter()
                        .filter(|(v, _)| *v != PRESENT)
                        .map(|(v, (n, _))| (v, n))
                        .collect();
                    edges_output.insert(PRESENT, node_value);
                    return (
                        pool.intern(
                            anchor,
                            Form::Node {
                                level,
                                edges: edges_output,
                            },
                        ),
                        crate::memory::EffectMemory::new(),
                    );
                }
            }
        }
        // Standard restructuring: we re-emit the node with the
        // transduced children (the default edge leads to D_δ = ε(None), implicit).
        let edges = edges.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(anchor, Form::Node { level, edges }),
            crate::memory::EffectMemory::new(),
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
            // Cherche (key not yet reached) or Extrait (level val(k)
            // implicit, grafted above): not found ⇒ None.
            Mode::Cherche | Mode::Extrait => {
                let terminal = pool.intern(anchor, Form::Terminal(None));
                (terminal, BTreeMap::new())
            }
        }
    }
}
