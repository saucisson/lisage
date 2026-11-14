//! Tests for relative forms (M4-2, statement C8, spec § 2.4.5): shift
//! activation (relative anchor), composite anchors per fragment, key
//! transformation group (canonical instance GroupeShift, min = 0 section),
//! entry arc (term, handle). Test model: the sliding window — the sequence
//! base lives on the entry arc, the slide is the anchor displacement, the pool
//! stays at O(1) forms.

use std::collections::BTreeMap;

use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool,
};
use lisage_algebra::relatif::{
    AncreComposee, AncreRelative, ArcEntree, Fragment, GroupeCles, GroupeShift,
};
use lisage_core::arena::Arena;
use lisage_core::observer::NoOpObserver;

/// LCG seed for property tests (same constant as the other suites).
const GRAINE_LCG: u64 = 0x9E37_79B9_7F4A_7C15;
/// LCG multiplier for property tests.
const MULTIPLICATEUR_LCG: u64 = 6364136223846793005;
/// LCG increment for property tests.
const INCREMENT_LCG: u64 = 1442695040888963407;

/// Declared mock cardinality: large enough to never collapse.
const CARDINALITE_MOCK: u64 = 1_000_000;
/// Size of the sliding window of the test model.
const TAILLE_FENETRE: u64 = 4;
/// Number of slides of the sliding window of the test model.
const NB_GLISSES: u64 = 50;
/// Number of shifts per fragment in the O(Σ kᵢ) degradation test.
const NB_DECALAGES_PAR_FRAGMENT: u64 = 8;
/// Level of the "machine" fragment.
const NIVEAU_MACHINE: u32 = 1;
/// Level of the "convoyeur" fragment.
const NIVEAU_CONVOYEUR: u32 = 2;
/// Base position of the relative anchor.
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

/// M4-2: relative anchor — shift activation. The absolute position reads as
/// `position + decalage`, the slide shifts, the return to an absolute anchor
/// is saturated.
#[test]
fn test_ancre_relative_shift() {
    let ancre = AncreRelative::nouvelle(POSITION_BASE);
    assert_eq!(ancre.decalage, 0);
    assert_eq!(ancre.absolue(), POSITION_BASE as i64);
    let glissee = ancre.decalee(1).decalee(1).decalee(-1);
    assert_eq!(glissee.absolue(), POSITION_BASE as i64 + 1);
    assert_eq!(glissee.vers_ancre(), Anchor(POSITION_BASE + 1));
    // From an absolute M1 anchor: zero shift.
    let depuis = AncreRelative::depuis_ancre(Anchor(7));
    assert_eq!(depuis.decalage, 0);
    assert_eq!(depuis.absolue(), 7);
    // Saturation of the conversion to an absolute anchor: a negative shift
    // below position 0 clamps to 0 (no wrap), an overflow above clamps to
    // u32::MAX.
    assert_eq!(
        AncreRelative::nouvelle(0).decalee(-1).vers_ancre(),
        Anchor(0)
    );
    assert_eq!(
        AncreRelative::nouvelle(u32::MAX).decalee(5).vers_ancre(),
        Anchor(u32::MAX)
    );
}

/// M4-2: the effective shift reading is saturated in ℕ (total): key + negative
/// argument clamps to 0, overflow above clamps to u64::MAX.
#[test]
fn test_groupe_shift_saturation() {
    let groupe = GroupeShift;
    assert_eq!(groupe.appliquer(&0, &-5), 0, "low clamp to 0");
    assert_eq!(groupe.appliquer(&10, &-5), 5);
    assert_eq!(groupe.appliquer(&u64::MAX, &1), u64::MAX, "high clamp");
    assert_eq!(groupe.appliquer(&u64::MAX, &-1), u64::MAX - 1);
}

/// M4-2: composite anchors — named recording of shifts, one per fragment;
/// zero = fragment start; lexicographic order of names; pointwise
/// composition.
#[test]
fn test_ancre_composee_fragments() {
    let machine = Fragment("machine".to_string());
    let convoyeur = Fragment("convoyeur".to_string());
    let ancre = AncreComposee::nouveau()
        .avec(machine.clone(), 3)
        .avec(convoyeur.clone(), -1);
    assert_eq!(ancre.decalage(&machine), 3);
    assert_eq!(ancre.decalage(&convoyeur), -1);
    // Absent fragment: zero = fragment start.
    assert_eq!(ancre.decalage(&Fragment("autre".to_string())), 0);
    assert!(!ancre.est_nulle());
    // The slide shifts a single fragment.
    let glissee = ancre.decalee(&machine, 2);
    assert_eq!(glissee.decalage(&machine), 5);
    assert_eq!(glissee.decalage(&convoyeur), -1);
    // Pointwise composition (absolute reading = sum of the arguments).
    let composee = ancre.composer(&glissee);
    assert_eq!(composee.decalage(&machine), 8);
    assert_eq!(composee.decalage(&convoyeur), -2);
    // Canonical order: lexicographic by name.
    let noms: Vec<&str> = ancre.fragments().map(|(f, _)| f.0.as_str()).collect();
    assert_eq!(noms, ["convoyeur", "machine"]);
    assert!(AncreComposee::nouveau().est_nulle());
}

/// M4-2, test model: the **sliding window**. The sequence base is the
/// argument on the entry arc, the slide is the anchor displacement; the window
/// stored relatively (keys 0..W-1, min = 0 section) is shared: the pool stays
/// at O(1) forms while the base ranges over ℕ, and the absolute reading gives
/// back the window {base..base+W-1}.
#[test]
fn test_fenetre_glissante_pool_constant() {
    let groupe = GroupeShift;
    let mut pool = arena();
    let anchor = Anchor(0);
    let vrai = pool.intern(anchor, Form::Terminal(true));
    // Relative window: keys 0..W-1 (min = 0 section), all true.
    let arcs: BTreeMap<u64, NodeId> = (0..TAILLE_FENETRE).map(|k| (k, vrai)).collect();
    let fenetre = pool.intern(
        anchor,
        Form::Node {
            level: Level(1),
            arcs,
        },
    );
    let noeuds_apres_fenetre = pool.node_count();
    // Slide: the base advances by 1 at each step, the stored form is
    // unchanged (R-share), only the entry arc's argument changes.
    for base in 0..NB_GLISSES as i64 {
        let arc = ArcEntree::nouvelle(base, pool.handle(fenetre, anchor));
        // Absolute reading: the window covers {base..base+W-1}.
        for k in 0..TAILLE_FENETRE {
            assert_eq!(arc.lire_cle(&groupe, &k), (base + k as i64) as u64);
        }
        // Re-intern the same relative form: hit, no node created.
        let meme = pool.intern(
            anchor,
            Form::Node {
                level: Level(1),
                arcs: (0..TAILLE_FENETRE).map(|k| (k, vrai)).collect(),
            },
        );
        assert_eq!(meme, fenetre, "shared relative form");
    }
    assert_eq!(
        pool.node_count(),
        noeuds_apres_fenetre,
        "the pool stays at O(1) forms during the slide"
    );
}

/// M4-2: composite anchors — the degradation stays in O(Σ kᵢ) irregular
/// points, never in O(∏): the forms factorize per fragment.
#[test]
fn test_ancres_composees_degradation_somme() {
    let mut pool = arena();
    let anchor = Anchor(0);
    let vrai = pool.intern(anchor, Form::Terminal(true));
    let machine = Fragment("machine".to_string());
    let convoyeur = Fragment("convoyeur".to_string());
    // k₁ forms for the machine fragment, k₂ for the convoyeur fragment.
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
    // 1 terminal + k₁ + k₂ nodes: the sum, not the product k₁ × k₂.
    let attendu = 1 + 2 * NB_DECALAGES_PAR_FRAGMENT as usize;
    assert_eq!(pool.node_count(), attendu);
    assert!(attendu < (1 + NB_DECALAGES_PAR_FRAGMENT as usize).pow(2));
    // The composite anchor carries the two named shifts.
    let ancre = AncreComposee::nouveau().avec(machine, 2).avec(convoyeur, 5);
    assert_eq!(ancre.decalage(&Fragment("machine".to_string())), 2);
    assert_eq!(ancre.decalage(&Fragment("convoyeur".to_string())), 5);
}

/// M4-2 (property tier, LCG): contract of the transformation group —
/// confluent section (idempotent), homomorphism f(n₁)∘f(n₂) = f(n₁ ⊕ n₂),
/// inverse t⁻¹, trivial kernel (f injective), composition of entry arcs.
/// Bounded values to avoid saturation.
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
    // Bound on the test's arguments/keys: no saturation.
    const BORNE: u64 = 500;

    for _ in 0..1000 {
        // Random arcs (keys in [0, BORNE), arbitrary targets).
        let nb_arcs = 1 + suivant() % 8;
        let arcs: BTreeMap<u64, NodeId> = (0..nb_arcs)
            .map(|_| (suivant() % BORNE, NodeId(suivant() as u32)))
            .collect();
        // (iii) Confluence: the section is idempotent — applied to the
        // normalized form, it yields the neutral argument and the same form.
        let (n1, arcs_norm) = groupe.section_arcs(&arcs);
        let (n2, arcs_norm2) = groupe.section_arcs(&arcs_norm);
        assert_eq!(n2, groupe.neutre(), "idempotent section");
        assert_eq!(arcs_norm2, arcs_norm);
        // The min = 0 section indeed starts at key 0.
        assert!(arcs_norm.contains_key(&0));
        assert_eq!(n1, *arcs.keys().next().unwrap() as i64);

        // (v) Homomorphism: f(n₁ ⊕ n₂) = f(n₁)∘f(n₂). Key large enough that
        // the intermediate readings do not saturate to 0.
        let cle_grande = BORNE + suivant() % BORNE;
        let a1 = (suivant() % (2 * BORNE)) as i64 - BORNE as i64;
        let a2 = (suivant() % (2 * BORNE)) as i64 - BORNE as i64;
        let compose = groupe.composer(&a1, &a2);
        assert_eq!(
            groupe.appliquer(&cle_grande, &compose),
            groupe.appliquer(&groupe.appliquer(&cle_grande, &a1), &a2)
        );

        // (i) Inverse: f(n) then f(n)⁻¹ = identity (key large enough).
        let n = (suivant() % BORNE) as i64;
        assert_eq!(
            groupe.appliquer(&groupe.appliquer(&cle_grande, &n), &groupe.inverser(&n)),
            cle_grande
        );

        // (vi) Trivial kernel: f injective over the arguments.
        let n1 = (suivant() % BORNE) as i64;
        let n2 = (suivant() % BORNE) as i64;
        if groupe.appliquer(&cle_grande, &n1) == groupe.appliquer(&cle_grande, &n2) {
            assert_eq!(
                n1, n2,
                "f(n₁) = f(n₂) ⇒ n₁ = n₂ (storage modulo the kernel)"
            );
        }

        // Entry arcs: the absolute reading composes the arguments.
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
