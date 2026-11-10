//! PBT de la mémoire de niveau (M2-3, T3) : invariants de versioning —
//! `version == nombre d'écritures`, read-after-write keyed by
//! `(NodeId, Level)`, et `taille` cohérent.

use lisage_algebra::interface::{Level, NodeId};
use lisage_core::memoire::MemoireNiveau;
use proptest::prelude::*;

proptest! {
    /// version(M) == nombre d'écritures ; read-after-write ; taille cohérent.
    #[test]
    fn test_memoire_niveau_invariants(
        ecritures in prop::collection::vec(
            (0u32..10, 0u32..5, 0u64..100),
            0..20
        )
    ) {
        let mut memoire: MemoireNiveau<u64> = MemoireNiveau::new();
        prop_assert_eq!(memoire.version(), 0);
        let mut attendu: std::collections::HashMap<(NodeId, Level), u64> =
            std::collections::HashMap::new();
        for (i, (noeud, niveau, valeur)) in ecritures.iter().enumerate() {
            let ancien = memoire.ecrire(NodeId(*noeud), Level(*niveau), *valeur);
            prop_assert_eq!(memoire.version(), (i + 1) as u64, "version == nombre d'écritures");
            prop_assert_eq!(ancien, attendu.insert((NodeId(*noeud), Level(*niveau)), *valeur));
        }
        prop_assert_eq!(memoire.taille(), attendu.len());
        // Read-after-write : chaque clé lit la dernière valeur écrite.
        for ((noeud, niveau), valeur) in &attendu {
            prop_assert_eq!(memoire.lire(*noeud, *niveau), Some(valeur));
        }
    }
}
