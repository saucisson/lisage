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

/// Transduction qui lit puis écrit dans la mémoire de niveau à chaque visite
/// d'un nœud : `M[id(n), ℓ] ← h` (l'attribut hérité = profondeur). La valeur
/// lue (si présente) est accumulée dans un compteur partagé pour prouver que
/// φ_out observe bien l'état stocké par une visite antérieure.
struct EcritMemoire {
    lus: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

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
        // elle est visible ici et comptabilisée (test : φ_out observe l'état
        // stocké par une visite antérieure du même nœud).
        if let Some(v) = ctx.memoire.lire(ctx.noeud_entree, ctx.level) {
            self.lus.fetch_add(*v, std::sync::atomic::Ordering::SeqCst);
        }
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

    let lus = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut runner = Transducteur::avec_politique(
        EcritMemoire { lus: lus.clone() },
        PolitiqueMemoire::ParNiveau,
    );
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
    // BLOCKING review : la lecture mémoire doit être observée. On exécute une
    // seconde fois : chaque nœud relit la valeur stockée à la première passe.
    let (r2, _) = runner.run(&entree, &mut sortie, anchor, b).unwrap();
    assert_eq!(
        r2, resultat,
        "déterminisme : même résultat (la version bumpée invalide la mémo, le nœud est recalculé)"
    );
    let total_lus = lus.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        total_lus > 0,
        "φ_out a observé l'état stocké par une visite antérieure (T3), total lu = {total_lus}"
    );
}

/// M2-3 : la politique `Aucune` rejette les effets mémoire (TO-FIX review).
#[test]
fn test_politique_aucune_rejette_effet_memoire() {
    let mut entree = pool();
    let mut sortie = pool();
    let anchor = Anchor(0);
    let vrai = entree.intern(anchor, Form::Terminal(true));
    let faux = entree.intern(anchor, Form::Terminal(false));
    // Deux arcs vers des cibles distinctes : pas de R-collapse.
    let a = entree.intern(
        anchor,
        Form::Node {
            level: Level(0),
            arcs: BTreeMap::from([(0, vrai), (1, faux)]),
        },
    );
    let lus = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    // Politique Aucune (défaut) : l'effet mémoire doit être rejeté.
    let mut runner = Transducteur::new(EcritMemoire { lus: lus.clone() });
    let resultat = runner.run(&entree, &mut sortie, anchor, a);
    assert!(
        matches!(
            resultat,
            Err(lisage_core::error::CoreError::MemoireNonAutorisee)
        ),
        "politique Aucune ⇒ effet mémoire rejeté, got {resultat:?}"
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
