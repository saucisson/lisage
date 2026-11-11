//! Événements non locaux (M2-5, spec § 7, énoncé T3) : compilation d'une
//! transition de largeur `w` en transduction à **mémoire de niveau**, avec
//! un surcoût polynomial en `w`.
//!
//! Exemple de référence (spec § 7) : le transfert `t : m(a) -= 1; m(b) += 1`
//! sur une `Map` — la transition touche les niveaux des deux clés `a` et
//! `b`, non adjacents. La sémantique est non locale (lire `a`, écrire `b`),
//! mais chaque *segment* est local : l'attribut hérité `H` porte le jeton
//! de transfert de `val(a)` vers `val(b)` à travers les niveaux
//! intermédiaires, et la mémoire de niveau `M` enregistre au nœud émetteur
//! que le décrément a déjà été produit (idempotence par `(nœud, niveau)`).
//!
//! **Surcoût poly(w) (T3)** : la transduction visite chaque nœud une fois
//! (mémo L1) et fait un travail O(1) par nœud ; `H` est un état O(1) par
//! niveau et `M` stocke au plus une entrée par `(nœud, niveau)` touché,
//! soit O(w) entrées par nœud. La compilation ne réifie aucune variable
//! intermédiaire (contrairement à Zhao/Ciardo) : `M` et `H` cousent les
//! segments.
//!
//! **Périmètre** : la destination doit exister dans le diagramme (clé
//! présente, ou nœud de présence conservé par une clé ultérieure) — le cas
//! « destination absente et collapsée » relève de l'explicitation d'axe
//! (T-EXPAND, § lookup), hors de ce jalon.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Form, Level, NodeId, Pool};

use crate::memoire::EffetMemoire;
use crate::schema::{ABSENT, Domaine, PRESENT, Schema};
use crate::transduction::{ContextePhiOut, Transduction};
use lisage_lang::value::Atom;

/// Fingerprint de base de la transduction de transfert.
const FINGERPRINT_TRANSFERT: u64 = 5001;
/// Nombre de niveaux touchés par un transfert entre deux clés : présence +
/// valeur de la source, présence + valeur de la destination.
pub const LARGEUR_TRANSFERT: u32 = 4;

/// Attribut hérité du transfert : état du jeton entre les niveaux.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum EtatTransfert {
    /// Descente normale : aucun transfert en cours.
    Normal,
    /// Transfert en cours : montant à ajouter à la valeur de la clé
    /// destination (1 unité transférée).
    Transfert(u64),
}

/// Transition non locale `m(a) -= 1; m(b) += 1` compilée en transduction à
/// mémoire de niveau (T3). Requiert `index_source < index_destination` : le
/// jeton hérité descend de `val(a)` vers `val(b)`.
#[derive(Clone)]
pub struct TransfertNonLocal {
    index_source: usize,
    index_destination: usize,
    domaine: Domaine,
}

impl TransfertNonLocal {
    /// Transfert de la clé `index_source` vers la clé `index_destination`
    /// (dans l'ordre des clés de la `Map`), sur le domaine des valeurs.
    ///
    /// # Panics
    /// Si `index_source >= index_destination` (le jeton hérité ne remonte
    /// pas).
    pub fn new(index_source: usize, index_destination: usize, domaine: Domaine) -> Self {
        assert!(
            index_source < index_destination,
            "le transfert exige source < destination (jeton hérité descendant)"
        );
        TransfertNonLocal {
            index_source,
            index_destination,
            domaine,
        }
    }

    /// Largeur `w` de la transition : niveaux touchés (T3, surcoût poly(w)).
    pub fn largeur(&self) -> u32 {
        LARGEUR_TRANSFERT
    }

    /// Niveau valeur de la clé source.
    fn niveau_source(&self) -> Level {
        Schema::niveau_valeur(self.index_source)
    }

    /// Niveau valeur de la clé destination.
    fn niveau_destination(&self) -> Level {
        Schema::niveau_valeur(self.index_destination)
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
            // φ_disp(val(a), h, v) = v ≥ 1 ? h_transfert(1) : stop :
            // le jeton naît à la valeur de a (1 unité à transférer).
            if arc >= 1 {
                EtatTransfert::Transfert(1)
            } else {
                EtatTransfert::Normal
            }
        } else {
            // Le jeton traverse les niveaux intermédiaires inchangé.
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
        let mut arcs = ctx.arcs;
        let mut effet = EffetMemoire::new();
        if ctx.level == self.niveau_source() {
            // Déjà produit par ce nœud à cette visite ? (idempotence M).
            let deja_produit = ctx.memoire.lire(ctx.noeud_entree, ctx.level) == Some(&true);
            if !deja_produit {
                let mut remap = BTreeMap::new();
                let mut decremente = false;
                for (v, (n, s)) in arcs {
                    if v >= 1 {
                        remap.insert(v - 1, (n, s));
                        decremente = true;
                    } else {
                        remap.insert(v, (n, s));
                    }
                }
                if decremente {
                    // M[id(n), val(a)] ← « décrément déjà produit ».
                    effet = EffetMemoire::une(ctx.noeud_entree, ctx.level, true);
                }
                arcs = remap;
            }
        } else if ctx.level == Schema::niveau_presence(self.index_destination)
            && let EtatTransfert::Transfert(montant) = ctx.h
        {
            // Clé destination absente : φ_disp(k_b?, h_transfert(w), ·) =
            // h_pose(w+1) — on rend la clé présente avec la valeur transférée.
            let montant = *montant;
            if self.domaine.contient(Atom(montant))
                && let Some((suite, _)) = arcs.remove(&ABSENT)
            {
                let noeud_valeur = pool.intern(
                    ctx.anchor,
                    Form::Node {
                        level: self.niveau_destination(),
                        arcs: BTreeMap::from([(montant, suite)]),
                    },
                );
                arcs.insert(PRESENT, (noeud_valeur, ()));
            }
        } else if ctx.level == self.niveau_destination()
            && let EtatTransfert::Transfert(montant) = ctx.h
        {
            // φ_out(val(b), h_transfert(w), …) : pose w+1 sur l'arc valeur.
            let montant = *montant;
            let mut remap = BTreeMap::new();
            for (u, (n, s)) in arcs {
                let cible = u + montant;
                if self.domaine.contient(Atom(cible)) {
                    remap.insert(cible, (n, s));
                } else {
                    // Hors domaine : le transfert est abandonné sur cet arc.
                    remap.insert(u, (n, s));
                }
            }
            arcs = remap;
        }
        let arcs = arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
        (
            pool.intern(
                ctx.anchor,
                Form::Node {
                    level: ctx.level,
                    arcs,
                },
            ),
            effet,
        )
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
