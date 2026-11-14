//! PBT of the level memory (M2-3, T3): versioning invariants —
//! `version == number of writes`, read-after-write keyed by
//! `(NodeId, Level)`, and `size` consistent.

use lisage_algebra::interface::{Level, NodeId};
use lisage_core::memory::MemoryLevel;
use proptest::prelude::*;

proptest! {
    /// version(M) == number of writes; read-after-write; consistent size.
    #[test]
    fn test_memory_level_invariants(
        ecritures in prop::collection::vec(
            (0u32..10, 0u32..5, 0u64..100),
            0..20
        )
    ) {
        let mut memory: MemoryLevel<u64> = MemoryLevel::new();
        prop_assert_eq!(memory.version(), 0);
        let mut attendu: std::collections::HashMap<(NodeId, Level), u64> =
            std::collections::HashMap::new();
        for (i, (node, level, value)) in ecritures.iter().enumerate() {
            let ancien = memory.ecrire(NodeId(*node), Level(*level), *value);
            prop_assert_eq!(memory.version(), (i + 1) as u64, "version == number of writes");
            prop_assert_eq!(ancien, attendu.insert((NodeId(*node), Level(*level)), *value));
        }
        prop_assert_eq!(memory.size(), attendu.len());
        // Read-after-write: each key reads the last written value.
        for ((node, level), value) in &attendu {
            prop_assert_eq!(memory.lire(*node, *level), Some(value));
        }
    }
}
