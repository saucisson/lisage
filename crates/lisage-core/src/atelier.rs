//! **Didactic workshop** — reduction dashboard (backlog § 9 of the
//! roadmap, request of November 10, 2026): making the thesis "the diagram
//! is a function of the controller" tangible by enabling/disabling each
//! reduction one at a time and measuring.
//!
//! First fragment (backend, no web front):
//!
//! - [`Reductions`]: R-collapse / R-default switches;
//! - [`AtelierController`]: wrapping controller that toggles them (the pool
//!   reads the `collapse_actif` / `defaut_actif` hooks at interning time);
//! - [`mesurer`]: three didactic metrics — the **shared nodes**
//!   (R-share active, count of reachable handles), the **paths**
//!   (number of root → terminal paths) and the **unfolded size** (number
//!   of nodes in the unfolded tree, each arc duplicating the target subtree:
//!   what the absence of hash-consing would cost). Thread example: R₃ fits in
//!   33 shared nodes versus an unfolded size of 126, the 20 paths
//!   being one per reachable state.
//!
//! R-share itself is not switchable (it is the pool's single table):
//! its deactivation is **measured** by the unfolded size, not by a
//! degraded pool.
//!
//! Fragment 3 (ordering parameter, backlog § 9):
//!
//! - [`Ordre`]: level → variable permutation, applied at compile time;
//! - [`compiler_bdd_ordre`] / [`comparer_ordres`]: the same boolean function
//!   compiled under two orderings — same paths (2^n, one per assignment),
//!   different shared nodes: "the diagram is a function of the
//!   controller", the ordering is visible to the naked eye.
//!
//! Fragment 4 (composite anchor parameter, backlog § 9, sliding-window
//! thread):
//!
//! - [`AncreFenetre`]: the composite anchor `{base, horizon}` of the
//!   SWP protocol — two named offsets, the slide is the movement of the base,
//!   carried by the incoming arc;
//! - [`construire_fenetre_ancree`] / [`comparer_ancres_fenetre`]: the
//!   window read under two anchors — each anchor has its own root, the
//!   relative shape of the window is shared (`meme_forme`), only the
//!   absolute readings differ: "the anchor moves, the pool never does".

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Level, NodeId, PolitiqueMemoire, Pool, PoolError,
};
use lisage_algebra::relatif::{AncreComposee, Fragment, GroupeCles};

use crate::observer::{CanalAttr, Observer, TraceEvent};

/// Controller-side reduction switches (R-share excluded: measured).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reductions {
    /// R-collapse active.
    pub collapse: bool,
    /// R-default active (absorption of the δ(ℓ) arc towards D_δ).
    pub defaut: bool,
}

impl Reductions {
    /// All reductions enabled (canonical configuration).
    pub const TOUTES: Reductions = Reductions {
        collapse: true,
        defaut: true,
    };
    /// No controller-side reduction (R-share always active, measured).
    pub const AUCUNE: Reductions = Reductions {
        collapse: false,
        defaut: false,
    };
    /// R-collapse toggled off, R-default active.
    pub fn sans_collapse() -> Self {
        Reductions {
            collapse: false,
            defaut: true,
        }
    }
    /// R-default toggled off, R-collapse active.
    pub fn sans_defaut() -> Self {
        Reductions {
            collapse: true,
            defaut: false,
        }
    }
}

/// Wrapping controller that toggles R-collapse and R-default according to
/// [`Reductions`]; everything else is delegated to the inner controller.
pub struct AtelierController<C: Controller> {
    inner: C,
    reductions: Reductions,
}

impl<C: Controller> AtelierController<C> {
    /// Wraps `inner` with the `reductions` switches.
    pub fn nouveau(inner: C, reductions: Reductions) -> Self {
        AtelierController { inner, reductions }
    }

    /// The active switches.
    pub fn reductions(&self) -> Reductions {
        self.reductions
    }
}

impl<C: Controller> Controller for AtelierController<C> {
    type Key = C::Key;
    type Payload = C::Payload;
    type Keys = C::Keys;

    // Fingerprint specific to the wrapper (outside the native controllers 1-6 range).
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

/// Didactic measurement report of a diagram.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportAtelier {
    /// Distinct reachable nodes (R-share active): the shared diagram.
    pub noeuds_partages: u64,
    /// Root → terminal paths in the DAG.
    pub chemins: u64,
    /// Nodes of the unfolded tree (each arc duplicating the target subtree):
    /// the cost of the absence of hash-consing.
    pub taille_depliee: u64,
}

/// Measures a diagram: shared nodes, paths and unfolded size. Propagates
/// the pool error (an unknown `NodeId` is an error, not a silent value).
/// `observateur`: narration channel (ADR-0100) — an `Attr`/`Synthétise`
/// event carries the full report.
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

/// Counts the distinct nodes reachable from `racine`, terminals included.
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

/// Counts the root → terminal paths of the DAG (memoized per node, saturating).
/// This is not the unfolded size: that one is the sum of path lengths,
/// computed by [`compter_taille_depliee`].
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

/// Number of nodes in the unfolded tree (memoized per node, saturating): each
/// arc duplicates the subtree of its target, what the absence of
/// R-share would cost.
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

/// Level of the sliding-window root.
const NIVEAU_RACINE: u32 = 0;
/// Level of the window node of the sliding window.
const NIVEAU_FENETRE: u32 = 1;

/// **Encoding parameter** (didactic workshop, backlog § 9: "playing with the
/// controller parameters — ordering, absolute/relative encoding, realm,
/// composite anchor").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encodage {
    /// Absolute keys: each window has its own keys, one node per base.
    Absolu,
    /// Relative keys (min section = 0, M4-2): a single shared shape,
    /// the position argument lives on the incoming arc.
    Relatif,
}

/// Didactic comparison of the two encodings on the same logical object.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportComparaison {
    /// Metrics of the absolute encoding.
    pub absolu: RapportAtelier,
    /// Metrics of the relative encoding.
    pub relatif: RapportAtelier,
}

/// Builds the **sliding window** in absolute encoding: for each base
/// `b ∈ [0, nb_bases)`, its own window `{b..b+taille-1}` (one node per
/// base), all linked to the root. The number of nodes grows with
/// `nb_bases` — this is the "infinity of the window" of the flat prototype.
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

/// Interns the relative shape of the window (keys `0..taille-1`, min
/// section = 0), shared by all bases.
fn interner_fenetre_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
) -> NodeId {
    let vrai = pool.intern(anchor, Form::Terminal(true));
    pool.intern(
        anchor,
        Form::Node {
            level: Level(NIVEAU_FENETRE),
            arcs: (0..taille).map(|k| (k, vrai)).collect(),
        },
    )
}

/// Builds the same sliding window in **relative** encoding (M4-2,
/// min section = 0): a single shape `{0..taille-1}` shared by all the
/// bases (the position argument lives on the incoming arc). The number of
/// nodes is constant — this is the "4 against infinity" of the window.
pub fn construire_fenetre_relative<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    nb_bases: u64,
) -> NodeId {
    let fenetre = interner_fenetre_relative(pool, anchor, taille);
    let arcs = (0..nb_bases).map(|base| (base, fenetre)).collect();
    pool.intern(
        anchor,
        Form::Node {
            level: Level(NIVEAU_RACINE),
            arcs,
        },
    )
}

/// Builds the sliding window in the requested `encodage`: single entry
/// point of the encoding parameter.
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

/// Compares the two encodings of the sliding window over `nb_bases` bases:
/// same paths (same logical object), different shared nodes — the
/// relative sharing is visible to the naked eye. `observateur`: ADR-0100 narration.
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

/// **Ordering parameter** (didactic workshop, backlog § 9: "playing with the
/// controller parameters — ordering, absolute/relative encoding, realm,
/// composite anchor"): level → variable permutation. Level ℓ decides
/// variable `permutation[ℓ]`; the natural ordering is the identity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ordre {
    /// `permutation[ℓ]` = variable decided at level ℓ.
    permutation: Vec<u32>,
}

impl Ordre {
    /// Natural ordering: level ℓ decides variable ℓ.
    pub fn naturel(nombre_variables: u32) -> Self {
        Ordre {
            permutation: (0..nombre_variables).collect(),
        }
    }

    /// Reversed ordering: level ℓ decides variable n-1-ℓ.
    pub fn inverse(nombre_variables: u32) -> Self {
        Ordre {
            permutation: (0..nombre_variables).rev().collect(),
        }
    }

    /// Custom ordering: `permutation[ℓ]` = variable of level ℓ. Must
    /// be a permutation of `0..n` (otherwise two levels would decide the
    /// same variable and one variable would be skipped).
    pub fn personnalise(permutation: Vec<u32>) -> Self {
        let mut triee = permutation.clone();
        triee.sort_unstable();
        assert_eq!(
            triee,
            (0..permutation.len() as u32).collect::<Vec<_>>(),
            "the ordering is a permutation of 0..n (internal invariant)"
        );
        Ordre { permutation }
    }

    /// Number of variables.
    pub fn nombre_variables(&self) -> u32 {
        self.permutation.len() as u32
    }

    /// Variable decided at level `niveau`.
    pub fn variable_au_niveau(&self, niveau: u32) -> u32 {
        self.permutation[niveau as usize]
    }

    /// Didactic label `[v₀, v₁, …]` for narration.
    pub fn etiquette(&self) -> String {
        let variables: Vec<String> = self.permutation.iter().map(u32::to_string).collect();
        format!("[{}]", variables.join(","))
    }
}

/// Compiles a truth table into a classic BDD under the given `ordre`:
/// top-down Shannon expansion, level ℓ decides variable
/// `ordre.variable_au_niveau(ℓ)`. `table[a]` = value of the function on
/// assignment `a` (bit ℓ of `a` = variable ℓ); the length must be
/// `2^n`.
pub fn compiler_bdd_ordre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    ordre: &Ordre,
    table: &[bool],
) -> NodeId {
    let nombre_variables = ordre.nombre_variables();
    assert_eq!(
        table.len(),
        1usize << nombre_variables,
        "truth table of size 2^n (internal invariant)"
    );
    // Permute the table: bit ℓ of the local index = bit `ordre[ℓ]` of
    // the original index, then the classic Shannon expansion applies
    // (at level ℓ we branch on bit ℓ of the local index).
    let table_locale: Vec<bool> = (0..table.len())
        .map(|indice| {
            let mut original = 0usize;
            for niveau in 0..nombre_variables {
                original |= ((indice >> niveau) & 1) << ordre.variable_au_niveau(niveau) as usize;
            }
            table[original]
        })
        .collect();
    shannon(pool, anchor, Level(NIVEAU_RACINE), &table_locale)
}

/// Top-down Shannon expansion: `table` is indexed by the bits of the
/// remaining variables (bit ℓ of the index = variable of level ℓ); the
/// even positions (bit of the current level = 0) give arc 0, the
/// odd ones arc 1.
fn shannon<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    level: Level,
    table: &[bool],
) -> NodeId {
    if table.len() == 1 {
        return pool.intern(anchor, Form::Terminal(table[0]));
    }
    let bas_table: Vec<bool> = table.iter().step_by(2).copied().collect();
    let haut_table: Vec<bool> = table.iter().skip(1).step_by(2).copied().collect();
    let bas = shannon(pool, anchor, Level(level.0 + 1), &bas_table);
    let haut = shannon(pool, anchor, Level(level.0 + 1), &haut_table);
    pool.intern(
        anchor,
        Form::Node {
            level,
            arcs: BTreeMap::from([(0, bas), (1, haut)]),
        },
    )
}

/// Didactic comparison of two orderings on the same boolean function.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportComparaisonOrdres {
    /// Metrics under the first ordering.
    pub premier: RapportAtelier,
    /// Metrics under the second ordering.
    pub second: RapportAtelier,
}

/// Compiles the same truth table under two orderings and measures: same
/// paths (2^n, one per assignment — same logical object), different shared
/// nodes — the ordering changes the diagram, not the function.
/// `observateur`: ADR-0100 narration.
pub fn comparer_ordres<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    table: &[bool],
    ordre_premier: &Ordre,
    ordre_second: &Ordre,
    observateur: Option<&dyn Observer>,
) -> Result<RapportComparaisonOrdres, PoolError> {
    assert_eq!(
        ordre_premier.nombre_variables(),
        ordre_second.nombre_variables(),
        "both orderings cover the same number of variables (internal invariant)"
    );
    let racine_premier = compiler_bdd_ordre(pool, anchor, ordre_premier, table);
    let racine_second = compiler_bdd_ordre(pool, anchor, ordre_second, table);
    let premier = mesurer(pool, racine_premier, observateur)?;
    let second = mesurer(pool, racine_second, observateur)?;
    if let Some(obs) = observateur {
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine_second,
            valeur: format!(
                "ordre={}_vs_{} noeuds={} contre {}",
                ordre_premier.etiquette(),
                ordre_second.etiquette(),
                premier.noeuds_partages,
                second.noeuds_partages
            ),
        });
    }
    Ok(RapportComparaisonOrdres { premier, second })
}

/// Fragment named "base" of the composite anchor of the sliding window
/// (sequence position, SWP thread).
pub fn fragment_base() -> Fragment {
    Fragment("base".to_string())
}

/// Fragment named "horizon" of the composite anchor of the sliding window
/// (window size, SWP thread).
pub fn fragment_horizon() -> Fragment {
    Fragment("horizon".to_string())
}

/// **Composite anchor parameter** (didactic workshop, backlog § 9:
/// "playing with the controller parameters — ordering, absolute/relative
/// encoding, realm, composite anchor"; sliding-window thread § 4-5):
/// the window's anchor is a named record of two offsets,
/// `{base, horizon}`. The **slide** is the movement of the base; the anchor
/// stays in the handle (C1 unchanged), the pool stays blind.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AncreFenetre {
    inner: AncreComposee,
}

impl AncreFenetre {
    /// Anchor `{base, horizon}`: the two named offsets of the SWP protocol.
    pub fn nouvelle(base: i64, horizon: i64) -> Self {
        AncreFenetre {
            inner: AncreComposee::nouveau()
                .avec(fragment_base(), base)
                .avec(fragment_horizon(), horizon),
        }
    }

    /// Offset of the base (sequence position).
    pub fn base(&self) -> i64 {
        self.inner.decalage(&fragment_base())
    }

    /// Offset of the horizon (window size).
    pub fn horizon(&self) -> i64 {
        self.inner.decalage(&fragment_horizon())
    }

    /// New anchor, base shifted by `delta`: the slide of the window.
    pub fn glessee(&self, delta: i64) -> Self {
        AncreFenetre {
            inner: self.inner.decalee(&fragment_base(), delta),
        }
    }

    /// Absolute reading of the window: the interval `[base, base + horizon)`.
    pub fn fenetre_absolue(&self) -> (i64, i64) {
        let base = self.base();
        (base, base + self.horizon())
    }

    /// Key of the incoming arc: the base of the anchor, saturated into `u64`.
    /// This is the position argument (spec § 2.4.5: the incoming arc carries
    /// the argument) — it lives on the arc, never in the interning key.
    pub fn cle_entree(&self) -> u64 {
        self.base().max(0) as u64
    }

    /// Didactic label `{base: b, horizon: k}` for narration.
    pub fn etiquette(&self) -> String {
        format!("{{base: {}, horizon: {}}}", self.base(), self.horizon())
    }
}

/// Builds the sliding window read under the composite `ancre`: the base of
/// the anchor is the argument of the incoming arc (key `cle_entree`), the
/// shape of the window stays relative (keys `0..taille-1`, min section = 0)
/// and shared. Returns `(racine, window)`: the root specific to this base,
/// the window shape reached by its incoming arc.
pub fn construire_fenetre_ancree<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    ancre: &AncreFenetre,
) -> (NodeId, NodeId) {
    let fenetre = interner_fenetre_relative(pool, anchor, taille);
    let racine = pool.intern(
        anchor,
        Form::Node {
            level: Level(NIVEAU_RACINE),
            arcs: BTreeMap::from([(ancre.cle_entree(), fenetre)]),
        },
    );
    (racine, fenetre)
}

/// Didactic comparison of two composite anchors on the same window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportComparaisonAncres {
    /// Metrics of the window read under the first anchor.
    pub avant: RapportAtelier,
    /// Metrics of the window read under the second anchor.
    pub apres: RapportAtelier,
    /// True if both readings reach the same window shape
    /// (`NodeId`): moving the base does not duplicate the shape.
    pub meme_forme: bool,
    /// Node count of the pool after both readings (one root per
    /// anchor, a single window shape).
    pub noeuds_pool: u64,
    /// Absolute reading `[base, base + horizon)` under the first anchor.
    pub lecture_avant: (i64, i64),
    /// Absolute reading `[base, base + horizon)` under the second anchor.
    pub lecture_apres: (i64, i64),
}

/// Compares the sliding window read under two composite anchors: each
/// anchor builds its own root (the incoming arc carries its base), but the
/// relative shape of the window (keys `0..taille-1`, min section = 0) is
/// shared — `meme_forme` true, same nodes, same paths — only the
/// absolute readings differ. This is the slide of the SWP protocol:
/// "the anchor moves, the pool never does": moving the base does not
/// duplicate the window shape. `observateur`: ADR-0100 narration.
pub fn comparer_ancres_fenetre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    ancre_avant: &AncreFenetre,
    ancre_apres: &AncreFenetre,
    observateur: Option<&dyn Observer>,
) -> Result<RapportComparaisonAncres, PoolError> {
    // Each anchor enters the pool through its incoming arc (its base); the
    // window shape reached on both sides must be the same node.
    let (racine_avant, fenetre_avant) =
        construire_fenetre_ancree(pool, anchor, taille, ancre_avant);
    let (racine_apres, fenetre_apres) =
        construire_fenetre_ancree(pool, anchor, taille, ancre_apres);
    let noeuds_pool = pool.node_count() as u64;
    let meme_forme = fenetre_avant == fenetre_apres;
    let avant = mesurer(pool, racine_avant, observateur)?;
    let apres = mesurer(pool, racine_apres, observateur)?;
    if let Some(obs) = observateur {
        let (debut_avant, fin_avant) = ancre_avant.fenetre_absolue();
        let (debut_apres, fin_apres) = ancre_apres.fenetre_absolue();
        obs.on_event(&TraceEvent::Attr {
            canal: CanalAttr::Synthétise,
            noeud: racine_apres,
            valeur: format!(
                "ancre={}_vs_{} noeuds={} contre {} lecture=[{}..{}) contre [{}..{}) pool={}",
                ancre_avant.etiquette(),
                ancre_apres.etiquette(),
                avant.noeuds_partages,
                apres.noeuds_partages,
                debut_avant,
                fin_avant,
                debut_apres,
                fin_apres,
                noeuds_pool,
            ),
        });
    }
    Ok(RapportComparaisonAncres {
        avant,
        apres,
        meme_forme,
        noeuds_pool,
        lecture_avant: ancre_avant.fenetre_absolue(),
        lecture_apres: ancre_apres.fenetre_absolue(),
    })
}
