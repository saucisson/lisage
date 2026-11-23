//! Unique table without form clone (MEM-14, ADR-0103): open addressing of
//! `(hash, anchor, NodeId)` records at load factor 0.5 (~24 bytes per node
//! at that factor — 12-byte slots thanks to the `NonZero<u32>` hash niche,
//! ALIGN-1; 32 before — against the cloned-form `HashMap` key before). The
//! hash-consing key is still the semantic form: equality is decided through
//! the stored node's entry (`same_form` callback), never by cloning the form
//! into the key. `ControllerId` is dropped from the key — it is constant
//! per pool (`Arena<C>` is per-controller).

use lisage_algebra::interface::{Anchor, Form, NodeId};
use std::hash::{Hash, Hasher};
use std::num::NonZero;

/// Load factor ceiling: the table grows when `len / capacity > 1/2`.
const LOAD_NUM: usize = 1;
const LOAD_DEN: usize = 2;

/// One occupied slot: truncated hash of the key (32 bits — equality is
/// always confirmed through the stored form, so a collision only costs a
/// probe), anchor, and the interned node. 12 bytes per slot.
///
/// The hash tag is `NonZero<u32>` (ALIGN-1): the zero niche makes
/// `Option<Slot>` 12 bytes instead of 16 (tag + padding before). A
/// truncated hash of 0 is remapped to 1 by [`slot_tag`] — safe because the
/// tag only steers bucket placement, never equality.
#[derive(Clone, Copy, Debug)]
struct Slot {
    hash: NonZero<u32>,
    anchor: Anchor,
    node: NodeId,
}

/// Truncate the 64-bit key hash to the 32-bit slot tag, remapping 0 to 1
/// so the tag stays not-zero (ALIGN-1 niche). Documented invariant: the tag
/// is used for bucket placement and cheap pre-filtering only; equality is
/// always confirmed semantically through the stored node's form.
#[inline]
fn slot_tag(hash: u64) -> NonZero<u32> {
    NonZero::new(hash as u32).unwrap_or(NonZero::<u32>::MIN)
}

/// Open-addressing unique table (linear probing, power-of-two capacity).
pub(crate) struct TableUnique {
    slots: Vec<Option<Slot>>,
    len: usize,
}

impl TableUnique {
    /// New empty table.
    pub(crate) fn new() -> Self {
        TableUnique {
            slots: Vec::new(),
            len: 0,
        }
    }

    /// Number of interned keys.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Memory footprint of the table in bytes (MEM-14 counter).
    pub(crate) fn bytes(&self) -> usize {
        self.slots.capacity() * size_of::<Option<Slot>>()
    }

    /// Grow to a power-of-two capacity keeping the load factor ≤ 1/2.
    fn grow(&mut self) {
        let new_capacity = if self.slots.is_empty() {
            8
        } else {
            // Load factor ≤ 1/2: capacity ≥ 2 × (len + 1), power of two.
            ((self.len + 1) * LOAD_DEN / LOAD_NUM)
                .max(8)
                .next_power_of_two()
        };
        let mut new_slots: Vec<Option<Slot>> = Vec::with_capacity(new_capacity);
        new_slots.resize_with(new_capacity, || None);
        for slot in self.slots.iter().flatten() {
            let mut index = (slot.hash.get() as usize) & (new_capacity - 1);
            while new_slots[index].is_some() {
                index = (index + 1) & (new_capacity - 1);
            }
            new_slots[index] = Some(*slot);
        }
        self.slots = new_slots;
    }

    /// Look up the node of `(hash, anchor)` whose semantic form satisfies
    /// `same_form` — the form itself is never cloned into the table.
    pub(crate) fn get(
        &self,
        hash: u64,
        anchor: Anchor,
        same_form: impl Fn(NodeId) -> bool,
    ) -> Option<NodeId> {
        if self.slots.is_empty() {
            return None;
        }
        let tag = slot_tag(hash);
        let mask = self.slots.len() - 1;
        let mut index = (tag.get() as usize) & mask;
        loop {
            let slot = self.slots[index]?;
            if slot.hash == tag && slot.anchor == anchor && same_form(slot.node) {
                return Some(slot.node);
            }
            index = (index + 1) & mask;
        }
    }

    /// Insert a `(hash, anchor, node)` record, growing at load factor 1/2.
    pub(crate) fn insert(&mut self, hash: u64, anchor: Anchor, node: NodeId) {
        if (self.len + 1) * LOAD_DEN > self.slots.len() * LOAD_NUM {
            self.grow();
        }
        let tag = slot_tag(hash);
        let mask = self.slots.len() - 1;
        let mut index = (tag.get() as usize) & mask;
        while let Some(slot) = self.slots[index] {
            if slot.hash == tag && slot.anchor == anchor && slot.node == node {
                return;
            }
            index = (index + 1) & mask;
        }
        self.slots[index] = Some(Slot {
            hash: tag,
            anchor,
            node,
        });
        self.len += 1;
    }

    /// Drop the records whose node fails `keep` (GC sweep of the table).
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(NodeId) -> bool) {
        for slot in &mut self.slots {
            if let Some(record) = slot
                && !keep(record.node)
            {
                *slot = None;
                self.len -= 1;
            }
        }
    }

    /// Mutable iteration over the interned nodes (compaction remap).
    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut NodeId> {
        self.slots
            .iter_mut()
            .filter_map(|slot| slot.as_mut().map(|record| &mut record.node))
    }
}

/// Fast deterministic hasher (PERF-3): FxHash-style rotate-xor-multiply
/// over 64-bit words with a fixed seed, replacing SipHash
/// (`DefaultHasher`) on the intern path — SipHash's constant cost on the
/// small node forms dominated the remaining build time. The unique table
/// uses the hash only for bucket placement (equality stays semantic via
/// `same_form`), so any deterministic hasher is correct; the fixed seed
/// and little-endian word reads keep measurements reproducible across
/// runs and machines (MEM-18 seeded workload).
struct FastHasher {
    hash: u64,
}

/// Fixed seed: the golden-ratio constant.
const FAST_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
/// FxHash multiplier constant.
const FAST_K: u64 = 0x517C_C1B7_2722_0A95;

impl FastHasher {
    fn new() -> Self {
        FastHasher { hash: FAST_SEED }
    }

    #[inline]
    fn add_word(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(FAST_K);
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let (words, tail) = bytes.as_chunks::<8>();
        for word in words {
            self.add_word(u64::from_le_bytes(*word));
        }
        if !tail.is_empty() {
            let mut word = [0u8; 8];
            word[..tail.len()].copy_from_slice(tail);
            self.add_word(u64::from_le_bytes(word));
        }
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add_word(u64::from(value));
    }

    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.add_word(u64::from(value));
    }

    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.add_word(u64::from(value));
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.add_word(value);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add_word(value as u64);
    }
}

/// Hash of the hash-consing key `(anchor, form)` — the semantic form,
/// hashed in place, never cloned.
pub(crate) fn hash_key<Key, Payload>(anchor: Anchor, form: &Form<Key, Payload>) -> u64
where
    Key: Hash,
    Payload: Hash,
{
    let mut hasher = FastHasher::new();
    anchor.hash(&mut hasher);
    form.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lisage_algebra::interface::{ArcRun, Level};

    /// Golden `hash_key` value for the fixed form of the test below.
    const GOLDEN_HASH_KEY: u64 = 3_561_021_471_050_673_137;

    /// ALIGN-1 witness: the `NonZero<u32>` hash tag gives `Option<Slot>` a
    /// niche, so the occupied/empty discriminant costs nothing and
    /// `Option<Slot>` is 12 bytes (was 16 at ALIGN-0 — tag + padding).
    #[test]
    fn test_align1_option_slot_is_12_bytes() {
        assert_eq!(
            size_of::<Slot>(),
            12,
            "Slot: hash NonZero<u32> + anchor + NodeId"
        );
        assert_eq!(
            size_of::<Option<Slot>>(),
            12,
            "ALIGN-1: NonZero<u32> niche, Option<Slot> = 12 bytes"
        );
    }

    /// ALIGN-1 invariant: a truncated hash of 0 is remapped to 1 so the tag
    /// stays not-zero; placement/equality semantics are unchanged because
    /// the tag only steers probing.
    #[test]
    fn test_align1_slot_tag_remaps_zero() {
        assert_eq!(slot_tag(0).get(), 1);
        assert_eq!(slot_tag(0x1_0000_0000).get(), 1, "low 32 bits are 0");
        assert_eq!(slot_tag(42).get(), 42);
        assert_eq!(
            slot_tag(0x1_0000_002A).get(),
            42,
            "only the low 32 bits are kept"
        );
    }

    /// PERF-3 (issue #284): golden value pinning `hash_key` — the
    /// `FastHasher` must stay deterministic across runs and machines
    /// (MEM-18 seeded-workload reproducibility); any change to the hash
    /// algorithm fails this test.
    #[test]
    fn test_hash_key_golden_value() {
        let form: Form<u64, bool> = Form::Node {
            level: Level(3),
            arcs: ArcRun::from_pairs([(0, NodeId(11)), (1, NodeId(12))]),
        };
        assert_eq!(
            hash_key(Anchor(0), &form),
            GOLDEN_HASH_KEY,
            "golden hash_key value (PERF-3 FastHasher)"
        );
        // Same key, different anchor: the anchor is part of the key.
        assert_ne!(hash_key(Anchor(1), &form), hash_key(Anchor(0), &form));
    }
}
