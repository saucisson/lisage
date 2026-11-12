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
//!
//! Fragment 3 (paramètre d'ordre, backlog § 9) :
//!
//! - [`Ordre`] : permutation niveau → variable, jouée à la compilation ;
//! - [`compiler_bdd_ordre`] / [`comparer_ordres`] : la même fonction booléenne
//!   compilée sous deux ordres — mêmes chemins (2^n, un par affectation),
//!   nœuds partagés différents : « le diagramme est une fonction du
//!   contrôleur », l'ordre se voit à l'œil nu.
//!
//! Fragment 4 (paramètre d'ancre composée, backlog § 9, fil rouge fenêtre
//! glissante) :
//!
//! - [`AncreFenetre`] : l'ancre composée `{base, horizon}` du protocole
//!   SWP — deux décalages nommés, la glisse est le déplacement de la base,
//!   portée par l'arc d'entrée ;
//! - [`construire_fenetre_ancree`] / [`comparer_ancres_fenetre`] : la
//!   fenêtre lue sous deux ancres — chaque ancre a sa racine, la forme
//!   relative de la fenêtre est partagée (`meme_forme`), seules les
//!   lectures absolues diffèrent : « l'ancre bouge, le pool jamais ».

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Level, NodeId, PolitiqueMemoire, Pool, PoolError,
};
use lisage_algebra::relatif::{AncreComposee, Fragment, GroupeCles};

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

/// Interne la forme relative de la fenêtre (clés `0..taille-1`, section
/// min = 0), partagée par toutes les bases.
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

/// **Paramètre d'ordre** (atelier didactique, backlog § 9 : « jouer les
/// paramètres du contrôleur — ordre, encodage absolu/relatif, royaume,
/// ancre composée ») : permutation niveau → variable. Le niveau ℓ décide
/// la variable `permutation[ℓ]` ; l'ordre naturel est l'identité.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ordre {
    /// `permutation[ℓ]` = variable décidée au niveau ℓ.
    permutation: Vec<u32>,
}

impl Ordre {
    /// Ordre naturel : le niveau ℓ décide la variable ℓ.
    pub fn naturel(nombre_variables: u32) -> Self {
        Ordre {
            permutation: (0..nombre_variables).collect(),
        }
    }

    /// Ordre inversé : le niveau ℓ décide la variable n-1-ℓ.
    pub fn inverse(nombre_variables: u32) -> Self {
        Ordre {
            permutation: (0..nombre_variables).rev().collect(),
        }
    }

    /// Ordre personnalisé : `permutation[ℓ]` = variable du niveau ℓ. Doit
    /// être une permutation de `0..n` (sinon deux niveaux décideraient la
    /// même variable et une variable serait sautée).
    pub fn personnalise(permutation: Vec<u32>) -> Self {
        let mut triee = permutation.clone();
        triee.sort_unstable();
        assert_eq!(
            triee,
            (0..permutation.len() as u32).collect::<Vec<_>>(),
            "l'ordre est une permutation de 0..n (invariant interne)"
        );
        Ordre { permutation }
    }

    /// Nombre de variables.
    pub fn nombre_variables(&self) -> u32 {
        self.permutation.len() as u32
    }

    /// Variable décidée au niveau `niveau`.
    pub fn variable_au_niveau(&self, niveau: u32) -> u32 {
        self.permutation[niveau as usize]
    }

    /// Étiquette didactique `[v₀, v₁, …]` pour la narration.
    pub fn etiquette(&self) -> String {
        let variables: Vec<String> = self.permutation.iter().map(u32::to_string).collect();
        format!("[{}]", variables.join(","))
    }
}

/// Compile une table de vérité en BDD classique sous l'`ordre` donné :
/// expansion de Shannon descendante, le niveau ℓ décide la variable
/// `ordre.variable_au_niveau(ℓ)`. `table[a]` = valeur de la fonction sur
/// l'affectation `a` (bit ℓ de `a` = variable ℓ) ; la longueur doit être
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
        "table de vérité de taille 2^n (invariant interne)"
    );
    // Permute la table : bit ℓ de l'indice local = bit `ordre[ℓ]` de
    // l'indice original, puis l'expansion de Shannon classique s'applique
    // (au niveau ℓ on branche sur le bit ℓ de l'indice local).
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

/// Expansion de Shannon descendante : `table` est indexée par les bits des
/// variables restantes (bit ℓ de l'indice = variable du niveau ℓ) ; les
/// positions paires (bit du niveau courant = 0) donnent l'arc 0, les
/// impaires l'arc 1.
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

/// Comparaison didactique de deux ordres sur la même fonction booléenne.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportComparaisonOrdres {
    /// Métriques sous le premier ordre.
    pub premier: RapportAtelier,
    /// Métriques sous le second ordre.
    pub second: RapportAtelier,
}

/// Compile la même table de vérité sous deux ordres et mesure : mêmes
/// chemins (2^n, un par affectation — même objet logique), nœuds partagés
/// différents — l'ordre change le diagramme, pas la fonction.
/// `observateur` : narration ADR-0100.
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
        "les deux ordres portent sur le même nombre de variables (invariant interne)"
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

/// Fragment nommé « base » de l'ancre composée de la fenêtre glissante
/// (position de séquence, fil rouge SWP).
pub fn fragment_base() -> Fragment {
    Fragment("base".to_string())
}

/// Fragment nommé « horizon » de l'ancre composée de la fenêtre glissante
/// (taille de la fenêtre, fil rouge SWP).
pub fn fragment_horizon() -> Fragment {
    Fragment("horizon".to_string())
}

/// **Paramètre d'ancre composée** (atelier didactique, backlog § 9 :
/// « jouer les paramètres du contrôleur — ordre, encodage absolu/relatif,
/// royaume, ancre composée » ; fil rouge fenêtre glissante § 4-5) :
/// l'ancre de la fenêtre est un enregistrement nommé de deux décalages,
/// `{base, horizon}`. La **glisse** est le déplacement de la base ; l'ancre
/// reste dans le handle (C1 inchangé), le pool reste aveugle.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AncreFenetre {
    inner: AncreComposee,
}

impl AncreFenetre {
    /// Ancre `{base, horizon}` : les deux décalages nommés du protocole SWP.
    pub fn nouvelle(base: i64, horizon: i64) -> Self {
        AncreFenetre {
            inner: AncreComposee::nouveau()
                .avec(fragment_base(), base)
                .avec(fragment_horizon(), horizon),
        }
    }

    /// Décalage de la base (position de séquence).
    pub fn base(&self) -> i64 {
        self.inner.decalage(&fragment_base())
    }

    /// Décalage de l'horizon (taille de la fenêtre).
    pub fn horizon(&self) -> i64 {
        self.inner.decalage(&fragment_horizon())
    }

    /// Nouvelle ancre, base déplacée de `delta` : la glisse de la fenêtre.
    pub fn glessee(&self, delta: i64) -> Self {
        AncreFenetre {
            inner: self.inner.decalee(&fragment_base(), delta),
        }
    }

    /// Lecture absolue de la fenêtre : l'intervalle `[base, base + horizon)`.
    pub fn fenetre_absolue(&self) -> (i64, i64) {
        let base = self.base();
        (base, base + self.horizon())
    }

    /// Clé de l'arc d'entrée : la base de l'ancre, saturée dans `u64`.
    /// C'est l'argument de position (spec § 2.4.5 : l'arc entrant porte
    /// l'argument) — il vit sur l'arc, jamais dans la clé d'interning.
    pub fn cle_entree(&self) -> u64 {
        self.base().max(0) as u64
    }

    /// Étiquette didactique `{base: b, horizon: k}` pour la narration.
    pub fn etiquette(&self) -> String {
        format!("{{base: {}, horizon: {}}}", self.base(), self.horizon())
    }
}

/// Construit la fenêtre glissante lue sous l'`ancre` composée : la base de
/// l'ancre est l'argument de l'arc d'entrée (clé `cle_entree`), la forme de
/// la fenêtre reste relative (clés `0..taille-1`, section min = 0) et
/// partagée. Retourne `(racine, fenêtre)` : la racine propre à cette base,
/// la forme de fenêtre atteinte par son arc d'entrée.
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

/// Comparaison didactique de deux ancres composées sur la même fenêtre.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RapportComparaisonAncres {
    /// Métriques de la fenêtre lue sous la première ancre.
    pub avant: RapportAtelier,
    /// Métriques de la fenêtre lue sous la seconde ancre.
    pub apres: RapportAtelier,
    /// Vrai si les deux lectures atteignent la même forme de fenêtre
    /// (`NodeId`) : déplacer la base ne duplique pas la forme.
    pub meme_forme: bool,
    /// Compte de nœuds du pool après les deux lectures (une racine par
    /// ancre, une seule forme de fenêtre).
    pub noeuds_pool: u64,
    /// Lecture absolue `[base, base + horizon)` sous la première ancre.
    pub lecture_avant: (i64, i64),
    /// Lecture absolue `[base, base + horizon)` sous la seconde ancre.
    pub lecture_apres: (i64, i64),
}

/// Compare la fenêtre glissante lue sous deux ancres composées : chaque
/// ancre construit sa propre racine (l'arc d'entrée porte sa base), mais la
/// forme relative de la fenêtre (clés `0..taille-1`, section min = 0) est
/// partagée — `meme_forme` vrai, mêmes nœuds, mêmes chemins — seules les
/// lectures absolues diffèrent. C'est la glisse du protocole SWP :
/// « l'ancre bouge, le pool jamais » : déplacer la base ne duplique pas la
/// forme de la fenêtre. `observateur` : narration ADR-0100.
pub fn comparer_ancres_fenetre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    anchor: Anchor,
    taille: u64,
    ancre_avant: &AncreFenetre,
    ancre_apres: &AncreFenetre,
    observateur: Option<&dyn Observer>,
) -> Result<RapportComparaisonAncres, PoolError> {
    // Chaque ancre entre dans le pool par son arc d'entrée (sa base) ; la
    // forme de fenêtre atteinte des deux côtés doit être le même nœud.
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
