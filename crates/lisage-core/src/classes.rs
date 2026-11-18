//! Per-class arenas (MEM-9, ADR-0103): `NodeId(u32)` = class bits + index
//! bits, routed to a per-class arena. Exact classes for the degrees
//! `d = 1..=C` (macro-generated table, one exact-stride slot per entry);
//! the catch-all class `2^b - 1` holds the degrees beyond the window —
//! their form carries the sorted run (`ArcRun`). Arenas are created
//! lazily, grown by doubling, in slabs chunked at 64 KiB. The class map
//! is frozen at pool construction (assertion).

use lisage_algebra::class_bits::{CLASS_BITS_MAX, class_bits_of};
use lisage_algebra::interface::{KeyAlgebra, NodeId};

use crate::biplan::NodeEntry;

/// Slab size: 64 KiB chunks.
pub const SLAB_SIZE: usize = 64 * 1024;

/// Maximum exact degree at the widest window: `C = 2^b_max - 2`.
pub const EXACT_MAX_DEGREE: u32 = (1 << CLASS_BITS_MAX) - 2;

/// Exact classes for the degrees `1..=C`, macro-generated (monomorphized):
/// the class of an exact degree `d` is `d` itself.
macro_rules! exact_class_table {
    ($($degree:literal),+ $(,)?) => {
        [$($degree),+]
    };
}

/// Table of the exact classes: `EXACT_CLASSES[d - 1] = d` for
/// `d = 1..=EXACT_MAX_DEGREE`.
pub const EXACT_CLASSES: [u32; EXACT_MAX_DEGREE as usize] = exact_class_table!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74,
    75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98,
    99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136,
    137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155,
    156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174,
    175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193,
    194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212,
    213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231,
    232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250,
    251, 252, 253, 254
);

/// Frozen class map, built once at pool creation and assertion-checked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClassMap {
    /// Effective class width `b`.
    pub class_bits: u32,
    /// Catch-all class: `2^b - 1`.
    pub catch_all: u32,
    /// Max exact degree `C = 2^b - 2`.
    pub exact_max: u32,
}

impl ClassMap {
    /// Build the class map from the key algebra: the width `b` is derived
    /// at runtime from `cardinality` over `number_levels` levels (MEM-8),
    /// honoring the declarative overload. Frozen: the window is asserted.
    pub fn new<Keys>(keys: &Keys, number_levels: u32) -> Self
    where
        Keys: KeyAlgebra,
    {
        let class_bits = class_bits_of(keys, number_levels);
        assert!(
            (1..=CLASS_BITS_MAX).contains(&class_bits),
            "class width {class_bits} out of the compiled window"
        );
        let catch_all = NodeId::catch_all_class(class_bits);
        ClassMap {
            class_bits,
            catch_all,
            exact_max: catch_all - 1,
        }
    }

    /// Number of classes: `2^b`.
    pub fn number_classes(&self) -> u32 {
        1 << self.class_bits
    }

    /// Class of a node of the given degree: exact class in `1..=C`,
    /// terminal class 0, catch-all class beyond the window.
    pub fn class_of_degree(&self, degree: u32) -> u32 {
        NodeId::class_of_degree(degree, self.class_bits)
    }
}

/// Per-class metrics: live and peak, traced by the pool.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ClassMetrics {
    /// Class identifier.
    pub class: u32,
    /// Live nodes in the class after the last GC.
    pub live: u64,
    /// Peak of live nodes in the class.
    pub peak: u64,
}

/// One class arena: created lazily, grown by doubling, slabs of
/// `SLAB_SIZE` bytes, each slot at the exact stride `size_of::<NodeEntry>()`.
pub(crate) struct ClassArena<Key, Payload> {
    slabs: Vec<Vec<Option<NodeEntry<Key, Payload>>>>,
    slab_capacity: usize,
    slots: usize,
    live: u64,
    peak: u64,
}

impl<Key, Payload> ClassArena<Key, Payload> {
    /// New empty arena; the slab capacity is the exact stride fill of a
    /// 64 KiB slab.
    pub(crate) fn new() -> Self {
        let slab_capacity = (SLAB_SIZE / size_of::<NodeEntry<Key, Payload>>()).max(1);
        ClassArena {
            slabs: Vec::new(),
            slab_capacity,
            slots: 0,
            live: 0,
            peak: 0,
        }
    }

    /// Slab capacity: entries per 64 KiB slab (exact stride).
    #[cfg(test)]
    pub(crate) fn slab_capacity(&self) -> usize {
        self.slab_capacity
    }

    /// Number of allocated slots (live + tombstones).
    /// Memory footprint of the arena in bytes: slots at the exact stride
    /// of `NodeEntry` (MEM-14 counter).
    pub(crate) fn bytes(&self) -> usize {
        self.slots * size_of::<NodeEntry<Key, Payload>>()
    }

    pub(crate) fn slot_count(&self) -> usize {
        self.slots
    }

    /// Number of live entries.
    pub(crate) fn live_count(&self) -> u64 {
        self.live
    }

    /// Push an entry, returns its index in the class arena. The arena is
    /// grown by doubling when all slabs are full.
    pub(crate) fn push(&mut self, entry: NodeEntry<Key, Payload>) -> u32 {
        let index = self.slots;
        let slab = index / self.slab_capacity;
        while self.slabs.len() <= slab {
            let added = self.slabs.len().max(1);
            for _ in 0..added {
                self.slabs
                    .push((0..self.slab_capacity).map(|_| None).collect());
            }
        }
        self.slabs[slab][index % self.slab_capacity] = Some(entry);
        self.slots += 1;
        self.live += 1;
        self.peak = self.peak.max(self.live);
        index as u32
    }

    /// Entry at `index`, if the slot exists.
    pub(crate) fn get(&self, index: u32) -> Option<&NodeEntry<Key, Payload>> {
        self.slabs
            .get(index as usize / self.slab_capacity)?
            .get(index as usize % self.slab_capacity)?
            .as_ref()
    }

    /// Mutable entry at `index`, if the slot exists.
    pub(crate) fn get_mut(&mut self, index: u32) -> Option<&mut NodeEntry<Key, Payload>> {
        self.slabs
            .get_mut(index as usize / self.slab_capacity)?
            .get_mut(index as usize % self.slab_capacity)?
            .as_mut()
    }

    /// Empty marking bitmap for this arena: 1 bit per slot (MEM-10).
    pub(crate) fn empty_marks(&self) -> Vec<u64> {
        vec![0u64; self.slots / 64 + 1]
    }

    /// Set the mark bit of `index`.
    pub(crate) fn set_mark(marks: &mut [u64], index: u32) {
        marks[index as usize / 64] |= 1u64 << (index % 64);
    }

    /// Mark bit of `index` set?
    pub(crate) fn mark_of(marks: &[u64], index: u32) -> bool {
        marks
            .get(index as usize / 64)
            .is_some_and(|word| word & (1u64 << (index % 64)) != 0)
    }

    /// Tombstone the unmarked slots (form set to `None`), returns the
    /// number of collected slots. Marking: 1 bit per node (MEM-10).
    pub(crate) fn sweep(&mut self, marks: &[u64]) -> u64 {
        let mut collected = 0;
        for index in 0..self.slots as u32 {
            if let Some(entry) = self.get_mut(index)
                && entry.form.is_some()
                && !Self::mark_of(marks, index)
            {
                entry.form = None;
                collected += 1;
            }
        }
        self.live -= collected;
        collected
    }

    /// Release the pages of an emptied class: the slabs are freed but the
    /// slot count stays as high-water mark — no index is recycled outside
    /// the compaction remap protocol (MEM-10, ADR-0103 invariant 3).
    pub(crate) fn release_pages(&mut self) {
        self.slabs.clear();
        self.live = 0;
    }

    /// Compact: remove the tombstones, returns the index remap
    /// (`remap[old] = Some(new)` or `None` for a tombstone).
    pub(crate) fn compact(&mut self) -> Vec<Option<u32>> {
        let mut remap: Vec<Option<u32>> = Vec::with_capacity(self.slots);
        let mut kept: Vec<Option<NodeEntry<Key, Payload>>> = Vec::new();
        let mut seen = 0usize;
        for slab in self.slabs.drain(..) {
            for slot in slab {
                if seen >= self.slots {
                    break;
                }
                seen += 1;
                // A tombstone is a slot whose form is `None`.
                match slot {
                    Some(entry) if entry.form.is_some() => {
                        remap.push(Some(kept.len() as u32));
                        kept.push(Some(entry));
                    }
                    _ => remap.push(None),
                }
            }
        }
        self.slabs.clear();
        self.slots = kept.len();
        for (index, entry) in kept.into_iter().enumerate() {
            let slab = index / self.slab_capacity;
            while self.slabs.len() <= slab {
                self.slabs
                    .push((0..self.slab_capacity).map(|_| None).collect());
            }
            self.slabs[slab][index % self.slab_capacity] = entry;
        }
        self.live = self.slots as u64;
        remap
    }

    /// Live and peak of the class.
    pub(crate) fn metrics(&self, class: u32) -> ClassMetrics {
        ClassMetrics {
            class,
            live: self.live,
            peak: self.peak,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Static test: the slab is filled at the exact stride —
    /// `slab_capacity` entries fit in 64 KiB, one more does not.
    #[test]
    fn test_slab_exact_stride() {
        let arena: ClassArena<u64, bool> = ClassArena::new();
        let entry = size_of::<NodeEntry<u64, bool>>();
        assert!(arena.slab_capacity() * entry <= SLAB_SIZE);
        assert!((arena.slab_capacity() + 1) * entry > SLAB_SIZE);
    }

    /// The macro-generated exact-class table: class of degree d is d.
    #[test]
    fn test_exact_class_table() {
        assert_eq!(EXACT_CLASSES.len(), EXACT_MAX_DEGREE as usize);
        for (index, class) in EXACT_CLASSES.iter().enumerate() {
            assert_eq!(*class, index as u32 + 1);
        }
    }
}
