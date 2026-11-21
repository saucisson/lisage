//! D1-3 tests for `TraceVal` / `Traceable`: bounds of the mini-ADT,
//! elision (lazy fragments are computed only when forced), the Debug
//! fallback, and the structured impls of the H/S/M types of the existing
//! transductions.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use lisage_algebra::interface::{Level, NodeId};
use lisage_core::lookup::Mode;
use lisage_core::observer::{
    TRACE_VAL_MAX_JETONS, TRACE_VAL_MAX_PROFONDEUR, TRACE_VAL_MAX_TEXTE, TraceVal, Traceable,
};
use lisage_core::operations::KeyPresentes;
use lisage_core::pli::{BoolOu, Naturel};
use lisage_core::transfert::EtatTransfert;

/// D1-3 DoD: the ADT is bounded — texts are truncated, collections capped
/// with an `Elide` marker.
#[test]
fn test_trace_val_bornes() {
    // Text bound.
    let long = "x".repeat(TRACE_VAL_MAX_TEXTE * 2);
    let valeur = TraceVal::texte(long);
    let TraceVal::Texte(texte) = &valeur else {
        panic!("texte attendu")
    };
    assert_eq!(texte.chars().count(), TRACE_VAL_MAX_TEXTE + 1);
    assert!(texte.ends_with('…'));

    // Ensemble bound: cap + Elide marker.
    let valeurs: Vec<TraceVal> = (0..TRACE_VAL_MAX_JETONS as i64 + 10)
        .map(TraceVal::entier)
        .collect();
    let TraceVal::Ensemble(ensemble) = TraceVal::ensemble(valeurs) else {
        panic!("ensemble attendu")
    };
    assert_eq!(ensemble.len(), TRACE_VAL_MAX_JETONS + 1);
    assert_eq!(ensemble.last(), Some(&TraceVal::Elide));

    // Carte bound: cap + Elide pair.
    let paires: Vec<(TraceVal, TraceVal)> = (0..TRACE_VAL_MAX_JETONS as i64 + 5)
        .map(|i| (TraceVal::entier(i), TraceVal::entier(i)))
        .collect();
    let TraceVal::Carte(carte) = TraceVal::carte(paires) else {
        panic!("carte attendue")
    };
    assert_eq!(carte.len(), TRACE_VAL_MAX_JETONS + 1);
    assert_eq!(carte.last(), Some(&(TraceVal::Elide, TraceVal::Elide)));

    // Small collections are untouched.
    let petite = TraceVal::ensemble(vec![TraceVal::entier(1), TraceVal::entier(2)]);
    assert_eq!(petite.rendre(), "{1, 2}");
}

/// D1-3 DoD: elision — a lazy fragment is **not** computed until forced.
#[test]
fn test_trace_val_elision_paresseuse() {
    let compteur = Arc::new(AtomicU64::new(0));
    let compteur_clone = compteur.clone();
    let fragment = TraceVal::Fragment(Arc::new(move || {
        compteur_clone.fetch_add(1, Ordering::Relaxed);
        TraceVal::texte("calcule")
    }));

    // Not forced: the payload is not computed, equality is by allocation.
    assert_eq!(compteur.load(Ordering::Relaxed), 0);
    assert_eq!(fragment.clone(), fragment);
    assert_ne!(fragment, TraceVal::texte("calcule"));
    assert_eq!(compteur.load(Ordering::Relaxed), 0);

    // Forced: computed exactly once per force call.
    assert_eq!(fragment.rendre(), "calcule");
    assert_eq!(compteur.load(Ordering::Relaxed), 1);
    assert_eq!(fragment.force().rendre(), "calcule");
    assert_eq!(compteur.load(Ordering::Relaxed), 2);

    // Fragment chains are bounded in depth: beyond the bound, elision.
    let mut profond = TraceVal::entier(0);
    for _ in 0..TRACE_VAL_MAX_PROFONDEUR + 5 {
        let suivante = profond;
        profond = TraceVal::Fragment(Arc::new(move || suivante.clone()));
    }
    assert_eq!(profond.rendre(), "…");
}

/// D1-3: the `Traceable` default is the lazy Debug fallback; the H/S/M
/// types of the existing transductions have structured impls.
#[test]
fn test_traceable_impls() {
    // Debug fallback (lazy): a custom type without a specific impl.
    #[derive(Clone, Debug)]
    struct Quelconque(#[allow(dead_code)] u64);
    impl Traceable for Quelconque {}
    let v = Quelconque(7).trace_val();
    assert!(matches!(v, TraceVal::Fragment(_)));
    assert_eq!(v.rendre(), "Quelconque(7)");

    // H of lookup: Mode.
    assert_eq!(Mode::Cherche.trace_val().rendre(), "Cherche");
    assert_eq!(Mode::Extrait.trace_val().rendre(), "Extrait");
    assert_eq!(Mode::Stop(3).trace_val().rendre(), "{Stop: 3}");

    // H of exists/filter: KeyPresentes.
    let mut cles: KeyPresentes = BTreeSet::new();
    cles.insert(0);
    cles.insert(2);
    assert_eq!(cles.trace_val().rendre(), "{0, 2}");

    // S of pli: Naturel / BoolOu; H/S/M of image: unit elides.
    assert_eq!(Naturel(42).trace_val().rendre(), "42");
    assert_eq!(BoolOu(true).trace_val().rendre(), "1");
    assert_eq!(().trace_val().rendre(), "…");
    assert_eq!(true.trace_val().rendre(), "1");

    // S of lookup: fragment map.
    let mut fragment = BTreeMap::new();
    fragment.insert(1u64, NodeId(3));
    assert_eq!(fragment.trace_val().rendre(), "{1: 3}");

    // M of transfert: bool; H of transfert: EtatTransfert.
    assert_eq!(EtatTransfert::Normal.trace_val().rendre(), "Normal");
    assert_eq!(
        EtatTransfert::Transfert(1).trace_val().rendre(),
        "{Transfert: 1}"
    );

    // Memory writes: Vec<(NodeId, Level, M)>.
    let ecritures = vec![(NodeId(3), Level(1), true)];
    assert_eq!(ecritures.trace_val().rendre(), "{{3: 1, …: 1}}");

    // NodeId / Level / u64.
    assert_eq!(NodeId(9).trace_val().rendre(), "9");
    assert_eq!(Level(2).trace_val().rendre(), "2");
    assert_eq!(5u64.trace_val().rendre(), "5");
}
