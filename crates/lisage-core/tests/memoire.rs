//! Tests de la mémoire de niveau (M2-3, spec § 3.1, énoncé T3) : stockage
//! entre deux visites d'un même niveau, versioning (L1), effet mémoire
//! produit par φ_out, et politique déclarée par le contrôleur.

use std::collections::BTreeMap;

use lisage_algebra::interface::{Anchor, Controller, Form, Level, NodeId, PolitiqueMemoire, Pool};
use lisage_core::arena::Arena;
use lisage_core::memoire::{EffetMemoire, MemoireNiveau};
use lisage_core::observer::NoOpObserver;
use lisage_core::schema::{Domaine, ReferenceController, Schema};
use lisage_core::transduction::{ContextePhiOut, Transducteur, Transduction};
use lisage_lang::value::Atom;

type PoolRef = Arena<ReferenceController>;

fn pool() -> PoolRef {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    Arena::new(ReferenceController::new(schema), Box::new(NoOpObserver))
}

/// M2-3 : la mémoire de niveau stocke et versionne (L1).
#[test]
fn test_memoire_niveau_stockage_et_version() {
    let mut memoire: MemoireNiveau<u64> = MemoireNiveau::new();
    assert_eq!(memoire.version(), 0);
    assert_eq!(memoire.lire(NodeId(0), Level(0)), None);

    let ancien = memoire.ecrire(NodeId(0), Level(0), 42);
    assert_eq!(ancien, None);
    assert_eq!(
        memoire.version(),
        1,
        "chaque écriture incrémente la version"
    );
    assert_eq!(memoire.lire(NodeId(0), Level(0)), Some(&42));

    let ancien = memoire.ecrire(NodeId(0), Level(0), 43);
    assert_eq!(ancien, Some(42), "l'écriture retourne l'ancienne valeur");
    assert_eq!(memoire.version(), 2);
    assert_eq!(memoire.lire(NodeId(0), Level(0)), Some(&43));
    assert_eq!(
        memoire.lire(NodeId(1), Level(0)),
        None,
        "clé par (nœud, niveau)"
    );
    assert_eq!(memoire.taille(), 1);
}

/// M2-3 : l'effet mémoire s'applique et bump la version.
#[test]
fn test_effet_memoire_appliquer() {
    let mut memoire: MemoireNiveau<u64> = MemoireNiveau::new();
    let effet = EffetMemoire::une(NodeId(3), Level(1), 7);
    effet.appliquer(&mut memoire);
    assert_eq!(memoire.version(), 1);
    assert_eq!(memoire.lire(NodeId(3), Level(1)), Some(&7));

    let effet_vide: EffetMemoire<u64> = EffetMemoire::new();
    effet_vide.appliquer(&mut memoire);
    assert_eq!(memoire.version(), 1, "effet vide : pas de bump");
}

/// Transduction qui écrit dans la mémoire de niveau à chaque visite d'un
/// nœud : `M[id(n), ℓ] ← h` (l'attribut hérité = profondeur). Utilisée pour
/// vérifier que φ_out reçoit la mémoire et que l'effet est appliqué.
struct EcritMemoire;

impl Transduction for EcritMemoire {
    type H = u64;
    type S = u64;
    type Out = bool;
    type M = u64;

    fn fingerprint(&self) -> u64 {
        3001
    }
    fn h0(&self) -> u64 {
        0
    }
    fn phi_disp(&self, _level: Level, h: &u64, _arc: u64) -> u64 {
        h + 1
    }
    fn phi_aggr(&self, _level: Level, _h: &u64, arcs: &BTreeMap<u64, (Option<Level>, u64)>) -> u64 {
        arcs.values().map(|(_, s)| *s).max().unwrap_or(0)
    }
    fn phi_out<P: Pool<Key = u64, Payload = bool>>(
        &self,
        pool: &mut P,
        ctx: ContextePhiOut<'_, Self::H, Self::S, Self::M>,
    ) -> (NodeId, EffetMemoire<u64>) {
        // Lit la mémoire : si une valeur était déjà stockée à ce (nœud, niveau),
        // elle est visible ici (test : la version a changé entre deux visites).
        let _lu = ctx.memoire.lire(ctx.noeud_entree, ctx.level);
        // Écrit h dans la mémoire à (noeud_entree, level).
        let effet = EffetMemoire::une(ctx.noeud_entree, ctx.level, *ctx.h);
        let arcs = ctx.arcs.into_iter().map(|(v, (n, _))| (v, n)).collect();
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
        _h: &u64,
    ) -> (NodeId, u64) {
        (pool.intern(anchor, Form::Terminal(payload)), 0)
    }
}

/// M2-3 : une transduction à mémoire écrit bien dans la mémoire de niveau
/// via l'effet produit par φ_out, et la version est bumpée (L1).
#[test]
fn test_transduction_avec_memoire_ecrit() {
    let mut entree = pool();
    let mut sortie = pool();
    let anchor = Anchor(0);
    let vrai = entree.intern(anchor, Form::Terminal(true));
    let faux = entree.intern(anchor, Form::Terminal(false));
    let a = entree.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let b = entree.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs: BTreeMap::from([(0, a), (1, vrai)]),
        },
    );

    let mut runner = Transducteur::new(EcritMemoire);
    assert_eq!(runner.version_memoire(), 0, "mémoire vide au départ");
    let (resultat, _) = runner.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(
        sortie.resolve(resultat).unwrap(),
        entree.resolve(b).unwrap(),
        "copie identité"
    );
    // Deux nœuds non terminaux (a et b) ont chacun écrit dans la mémoire.
    assert!(
        runner.version_memoire() >= 2,
        "version bumpée par les effets mémoire (L1), version = {}",
        runner.version_memoire()
    );
}

/// M2-3 : la politique de stockage est déclarée par le contrôleur
/// (`Aucune` par défaut en M1, `ParNiveau` pour autoriser la mémoire).
#[test]
fn test_politique_memoire_du_controleur() {
    let schema = Schema::Set(Domaine::nouveau(vec![Atom(0), Atom(1)]));
    let controleur = ReferenceController::new(schema);
    assert_eq!(
        controleur.politique_memoire(),
        PolitiqueMemoire::Aucune,
        "politique par défaut (M1) : pas de mémoire de niveau"
    );
}
