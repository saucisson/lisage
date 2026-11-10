//! Charte de test (M0-4) : triptyque A/B/C, interprète de référence comme
//! source de vérité, convention de nommage `test_c<n>_*` par énoncé.
//!
//! Test différentiel factice : les trois voies doivent agree sur un exemple.

use lisage_bench::{voie_a, voie_b, voie_c};
use proptest::prelude::*;

proptest! {
    /// C0 (factice) : les trois voies s'accordent sur n'importe quel entier.
    #[test]
    fn test_c0_triptyque_accord(entier in 0u32..1000) {
        let a = voie_a(entier);
        let b = voie_b(entier);
        let c = voie_c(entier);
        prop_assert_eq!(a, b, "voie A (référence) != voie B");
        prop_assert_eq!(a, c, "voie A (référence) != voie C");
    }
}

#[test]
fn test_c0_exemple_factice() {
    assert_eq!(voie_a(41), 42);
    assert_eq!(voie_b(41), voie_a(41));
    assert_eq!(voie_c(41), voie_a(41));
}
