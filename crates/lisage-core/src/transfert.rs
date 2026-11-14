//! Non-local events (M2-5, spec § 7, statement T3): compiling a transition of
//! width `w` into a transduction with **level memory**, with
//! a polynomial overhead in `w`.
//!
//! Reference example (spec § 7): the transfer `t : m(a) -= 1; m(b) += 1`
//! on a `Map` — the transition touches the levels of the two keys `a` and
//! `b`, non-adjacent. The semantics is non-local (read `a`, write `b`),
//! but each *segment* is local: the inherited attribute `H` carries the
//! transfer token from `val(a)` to `val(b)` through the
//! intermediate levels, and the level memory `M` records at the emitting node
//! that the decrement has already been produced (idempotence by `(node, level)`).
//!
//! **Per-arc semantics** (spec § 7, `φ_disp(val(a), h, v) = v ≥ 1 ?
//! h_transfert : stop`): the source is decremented as soon as `m(a) ≥ 1`;
//! the destination receives `m(b) + 1` if this value is in the domain,
//! otherwise its arc is kept (the source decrement still applies —
//! case pinned by `test_transfert_destination_hors_domaine`).
//!
//! **Poly(w) overhead (T3)**: the transduction visits each node once
//! (memo L1) and does O(1) work per node; `H` is an O(1) state per
//! level and `M` stores at most one entry per touched `(node, level)`,
//! i.e. O(w) entries per node. The compilation does not reify any
//! intermediate variable (unlike Zhao/Ciardo): `M` and `H` stitch the
//! segments.
//!
//! **Scope**: the destination must exist in the diagram (key
//! present, or presence node kept by a later key) — the
//! "absent and collapsed destination" case falls under axis
//! explicitation (T-EXPAND, § lookup), outside this milestone.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::memoire::{EffetMemoire, MemoireNiveau};
use crate::pli::union;
use crate::schema::{ABSENT, Domaine, PRESENT, Schema, SchemaKeys};
use crate::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::Atom;

/// Base fingerprint of the transfer transduction.
const FINGERPRINT_TRANSFERT: u64 = 5001;
/// Number of levels touched by a transfer between two keys: presence +
/// value of the source, presence + value of the destination.
pub const LARGEUR_TRANSFERT: u32 = 4;

/// Inherited attribute of the transfer: state of the token between levels.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum EtatTransfert {
    /// Normal descent: no transfer in progress.
    Normal,
    /// Transfer in progress: amount to add to the value of the
    /// destination key (1 unit transferred).
    Transfert(u64),
}

/// Non-local transition `m(a) -= 1; m(b) += 1` compiled into a level-memory
/// transduction (T3). Requires `index_source < index_destination`: the
/// inherited token descends from `val(a)` to `val(b)`.
#[derive(Clone)]
pub struct TransfertNonLocal {
    index_source: usize,
    index_destination: usize,
    domaine: Domaine,
    keys: SchemaKeys,
}

impl TransfertNonLocal {
    /// Transfer from key `index_source` to key `index_destination`
    /// (in the key order of the `Map`) of the given schema.
    ///
    /// # Panics
    /// If the schema is not a `Map`, or if `index_source >=
    /// index_destination` (the inherited token does not go upward).
    pub fn new(schema: &Schema, index_source: usize, index_destination: usize) -> Self {
        assert!(
            index_source < index_destination,
            "transfer requires source < destination (downward inherited token)"
        );
        let Schema::Map(_, domaine) = schema else {
            panic!("TransfertNonLocal requires a Map schema");
        };
        TransfertNonLocal {
            index_source,
            index_destination,
            domaine: domaine.clone(),
            keys: SchemaKeys::new(schema.clone()),
        }
    }

    /// Width `w` of the transition: touched levels (T3, poly(w) overhead).
    pub fn largeur(&self) -> u32 {
        LARGEUR_TRANSFERT
    }

    /// Value level of the source key.
    fn niveau_source(&self) -> Level {
        Schema::niveau_valeur(self.index_source)
    }

    /// Value level of the destination key.
    fn niveau_destination(&self) -> Level {
        Schema::niveau_valeur(self.index_destination)
    }

    /// Inserts the remapped arc `cle ↦ (n, s)` into `remap`; on key collision
    /// (family: two arcs remapped to the same target), the two
    /// subtrees are **unioned**, never overwritten.
    fn inserer_remap<P: Pool<Key = u64, Payload = bool>>(
        pool: &mut P,
        keys: &SchemaKeys,
        anchor: Anchor,
        remap: &mut BTreeMap<u64, (NodeId, ())>,
        cle: u64,
        n: NodeId,
        s: (),
    ) {
        match remap.remove(&cle) {
            Some((ancien, _)) => {
                let fusion = union(pool, keys, anchor, ancien, n)
                    .expect("union of sub-arcs of the same pool");
                remap.insert(cle, (fusion, s));
            }
            None => {
                remap.insert(cle, (n, s));
            }
        }
    }

    /// Decrement of the source (level `val(a)`): remap `v → v-1` if
    /// `v ≥ 1` (spec § 7: `φ_disp(val(a), h, v) = v ≥ 1 ? h_transfert : stop`),
    /// and memory effect `M[id(n), val(a)] ← true` if a decrement was
    /// produced (idempotence by `(node, level)`).
    fn decremente_source<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        memoire: &MemoireNiveau<bool>,
        noeud_entree: NodeId,
        level: Level,
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> (BTreeMap<u64, (NodeId, ())>, EffetMemoire<bool>) {
        // Already produced by this node at this visit? (idempotence M).
        if memoire.lire(noeud_entree, level) == Some(&true) {
            return (arcs, EffetMemoire::new());
        }
        let mut remap = BTreeMap::new();
        let mut decremente = false;
        for (v, (n, s)) in arcs {
            if v >= 1 {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, v - 1, n, s);
                decremente = true;
            } else {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, v, n, s);
            }
        }
        let effet = if decremente {
            EffetMemoire::une(noeud_entree, level, true)
        } else {
            EffetMemoire::new()
        };
        (remap, effet)
    }

    /// Placement of the **absent** destination (level `k_b?`): the `ABSENT`
    /// arc becomes `PRESENT` towards a new value node carrying the
    /// transferred amount (spec § 7: `φ_disp(k_b?, h_transfert(w), ·) = h_pose(w+1)`).
    fn pose_presence_destination<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        h: &EtatTransfert,
        mut arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> BTreeMap<u64, (NodeId, ())> {
        let EtatTransfert::Transfert(montant) = h else {
            return arcs;
        };
        let montant = *montant;
        if self.domaine.contient(Atom(montant))
            && let Some((suite, _)) = arcs.remove(&ABSENT)
        {
            let noeud_valeur = pool.intern(
                anchor,
                Form::Node {
                    level: self.niveau_destination(),
                    arcs: BTreeMap::from([(montant, suite)]),
                },
            );
            // Collision with an existing PRESENT arc (family): union.
            match arcs.remove(&PRESENT) {
                Some((ancien, _)) => {
                    let fusion = union(pool, &self.keys, anchor, ancien, noeud_valeur)
                        .expect("union of sub-arcs of the same pool");
                    arcs.insert(PRESENT, (fusion, ()));
                }
                None => {
                    arcs.insert(PRESENT, (noeud_valeur, ()));
                }
            }
        }
        arcs
    }

    /// Placement on the value of the destination (level `val(b)`): remap
    /// `u → u + amount` if the target is in the domain; otherwise the arc is
    /// kept as is (**per-arc** semantics: the source decrement
    /// still applies, pinned by `test_transfert_destination_hors_domaine`).
    fn pose_valeur_destination<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        h: &EtatTransfert,
        arcs: BTreeMap<u64, (NodeId, ())>,
    ) -> BTreeMap<u64, (NodeId, ())> {
        let EtatTransfert::Transfert(montant) = h else {
            return arcs;
        };
        let montant = *montant;
        let mut remap = BTreeMap::new();
        for (u, (n, s)) in arcs {
            let cible = u + montant;
            if self.domaine.contient(Atom(cible)) {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, cible, n, s);
            } else {
                Self::inserer_remap(pool, &self.keys, anchor, &mut remap, u, n, s);
            }
        }
        remap
    }
}

impl Transduction for TransfertNonLocal {
    type H = EtatTransfert;
    type S = ();
    type Out = bool;
    type M = bool;

    fn fingerprint(&self) -> u64 {
        FINGERPRINT_TRANSFERT ^ (self.index_source as u64) << 32 ^ self.index_destination as u64
    }

    fn h0(&self) -> EtatTransfert {
        EtatTransfert::Normal
    }

    fn phi_disp(&self, level: Level, h: &EtatTransfert, arc: u64) -> EtatTransfert {
        if level == self.niveau_source() {
            // φ_disp(val(a), h, v) = v ≥ 1 ? h_transfert(1) : stop:
            // the token is born at the value of a (1 unit to transfer).
            if arc >= 1 {
                EtatTransfert::Transfert(1)
            } else {
                EtatTransfert::Normal
            }
        } else {
            // The token crosses the intermediate levels unchanged.
            h.clone()
        }
    }

    fn phi_aggr(
        &self,
        _level: Level,
        _h: &EtatTransfert,
        _arcs: &BTreeMap<u64, (Option<Level>, ())>,
    ) {
    }

    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffetMemoire<bool>) {
        let ContextePhiOut {
            anchor,
            level,
            h,
            arcs,
            memoire,
            noeud_entree,
        } = ctx;
        let (arcs, effet) = if level == self.niveau_source() {
            self.decremente_source(pool, anchor, memoire, noeud_entree, level, arcs)
        } else if level == Schema::niveau_presence(self.index_destination)
            && matches!(h, EtatTransfert::Transfert(_))
        {
            (
                self.pose_presence_destination(pool, anchor, h, arcs),
                EffetMemoire::new(),
            )
        } else if level == self.niveau_destination() && matches!(h, EtatTransfert::Transfert(_)) {
            (
                self.pose_valeur_destination(pool, anchor, h, arcs),
                EffetMemoire::new(),
            )
        } else {
            (arcs, EffetMemoire::new())
        };
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (pool.intern(anchor, Form::Node { level, arcs }), effet)
    }

    fn psi<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &EtatTransfert,
    ) -> (NodeId, ()) {
        (pool.intern(anchor, Form::Terminal(payload)), ())
    }
}
