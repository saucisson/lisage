//! Tests des formes relatives (M4-2, énoncé C8, spec § 2.4.5) : activation
//! du shift (ancre relative), ancres composées par fragment, groupe de
//! transformations de clés (instance canonique GroupeShift, section
//! min = 0), arc d'entrée (terme, handle). Modèle-test : la fenêtre
//! glissante — la base de séquence vit sur l'arc d'entrée, la glisse est
//! le déplacement d'ancre, le pool reste à O(1) formes.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::relatif::{
    AncreComposee, AncreRelative, ArcEntree, Fragment, GroupeCles, GroupeShift,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// Graine du LCG des tests de propriété (même constante que les autres suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplicateur du LCG des tests de propriété.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// Incrément du LCG des tests de propriété.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Cardinalité déclarée du mock : assez grande pour ne jamais collapser.
const CARDINALITE_MOCK: u64 = 1_000_000;
/// Taille de la fenêtre glissante du modèle-test.
const TAILLE_FENETRE: u64 = 4;
/// Nombre de glisses de la fenêtre glissante du modèle-test.
const NB_GLISSES: u64 = 50;
/// Nombre de décalages par fragment du test de dégradation O(Σ kᵢ).
const NB_DECALAGES_PAR_FRAGMENT: u64 = 8;
/// Niveau du fragment « machine ».
const NIVEAU_MACHINE: u32 = 1;
/// Niveau du fragment « convoyeur ».
const NIVEAU_CONVOYEUR: u32 = 2;
/// Position de base de l'ancre relative.
const POSITION_BASE: u32 = 100;

struct MockKeys;

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITE_MOCK
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        None
    }
}

struct MockController {
    keys: MockKeys,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(11);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        None
    }
}

fn arena() -> Arena<MockController> {
    Arena::new(MockController { keys: MockKeys }, Box::new(NoOpObserver))
}

/// M4-2 : ancre relative — activation du shift. La position absolue se lit
/// `position + decalage`, la glisse décale, le retour en ancre absolue est
/// saturé.
#[test]
fn test_ancre_relative_shift() {
    let ancre = AncreRelative::nouvelle(POSITION_BASE);
    assert_eq!(ancre.decalage, 0);
    assert_eq!(ancre.absolue(), POSITION_BASE as i64);
    let glissee = ancre.decalee(1).decalee(1).decalee(-1);
    assert_eq!(glissee.absolue(), POSITION_BASE as i64 + 1);
    assert_eq!(glissee.vers_ancre(), Anchor(POSITION_BASE + 1));
    // Depuis une ancre absolue M1 : décalage nul.
    let depuis = AncreRelative::depuis_ancre(Anchor(7));
    assert_eq!(depuis.decalage, 0);
    assert_eq!(depuis.absolue(), 7);
}

/// M4-2 : ancres composées — enregistrement nommé de décalages, un par
/// fragment ; zéro = début de fragment ; ordre lexicographique des noms ;
/// composition point par point.
#[test]
fn test_ancre_composee_fragments() {
    let machine = Fragment("machine".to_string());
    let convoyeur = Fragment("convoyeur".to_string());
    let ancre = AncreComposee::nouveau()
        .avec(machine.clone(), 3)
        .avec(convoyeur.clone(), -1);
    assert_eq!(ancre.decalage(&machine), 3);
    assert_eq!(ancre.decalage(&convoyeur), -1);
    // Fragment absent : zéro = début de fragment.
    assert_eq!(ancre.decalage(&Fragment("autre".to_string())), 0);
    assert!(!ancre.est_nulle());
    // La glisse décale un seul fragment.
    let glissee = ancre.decalee(&machine, 2);
    assert_eq!(glissee.decalage(&machine), 5);
    assert_eq!(glissee.decalage(&convoyeur), -1);
    // Composition point par point (lecture absolue = somme des arguments).
    let composee = ancre.composer(&glissee);
    assert_eq!(composee.decalage(&machine), 8);
    assert_eq!(composee.decalage(&convoyeur), -2);
    // Ordre canonique : lexicographique des noms.
    let noms: Vec<&str> = ancre.fragments().map(|(f, _)| f.0.as_str()).collect();
    assert_eq!(noms, ["convoyeur", "machine"]);
    assert!(AncreComposee::nouveau().est_nulle());
}

/// M4-2, modèle-test : la **fenêtre glissante**. La base de séquence est
/// l'argument sur l'arc d'entrée, la glisse est le déplacement d'ancre ;
/// la fenêtre stockée en relatif (clés 0..W-1, section min = 0) est
/// partagée : le pool reste à O(1) formes quand la base parcourt ℕ, et la
/// lecture absolue redonne la fenêtre {base..base+W-1}.
#[test]
fn test_fenetre_glissante_pool_constant() {
    let groupe = GroupeShift;
    let mut pool = arena();
    let anchor = Anchor(0);
    let vrai = pool.intern(anchor, Form::Terminal(true));
    // Fenêtre relative : clés 0..W-1 (section min = 0), toutes vraies.
    let arcs: BTreeMap<u64, NodeId> = (0..TAILLE_FENETRE).map(|k| (k, vrai)).collect();
    let fenetre = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs,
        },
    );
    let noeuds_apres_fenetre = pool.node_count();
    // Glisse : la base avance de 1 à chaque pas, la forme stockée est
    // inchangée (R-share), seul l'argument de l'arc d'entrée change.
    for base in 0..NB_GLISSES as i64 {
        let arc = ArcEntree::nouvelle(base, pool.handle(fenetre, anchor));
        // Lecture absolue : la fenêtre couvre {base..base+W-1}.
        for k in 0..TAILLE_FENETRE {
            assert_eq!(arc.lire_cle(&groupe, &k), (base + k as i64) as u64);
        }
        // Ré-interner la même forme relative : hit, aucun nœud créé.
        let meme = pool.intern(
            anchor,
            Form::Node {
                level: Level(1),
                arcs: (0..TAILLE_FENETRE).map(|k| (k, vrai)).collect(),
            },
        );
        assert_eq!(meme, fenetre, "forme relative partagée");
    }
    assert_eq!(
        pool.node_count(),
        noeuds_apres_fenetre,
        "le pool reste à O(1) formes pendant la glisse"
    );
}

/// M4-2 : ancres composées — la dégradation reste en O(Σ kᵢ) points
/// irréguliers, jamais en O(∏) : les formes se factorisent par fragment.
#[test]
fn test_ancres_composees_degradation_somme() {
    let mut pool = arena();
    let anchor = Anchor(0);
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let machine = Fragment("machine".to_string());
    let convoyeur = Fragment("convoyeur".to_string());
    // k₁ formes pour le fragment machine, k₂ pour le fragment convoyeur.
    for i in 0..NB_DECALAGES_PAR_FRAGMENT {
        pool.intern(
            anchor,
            Form::Node {
                level: Level(NIVEAU_MACHINE),
                arcs: BTreeMap::from([(i, vrai)]),
            },
        );
    }
    for j in 0..NB_DECALAGES_PAR_FRAGMENT {
        pool.intern(
            anchor,
            Form::Node {
                level: Level(NIVEAU_CONVOYEUR),
                arcs: BTreeMap::from([(j, vrai)]),
            },
        );
    }
    // 1 terminal + k₁ + k₂ nœuds : la somme, pas le produit k₁ × k₂.
    let attendu = 1 + 2 * NB_DECALAGES_PAR_FRAGMENT as usize;
    assert_eq!(pool.node_count(), attendu);
    assert!(attendu < (1 + NB_DECALAGES_PAR_FRAGMENT as usize).pow(2));
    // L'ancre composée porte les deux décalages nommés.
    let ancre = AncreComposee::nouveau().avec(machine, 2).avec(convoyeur, 5);
    assert_eq!(ancre.decalage(&Fragment("machine".to_string())), 2);
    assert_eq!(ancre.decalage(&Fragment("convoyeur".to_string())), 5);
}

/// M4-2 (tier propriété, LCG) : contrat du groupe de transformations —
/// section confluente (idempotente), homomorphisme f(n₁)∘f(n₂) =
/// f(n₁ ⊕ n₂), inverse t⁻¹, noyau trivial (f injective), composition des
/// arcs d'entrée. Valeurs bornées pour éviter la saturation.
#[test]
fn test_propriete_groupe_shift() {
    let groupe = GroupeShift;
    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };
    // Borne des arguments/clés du test : pas de saturation.
    const BORNE: u64 = 500;

    for _ in 0..1000 {
        // Arcs aléatoires (clés dans [0, BORNE), cibles arbitraires).
        let nb_arcs = 1 + suivant() % 8;
        let arcs: BTreeMap<u64, NodeId> = (0..nb_arcs)
            .map(|_| (suivant() % BORNE, NodeId(suivant() as u32)))
            .collect();
        // (iii) Confluence : la section est idempotente — appliquée à la
        // forme normalisée, elle rend l'argument neutre et la même forme.
        let (n1, arcs_norm) = groupe.section_arcs(&arcs);
        let (n2, arcs_norm2) = groupe.section_arcs(&arcs_norm);
        assert_eq!(n2, groupe.neutre(), "section idempotente");
        assert_eq!(arcs_norm2, arcs_norm);
        // La section min = 0 commence bien à la clé 0.
        assert!(arcs_norm.contains_key(&0));
        assert_eq!(n1, *arcs.keys().next().unwrap() as i64);

        // (v) Homomorphisme : f(n₁ ⊕ n₂) = f(n₁)∘f(n₂). Clé assez grande
        // pour que les lectures intermédiaires ne saturent pas à 0.
        let cle_grande = BORNE + suivant() % BORNE;
        let a1 = (suivant() % (2 * BORNE)) as i64 - BORNE as i64;
        let a2 = (suivant() % (2 * BORNE)) as i64 - BORNE as i64;
        let compose = groupe.composer(&a1, &a2);
        assert_eq!(
            groupe.appliquer(&cle_grande, &compose),
            groupe.appliquer(&groupe.appliquer(&cle_grande, &a1), &a2)
        );

        // (i) Inverse : f(n) puis f(n)⁻¹ = identité (clé assez grande).
        let n = (suivant() % BORNE) as i64;
        assert_eq!(
            groupe.appliquer(&groupe.appliquer(&cle_grande, &n), &groupe.inverser(&n)),
            cle_grande
        );

        // (vi) Noyau trivial : f injective sur les arguments.
        let n1 = (suivant() % BORNE) as i64;
        let n2 = (suivant() % BORNE) as i64;
        if groupe.appliquer(&cle_grande, &n1) == groupe.appliquer(&cle_grande, &n2) {
            assert_eq!(n1, n2, "f(n₁) = f(n₂) ⇒ n₁ = n₂ (stockage modulo le noyau)");
        }

        // Arcs d'entrée : la lecture absolue compose les arguments.
        let handle = Handle {
            controller: ControllerId(11),
            anchor: Anchor(0),
            node: NodeId(1),
        };
        let arc1 = ArcEntree::nouvelle(a1, handle);
        let arc2 = ArcEntree::nouvelle(a2, handle);
        let arc_compose = arc1.composer(&groupe, &arc2);
        assert_eq!(
            arc_compose.lire_cle(&groupe, &cle_grande),
            arc2.lire_cle(&groupe, &arc1.lire_cle(&groupe, &cle_grande))
        );
    }
}
