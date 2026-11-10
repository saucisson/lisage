//! Pli et semi-anneaux (M1-6, spec § 5, énoncés A4/A5/C7).
//!
//! **Deux algèbres déclarées (A5)** : le semi-anneau de **construction**
//! (le merge des écrits) et le semi-anneau de **requête** (paramètre de la
//! transduction) sont deux déclarations distinctes. La construction est
//! `Bool` idempotent — **le hash-consing est ce choix** : [`union`] est
//! l'union idempotente, elle détruit la multiplicité à l'écriture. Le
//! semi-anneau de requête est le paramètre `S` de [`Pli`] : un pli est
//! exact ssi sa lecture factorise par ce que la construction a préservé ;
//! sur une construction Bool, seul le cardinal est exact (spec § 5.1).
//!
//! **Pli** [`Pli<S>`] : `H` trivial, le terminal produit la base
//! (`poids(⊤) = 1`, `poids(⊥) = 0`), `φ_aggr` la structure du semi-anneau.
//! L'expansion-comptage **(A4)** : entre un nœud de niveau `ℓ` et un fils
//! de niveau `ℓ'`, les niveaux `ℓ+1..ℓ'` ont été collapsés (toutes leurs
//! valeurs mènent à la même continuation) ⇒ le poids de l'arc est
//! multiplié par `∏ |Val(n)|`. L'arc par défaut implicite (R-default)
//! contribue le poids de `D_δ` — sur l'instance de référence `D_δ =
//! terminal(false)`, de poids nul (A6 : une construction levée paierait
//! son poids ici).

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use lisage_algebra::interface::{Anchor, Form, KeyAlgebra, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::schema::{Schema, SchemaKeys};
use crate::transduction::{ContextePhiOut, Transducteur, Transduction};

/// Semi-anneau de requête d'un pli (spec § 5) : `(S, ⊕, ×)` avec zéros et
/// unités. `depuis_cardinalite` traduit `|Val(ℓ)|` dans `S` (A4).
pub trait SemiAnneau: Clone + PartialEq + Eq + Hash + Debug + Send + Sync {
    /// Empreinte du semi-anneau : entre dans le fingerprint de la mémo.
    const FINGERPRINT: u64;

    /// Élément neutre de ⊕ (zéro).
    fn zero() -> Self;
    /// Élément neutre de × (un).
    fn un() -> Self;
    /// Cardinalité d'un niveau, traduite dans le semi-anneau (A4).
    fn depuis_cardinalite(cardinalite: u64) -> Self;
    /// ⊕ (agrégation des fils).
    fn plus(&self, autre: &Self) -> Self;
    /// × (poids des arcs par les sous-totaux).
    fn fois(&self, autre: &Self) -> Self;
}

/// `(ℕ, +, ×)` : le semi-anneau du comptage cardinal (spec § 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Naturel(pub u64);

impl SemiAnneau for Naturel {
    const FINGERPRINT: u64 = 4001;

    fn zero() -> Self {
        Naturel(0)
    }
    fn un() -> Self {
        Naturel(1)
    }
    fn depuis_cardinalite(cardinalite: u64) -> Self {
        Naturel(cardinalite)
    }
    fn plus(&self, autre: &Self) -> Self {
        Naturel(self.0 + autre.0)
    }
    fn fois(&self, autre: &Self) -> Self {
        Naturel(self.0 * autre.0)
    }
}

/// `(Bool, ∨, ∧)` : le semi-anneau des ∃-summaries (`exists`, spec § 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BoolOu(pub bool);

impl SemiAnneau for BoolOu {
    const FINGERPRINT: u64 = 4002;

    fn zero() -> Self {
        BoolOu(false)
    }
    fn un() -> Self {
        BoolOu(true)
    }
    fn depuis_cardinalite(cardinalite: u64) -> Self {
        BoolOu(cardinalite > 0)
    }
    fn plus(&self, autre: &Self) -> Self {
        BoolOu(self.0 || autre.0)
    }
    fn fois(&self, autre: &Self) -> Self {
        BoolOu(self.0 && autre.0)
    }
}

/// Pli (fold) sur un semi-anneau de requête `S` (spec § 5) : `H` trivial,
/// le diagramme de sortie est la copie identité de l'entrée, et l'attribut
/// synthétisé est la valeur du pli. `Pli<Naturel>` = `count`.
pub struct Pli<S: SemiAnneau> {
    keys: SchemaKeys,
    nombre_niveaux: u32,
    _semi_anneau: PhantomData<S>,
}

impl<S: SemiAnneau> Pli<S> {
    /// Pli sur le schéma donné (les cardinalités des niveaux viennent de
    /// l'algèbre de clés du schéma, A4).
    pub fn new(schema: Schema) -> Self {
        let nombre_niveaux = schema.nombre_niveaux();
        Pli {
            keys: SchemaKeys::new(schema),
            nombre_niveaux,
            _semi_anneau: PhantomData,
        }
    }

    /// Poids d'un terminal : `poids(⊤) = 1`, `poids(⊥) = 0` (spec § 5).
    fn poids(&self, payload: bool) -> S {
        if payload { S::un() } else { S::zero() }
    }

    /// (A4) multiplicateur de l'arc `v` du niveau `ℓ` vers un fils de
    /// niveau `niveau_cible` (`None` = terminal) : produit des cardinalités
    /// des niveaux sautés **sur le chemin** de l'arc. Un niveau valeur
    /// sauté a été collapsé avec toutes ses valeurs explicites ⇒ facteur
    /// `|Val(n)|` ; un niveau de présence sauté ne collapse que par son arc
    /// par défaut (facteur 1) et son niveau valeur `n+1` est hors chemin.
    fn multiplicateur(&self, level: Level, arc: u64, niveau_cible: Option<Level>) -> S {
        let borne = match niveau_cible {
            Some(niveau) => niveau.0,
            None => self.nombre_niveaux,
        };
        // Prochain niveau sur le chemin de l'arc : un niveau valeur est
        // toujours suivi ; une présence suivie de l'arc par défaut saute
        // le niveau valeur.
        let mut n = match self.keys.default(level) {
            Some(defaut) if arc == defaut => level.0 + 2,
            _ => level.0 + 1,
        };
        let mut produit = S::un();
        while n < borne {
            if self.keys.default(Level(n)).is_some() {
                // Niveau de présence sauté : collapse par défaut seul.
                n += 2;
            } else {
                produit = produit.fois(&S::depuis_cardinalite(self.keys.cardinality(Level(n))));
                n += 1;
            }
        }
        produit
    }
}

impl<S: SemiAnneau> Transduction for Pli<S> {
    type H = ();
    type S = S;
    type Out = bool;
    type M = ();

    fn fingerprint(&self) -> u64 {
        S::FINGERPRINT
    }

    fn h0(&self) {}

    fn phi_disp(&self, _level: Level, h: &(), _arc: u64) {
        let _ = h;
    }

    fn phi_aggr(&self, level: Level, _h: &(), arcs: &BTreeMap<u64, (Option<Level>, S)>) -> S {
        let mut total = S::zero();
        for (arc, (niveau_cible, s)) in arcs {
            let multiplicateur = self.multiplicateur(level, *arc, *niveau_cible);
            total = total.plus(&multiplicateur.fois(s));
        }
        // Arc par défaut implicite (R-default) : sa continuation est D_δ,
        // de poids `poids(false)` sur l'instance de référence (A6).
        if let Some(defaut) = self.keys.default(level)
            && !arcs.contains_key(&defaut)
        {
            total = total.plus(&self.poids(false));
        }
        total
    }

    fn phi_out<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, crate::memoire::EffetMemoire<Self::M>) {
        // Copie identité : le pli ne transforme pas le diagramme.
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            crate::memoire::EffetMemoire::new(),
        )
    }

    fn psi<P: Pool<Key = u64, Payload = Self::Out>>(
        &self,
        pool: &mut P,
        anchor: Anchor,
        payload: bool,
        _h: &(),
    ) -> (NodeId, S) {
        let terminal = pool.intern(anchor, Form::Terminal(payload));
        let poids = self.poids(payload);
        (terminal, poids)
    }
}

/// Comptage cardinal d'un diagramme (ensemble dénoté) : pli sur `(ℕ, +, ×)`,
/// exact sur construction Bool (A5). Lève le résultat du canal synthétisé.
pub fn compter<PIn, POut>(
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    anchor: Anchor,
    schema: &Schema,
    noeud: NodeId,
) -> Result<u64, CoreError>
where
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let mut runner = Transducteur::new(Pli::<Naturel>::new(schema.clone()));
    let (_, total) = runner.run(pool_entree, pool_sortie, anchor, noeud)?;
    Ok(total.0)
}

/// Union de deux diagrammes à payload `bool` (merge de construction, A5) :
/// l'union idempotente — **le hash-consing est ce choix Bool** : la
/// multiplicité est détruite à l'écriture, seul le cardinal est exact.
/// Style ZDD : le terminal `⊤` est le membre « tous les niveaux par
/// défaut » (porté par l'arc `δ(ℓ)`), et un nœud de niveau supérieur à
/// `ℓ` est la continuation de l'arc par défaut du niveau `ℓ`.
pub fn union<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    a: NodeId,
    b: NodeId,
) -> Result<NodeId, CoreError> {
    if a == b {
        return Ok(a);
    }
    let (forme_a, forme_b) = (pool.resolve(a)?.clone(), pool.resolve(b)?.clone());
    match (forme_a, forme_b) {
        (Form::Terminal(pa), Form::Terminal(pb)) => {
            Ok(pool.intern(anchor, Form::Terminal(pa || pb)))
        }
        (Form::Node { level, arcs }, Form::Terminal(pb)) => {
            if pb {
                let vrai = pool.intern(anchor, Form::Terminal(true));
                union_avec_autre(pool, keys, anchor, level, arcs, vrai)
            } else {
                Ok(a)
            }
        }
        (Form::Terminal(pa), Form::Node { level, arcs }) => {
            if pa {
                let vrai = pool.intern(anchor, Form::Terminal(true));
                union_avec_autre(pool, keys, anchor, level, arcs, vrai)
            } else {
                Ok(b)
            }
        }
        (
            Form::Node {
                level: level_a,
                arcs: arcs_a,
            },
            Form::Node {
                level: level_b,
                arcs: arcs_b,
            },
        ) => {
            if level_a == level_b {
                let mut arcs = arcs_a;
                for (arc, cible_b) in arcs_b {
                    match arcs.remove(&arc) {
                        Some(cible_a) => {
                            let uni = union(pool, keys, anchor, cible_a, cible_b)?;
                            arcs.insert(arc, uni);
                        }
                        None => {
                            arcs.insert(arc, cible_b);
                        }
                    }
                }
                Ok(pool.intern(
                    anchor,
                    Form::Node {
                        level: level_a,
                        arcs,
                    },
                ))
            } else if level_a < level_b {
                // Le nœud b est la continuation de l'arc par défaut de a.
                union_avec_autre(pool, keys, anchor, level_a, arcs_a, b)
            } else {
                union_avec_autre(pool, keys, anchor, level_b, arcs_b, a)
            }
        }
    }
}

/// Union d'un nœud de niveau `level` avec un autre diagramme `autre`
/// (terminal `⊤`, ou nœud plus profond) : le chemin de `autre` passe par
/// l'arc par défaut `δ(level)` si le niveau en a un (les arcs absents
/// valent δ) ; sinon (niveau valeur) il passe par **toutes** les valeurs
/// du niveau, explicites ou non.
fn union_avec_autre<P: Pool<Key = u64, Payload = bool>>(
    pool: &mut P,
    keys: &SchemaKeys,
    anchor: Anchor,
    level: Level,
    mut arcs: BTreeMap<u64, NodeId>,
    autre: NodeId,
) -> Result<NodeId, CoreError> {
    match keys.default(level) {
        Some(defaut) => {
            let cible = match arcs.remove(&defaut) {
                Some(cible) => union(pool, keys, anchor, cible, autre)?,
                None => autre,
            };
            arcs.insert(defaut, cible);
        }
        None => {
            let cardinalite = keys.cardinality(level);
            for v in 0..cardinalite {
                let cible = match arcs.remove(&v) {
                    Some(cible) => union(pool, keys, anchor, cible, autre)?,
                    None => autre,
                };
                arcs.insert(v, cible);
            }
        }
    }
    Ok(pool.intern(anchor, Form::Node { level, arcs }))
}
