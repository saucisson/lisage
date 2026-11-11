//! Tests du défaut généralisé (M4-3, énoncé A6, spec § 2.4.1) : (D-abs)
//! absorption des arcs vers D^déf sur tout sous-domaine R ⊆ Val(ℓ), et
//! (D-trans) nœud transparent (A = ∅ ou toutes cibles ≡ D → D). Le D^déf
//! reste désigné par le contrôleur (`default_payload`), jamais par la
//! forme. Le système § 2.3 (R-default sur δ(ℓ) seul) reste le défaut.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::reduction::{Reduction, reduire_dabs};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// Graine du LCG des tests de propriété (même constante que les autres suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplicateur du LCG des tests de propriété.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// Incrément du LCG des tests de propriété.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Cardinalité du mock : 4 valeurs par niveau.
const CARDINALITE_MOCK: u64 = 4;
/// Niveau de test.
const NIVEAU_TEST: u32 = 1;
/// Ancre de test.
const ANCRE_TEST: Anchor = Anchor(0);

/// Le nœud 0 joue le rôle du diagramme de défaut D^déf dans les tests de
/// propriété.
const NOEUD_DEFAUT: NodeId = NodeId(0);

struct MockKeys {
    avec_defaut: bool,
}

impl KeyAlgebra for MockKeys {
    type Key = u64;
    fn normal_form(&self, key: &Self::Key) -> Self::Key {
        *key
    }
    fn cardinality(&self, _level: Level) -> u64 {
        CARDINALITE_MOCK
    }
    fn default(&self, _level: Level) -> Option<Self::Key> {
        self.avec_defaut.then_some(0)
    }
}

struct MockController {
    keys: MockKeys,
    absorption: bool,
}

impl Controller for MockController {
    type Key = u64;
    type Payload = bool;
    type Keys = MockKeys;
    const FINGERPRINT: ControllerId = ControllerId(13);
    fn keys(&self) -> &Self::Keys {
        &self.keys
    }
    fn default_payload(&self, _level: Level) -> Option<Self::Payload> {
        Some(false)
    }
    fn absorption_generalisee(&self) -> bool {
        self.absorption
    }
}

/// Pool avec défaut généralisé activé (D-abs/D-trans) et clé de défaut 0.
fn arena_dabs() -> Arena<MockController> {
    Arena::new(
        MockController {
            keys: MockKeys { avec_defaut: true },
            absorption: true,
        },
        Box::new(NoOpObserver),
    )
}

/// Pool avec le système § 2.3 seul (absorption non activée).
fn arena_23() -> Arena<MockController> {
    Arena::new(
        MockController {
            keys: MockKeys { avec_defaut: true },
            absorption: false,
        },
        Box::new(NoOpObserver),
    )
}

/// M4-3 (D-abs) : tout arc vers D^déf est absorbé, sur n'importe quel
/// sous-domaine — pas seulement l'arc δ(ℓ) = 0. Ici les clés 0, 2 et 3
/// mènent à ⊥ = D^déf : elles disparaissent, il reste {1 ↦ ⊤}.
#[test]
fn test_dabs_absorption_tout_sous_domaine() {
    let mut pool = arena_dabs();
    let faux = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let vrai = pool.intern(ANCRE_TEST, Form::Terminal(true));
    let noeud = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(0, faux), (1, vrai), (2, faux), (3, faux)]),
        },
    );
    assert_eq!(
        pool.resolve(noeud),
        Ok(&Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(1, vrai)]),
        }),
        "D-abs : les arcs 0, 2, 3 vers D^déf sont absorbés"
    );
}

/// M4-3 : sans D-abs (système § 2.3, `absorption_generalisee` à false),
/// seul l'arc δ(ℓ) = 0 vers D^déf est absorbé — l'arc 2 ↦ ⊥ reste.
#[test]
fn test_sans_dabs_rdefault_seul() {
    let mut pool = arena_23();
    let faux = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let vrai = pool.intern(ANCRE_TEST, Form::Terminal(true));
    let noeud = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(0, faux), (1, vrai), (2, faux)]),
        },
    );
    assert_eq!(
        pool.resolve(noeud),
        Ok(&Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(1, vrai), (2, faux)]),
        }),
        "R-default : seul l'arc δ(ℓ) = 0 est absorbé"
    );
}

/// M4-3 (D-trans) : A = ∅ après absorption → collapse en D^déf. Un nœud
/// dont le seul arc (clé 3, hors δ) mène à D^déf est transparent : il
/// vaut D^déf lui-même.
#[test]
fn test_dtrans_forme_vide_collapse() {
    let mut pool = arena_dabs();
    let faux = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let noeud = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(3, faux)]),
        },
    );
    assert_eq!(noeud, faux, "D-trans : ν(ℓ, {{3 ↦ D^déf}}) = D^déf");
    assert_eq!(pool.resolve(noeud), Ok(&Form::Terminal(false)));
}

/// M4-3 (D-trans) « toutes cibles ≡ D » : un seul arc explicite sur δ
/// collapse vers sa cible ; deux arcs explicites vers la même cible hors
/// couverture ne collapsent pas (présence, confluence § 2.3).
#[test]
fn test_dtrans_couverture() {
    let mut pool = arena_dabs();
    let vrai = pool.intern(ANCRE_TEST, Form::Terminal(true));
    // Seul l'arc δ = 0 explicite, cible ⊤ : couvre (absents = D^déf ≠ ⊤,
    // mais la règle de couverture autorise le collapse sur arc δ seul).
    let seul_delta = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(0, vrai)]),
        },
    );
    assert_eq!(seul_delta, vrai, "arc δ seul : collapse en sa cible");
    // Deux arcs explicites vers ⊤, hors couverture : pas de collapse.
    let deux_arcs = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(1, vrai), (2, vrai)]),
        },
    );
    assert_eq!(
        pool.resolve(deux_arcs),
        Ok(&Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(1, vrai), (2, vrai)]),
        }),
        "hors couverture : les deux arcs restent (présence)"
    );
}

/// M4-3 : sans clé de défaut (δ(ℓ) = None), D-abs ne s'applique pas —
/// les arcs absents ne vaudraient pas D^déf.
#[test]
fn test_sans_cle_defaut_pas_dabs() {
    let mut pool = Arena::new(
        MockController {
            keys: MockKeys { avec_defaut: false },
            absorption: true,
        },
        Box::new(NoOpObserver),
    );
    let faux = pool.intern(ANCRE_TEST, Form::Terminal(false));
    let noeud = pool.intern(
        ANCRE_TEST,
        Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(1, faux), (2, faux)]),
        },
    );
    assert_eq!(
        pool.resolve(noeud),
        Ok(&Form::Node {
            level: Level(NIVEAU_TEST),
            arcs: BTreeMap::from([(1, faux), (2, faux)]),
        }),
        "pas de δ(ℓ) : pas d'absorption"
    );
}

/// M4-3 (tier propriété, LCG) : confluence du défaut généralisé —
/// `reduire_dabs ∘ reduire_dabs = reduire_dabs` (idempotence), aucune forme
/// résultante ne conserve d'arc vers D^déf (quand δ existe), et un
/// collapse ne peut viser que D^déf ou une cible couverte.
#[test]
fn test_propriete_dabs_confluence() {
    let est_defaut = |node: NodeId| node == NOEUD_DEFAUT;
    let mut graine = GRAINE_LCG;
    let mut suivant = move || {
        graine = graine
            .wrapping_mul(MULTIPLICATEUR_LCG)
            .wrapping_add(INCREMENT_LCG);
        graine >> 33
    };

    for _ in 0..1000 {
        let nb_arcs = suivant() % 6;
        let arcs: BTreeMap<u64, NodeId> = (0..nb_arcs)
            .map(|_| {
                let cle = suivant() % CARDINALITE_MOCK;
                let cible = NodeId((suivant() % 3) as u32);
                (cle, cible)
            })
            .collect();
        let cle_defaut = if suivant() % 2 == 0 { Some(0) } else { None };

        let premier = reduire_dabs(arcs.clone(), cle_defaut, &est_defaut, CARDINALITE_MOCK);
        // Idempotence : une forme réduite est un point fixe (un Collapse
        // vise un nœud, pas une forme d'arcs — rien à re-réduire).
        if let Reduction::Forme(arcs_reduits) = &premier {
            let second = reduire_dabs(
                arcs_reduits.clone(),
                cle_defaut,
                &est_defaut,
                CARDINALITE_MOCK,
            );
            assert_eq!(premier.clone(), second, "idempotence de reduire_dabs");
        }

        // D-abs : quand δ(ℓ) existe, plus aucun arc vers D^déf ne reste.
        if cle_defaut.is_some()
            && let Reduction::Forme(a) = &premier
        {
            assert!(
                a.values().all(|c| !est_defaut(*c)),
                "aucun arc vers D^déf après D-abs"
            );
        }
        // D-trans / R-collapse : un collapse n'a lieu que si la couverture
        // est satisfaite (mêmes règles que `reduire`) ou si A = ∅ après
        // absorption (alors la cible est D^déf).
        if let Reduction::Collapse(cible) = &premier {
            // Mêmes arcs après D-abs que dans `reduire_dabs`.
            let arcs_filtres: BTreeMap<u64, NodeId> = match cle_defaut {
                Some(_) => arcs
                    .iter()
                    .filter(|(_, c)| !est_defaut(**c))
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                None => arcs.clone(),
            };
            let toutes_defaut = !arcs.is_empty() && arcs.values().all(|c| est_defaut(*c));
            let couvre = match cle_defaut {
                None => arcs_filtres.len() as u64 >= CARDINALITE_MOCK,
                Some(cle) => {
                    est_defaut(*cible)
                        || toutes_defaut
                        || (arcs_filtres.len() == 1 && arcs_filtres.contains_key(&cle))
                }
            };
            assert!(couvre, "collapse licite: cible={cible:?}");
        }
    }
}
