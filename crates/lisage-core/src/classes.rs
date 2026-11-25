//! Per-class arenas (MEM-9, ADR-0103): `NodeId(u32)` = class bits + index
//! bits, routed to a per-class arena. Exact classes for the degrees
//! `d = 1..=C` (macro-generated table, one exact-stride slot per entry);
//! the catch-all class `2^b - 1` holds the degrees beyond the window and
//! the open-domain levels — their form carries the sorted run (`ArcRun`).
//!
//! MEM-16 (ADR-0104): the exact classes store the arcs **inline** in a
//! monomorphic entry `InlineEntry<D>` at the exact stride
//! `HEADER + 8·D` (one `(u32, NodeId)` per arc: the key narrowed to its
//! 32-bit storage form), instead of a boxed run behind a `Form`. The
//! catch-all class and the terminal class keep the `NodeEntry` with the
//! full `Form` — the key/value run path is unchanged on open domains.
//! Arenas are created lazily, grown by doubling, in slabs chunked at
//! 64 KiB. The class map is frozen at pool construction (assertion).

use lisage_algebra::class_bits::{CLASS_BITS_MAX, class_bits_of};
use lisage_algebra::interface::{ArcRun, Form, KeyAlgebra, Level, NodeId};

use crate::biplan::{NodeEntry, Plan};

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
    /// Slack: tombstone slots kept in the class arena (slots − live).
    pub slack: u64,
}

/// Inline entry of an exact class (MEM-16): the arcs of a degree-`D` node
/// stored inline, one `(u32, NodeId)` per arc — the key in its narrowed
/// 32-bit storage form plus the target. Slot stride: `HEADER + 8·D`.
pub(crate) struct InlineEntry<const D: usize> {
    /// Plane of the node.
    pub plan: Plan,
    /// Live flag: `false` marks a tombstone.
    pub live: bool,
    /// Level of the node.
    pub level: Level,
    /// Inline arcs: narrowed key + target, 8 bytes each.
    pub arcs: [(u32, NodeId); D],
}

/// Header size of an inline entry: `plan` + `live` + `level`, padded.
#[cfg(test)]
pub(crate) const INLINE_HEADER: usize = size_of::<InlineEntry<0>>();

/// Liveness of a class-arena entry: the tombstone protocol (MEM-10) is
/// entry-kind agnostic.
pub(crate) trait EntryLive {
    /// The entry holds a live node.
    fn is_live(&self) -> bool;
    /// Tombstone the entry.
    fn tombstone(&mut self);
}

/// Contract of a class-arena entry (MEM-16): the two entry kinds —
/// `NodeEntry` (terminal class 0 and catch-all: the full `Form` stored,
/// key/value run unchanged) and `InlineEntry<D>` (exact class `D`: arcs
/// inline, keys narrowed) — behind the same object-safe arena trait.
pub(crate) trait EntrySpec<A: KeyAlgebra, Payload>: EntryLive + Sized + Send + Sync
where
    Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync,
{
    /// Build an entry from a form; `None` when the form is not
    /// representable in this entry kind (an exact entry whose keys do
    /// not narrow, or a degree mismatch).
    fn from_form(plan: Plan, form: Form<A::Key, Payload>, keys: &A) -> Option<Self>;
    /// Plane of the node.
    fn plan(&self) -> Plan;
    /// Move the node to another plane.
    fn set_plan(&mut self, plan: Plan);
    /// Targets of the arcs, for GC marking.
    fn targets(&self) -> Vec<NodeId>;
    /// Semantic comparison against a form (unique table): the stored
    /// narrowed keys are widened back, the form is never cloned.
    fn same_form(&self, form: &Form<A::Key, Payload>, keys: &A) -> bool;
    /// Owned form reconstruction (`Pool::resolve`).
    fn to_form(&self, keys: &A) -> Option<Form<A::Key, Payload>>;
}

impl<Key, Payload> EntryLive for NodeEntry<Key, Payload> {
    fn is_live(&self) -> bool {
        self.form.is_some()
    }

    fn tombstone(&mut self) {
        self.form = None;
    }
}

impl<A: KeyAlgebra, Payload> EntrySpec<A, Payload> for NodeEntry<A::Key, Payload>
where
    Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync,
{
    fn from_form(plan: Plan, form: Form<A::Key, Payload>, _keys: &A) -> Option<Self> {
        Some(NodeEntry {
            plan,
            form: Some(form),
        })
    }

    fn plan(&self) -> Plan {
        self.plan
    }

    fn set_plan(&mut self, plan: Plan) {
        self.plan = plan;
    }

    fn targets(&self) -> Vec<NodeId> {
        match &self.form {
            Some(Form::Node { arcs, .. }) => arcs.values().copied().collect(),
            _ => Vec::new(),
        }
    }

    fn same_form(&self, form: &Form<A::Key, Payload>, _keys: &A) -> bool {
        self.form.as_ref().is_some_and(|stored| stored == form)
    }

    fn to_form(&self, _keys: &A) -> Option<Form<A::Key, Payload>> {
        self.form.clone()
    }
}

impl<const D: usize> EntryLive for InlineEntry<D> {
    fn is_live(&self) -> bool {
        self.live
    }

    fn tombstone(&mut self) {
        self.live = false;
    }
}

impl<A: KeyAlgebra, Payload, const D: usize> EntrySpec<A, Payload> for InlineEntry<D>
where
    Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync,
{
    fn from_form(plan: Plan, form: Form<A::Key, Payload>, keys: &A) -> Option<Self> {
        let Form::Node { level, arcs } = form else {
            return None;
        };
        if arcs.len() != D {
            return None;
        }
        let mut narrowed = [(0u32, NodeId(0)); D];
        for (slot, (key, target)) in narrowed.iter_mut().zip(arcs.iter()) {
            *slot = (keys.narrow(level, key)?, *target);
        }
        Some(InlineEntry {
            plan,
            live: true,
            level,
            arcs: narrowed,
        })
    }

    fn plan(&self) -> Plan {
        self.plan
    }

    fn set_plan(&mut self, plan: Plan) {
        self.plan = plan;
    }

    fn targets(&self) -> Vec<NodeId> {
        self.arcs.iter().map(|(_, target)| *target).collect()
    }

    fn same_form(&self, form: &Form<A::Key, Payload>, keys: &A) -> bool {
        let Form::Node { level, arcs } = form else {
            return false;
        };
        if *level != self.level || arcs.len() != D {
            return false;
        }
        arcs.iter()
            .zip(self.arcs.iter())
            .all(|((key, target), (narrowed, stored_target))| {
                *target == *stored_target && keys.widen(self.level, *narrowed).as_ref() == Some(key)
            })
    }

    fn to_form(&self, keys: &A) -> Option<Form<A::Key, Payload>> {
        if !self.live {
            return None;
        }
        let mut arcs: Vec<(A::Key, NodeId)> = Vec::with_capacity(D);
        for (narrowed, target) in &self.arcs {
            arcs.push((keys.widen(self.level, *narrowed)?, *target));
        }
        // The resolved form is canonical: the dense-family choice (MEM-12)
        // is reapplied on the rebuilt run, exactly as if the form had been
        // stored boxed — the inline storage is a memory layout, not a
        // semantic change.
        let arcs = ArcRun::choose_dense_family(self.level, keys, ArcRun::from_pairs(arcs));
        Some(Form::Node {
            level: self.level,
            arcs,
        })
    }
}

/// Object-safe per-class arena (MEM-16): one implementation per entry
/// kind behind the same trait, so the pool routes a `NodeId` to its class
/// without knowing the entry type.
pub(crate) trait ClassArenaDyn<A: KeyAlgebra + 'static, Payload>: Send + Sync
where
    Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static,
{
    /// Memory footprint of the arena in bytes (exact entry stride).
    fn bytes(&self) -> usize;
    /// Number of allocated slots (live + tombstones).
    fn slot_count(&self) -> usize;
    /// Number of live entries.
    fn live_count(&self) -> u64;
    /// Live, peak and slack of the class.
    fn metrics(&self, class: u32) -> ClassMetrics;
    /// Empty marking bitmap for this arena: 1 bit per slot (MEM-10).
    fn empty_marks(&self) -> Vec<u64>;
    /// Tombstone the unmarked slots, returns the number collected.
    fn sweep(&mut self, marks: &[u64]) -> u64;
    /// Release the pages of an emptied class (high-water mark kept).
    fn release_pages(&mut self);
    /// Compact: remove the tombstones, returns the index remap.
    fn compact(&mut self) -> Vec<Option<u32>>;
    /// The slot holds a live node.
    fn is_live(&self, index: u32) -> bool;
    /// Plane of a live node.
    fn plan_of(&self, index: u32) -> Option<Plan>;
    /// Move a live node to another plane.
    fn set_plan(&mut self, index: u32, plan: Plan);
    /// Push a form, returns its index; `None` when the form is not
    /// representable in this arena (exact arena, key not narrowable).
    fn push_form(&mut self, plan: Plan, form: Form<A::Key, Payload>, keys: &A) -> Option<u32>;
    /// Targets of a live slot, for GC marking.
    fn targets(&self, index: u32) -> Vec<NodeId>;
    /// Semantic comparison of a live slot against a form (unique table).
    fn same_form(&self, index: u32, form: &Form<A::Key, Payload>, keys: &A) -> bool;
    /// Owned form of a live slot (`Pool::resolve`).
    fn to_form(&self, index: u32, keys: &A) -> Option<Form<A::Key, Payload>>;
}

/// One class arena: created lazily, grown by doubling, slabs of
/// `SLAB_SIZE` bytes, each slot at the exact stride of the entry type.
pub(crate) struct ClassArena<E> {
    slabs: Vec<Vec<Option<E>>>,
    slab_capacity: usize,
    slots: usize,
    live: u64,
    peak: u64,
}

impl<E> ClassArena<E> {
    /// New empty arena; the slab capacity is the exact stride fill of a
    /// 64 KiB slab.
    pub(crate) fn new() -> Self {
        let slab_capacity = (SLAB_SIZE / size_of::<E>()).max(1);
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

    /// Memory footprint of the arena in bytes: slots at the exact stride
    /// of the entry type (MEM-14 counter).
    pub(crate) fn bytes(&self) -> usize {
        self.slots * size_of::<E>()
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
    pub(crate) fn push(&mut self, entry: E) -> u32 {
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
    pub(crate) fn get(&self, index: u32) -> Option<&E> {
        self.slabs
            .get(index as usize / self.slab_capacity)?
            .get(index as usize % self.slab_capacity)?
            .as_ref()
    }

    /// Mutable entry at `index`, if the slot exists.
    pub(crate) fn get_mut(&mut self, index: u32) -> Option<&mut E> {
        self.slabs
            .get_mut(index as usize / self.slab_capacity)?
            .get_mut(index as usize % self.slab_capacity)?
            .as_mut()
    }

    /// Empty marking bitmap for this arena: 1 bit per slot (MEM-10).
    pub(crate) fn empty_marks(&self) -> Vec<u64> {
        vec![0u64; self.slots / 64 + 1]
    }
}

impl<E: EntryLive> ClassArena<E> {
    /// Tombstone the unmarked live slots, returns the number of
    /// collected slots. Marking: 1 bit per node (MEM-10).
    pub(crate) fn sweep(&mut self, marks: &[u64]) -> u64 {
        let mut collected = 0;
        for index in 0..self.slots as u32 {
            if let Some(entry) = self.get_mut(index)
                && entry.is_live()
                && !mark_of(marks, index)
            {
                entry.tombstone();
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
        let mut kept: Vec<Option<E>> = Vec::new();
        let mut seen = 0usize;
        for slab in self.slabs.drain(..) {
            for slot in slab {
                if seen >= self.slots {
                    break;
                }
                seen += 1;
                // A tombstone is a slot whose entry is not live.
                match slot {
                    Some(entry) if entry.is_live() => {
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
            slack: self.slots.saturating_sub(self.live as usize) as u64,
        }
    }
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

impl<A: KeyAlgebra + 'static, Payload, E> ClassArenaDyn<A, Payload> for ClassArena<E>
where
    Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static,
    E: EntrySpec<A, Payload>,
{
    fn bytes(&self) -> usize {
        self.bytes()
    }

    fn slot_count(&self) -> usize {
        self.slot_count()
    }

    fn live_count(&self) -> u64 {
        self.live_count()
    }

    fn metrics(&self, class: u32) -> ClassMetrics {
        self.metrics(class)
    }

    fn empty_marks(&self) -> Vec<u64> {
        self.empty_marks()
    }

    fn sweep(&mut self, marks: &[u64]) -> u64 {
        self.sweep(marks)
    }

    fn release_pages(&mut self) {
        self.release_pages()
    }

    fn compact(&mut self) -> Vec<Option<u32>> {
        self.compact()
    }

    fn is_live(&self, index: u32) -> bool {
        self.get(index).is_some_and(|entry| entry.is_live())
    }

    fn plan_of(&self, index: u32) -> Option<Plan> {
        self.get(index)
            .filter(|entry| entry.is_live())
            .map(|entry| entry.plan())
    }

    fn set_plan(&mut self, index: u32, plan: Plan) {
        if let Some(entry) = self.get_mut(index)
            && entry.is_live()
        {
            entry.set_plan(plan);
        }
    }

    fn push_form(&mut self, plan: Plan, form: Form<A::Key, Payload>, keys: &A) -> Option<u32> {
        let entry = E::from_form(plan, form, keys)?;
        Some(self.push(entry))
    }

    fn targets(&self, index: u32) -> Vec<NodeId> {
        self.get(index)
            .filter(|entry| entry.is_live())
            .map(|entry| entry.targets())
            .unwrap_or_default()
    }

    fn same_form(&self, index: u32, form: &Form<A::Key, Payload>, keys: &A) -> bool {
        self.get(index)
            .is_some_and(|entry| entry.is_live() && entry.same_form(form, keys))
    }

    fn to_form(&self, index: u32, keys: &A) -> Option<Form<A::Key, Payload>> {
        self.get(index)
            .filter(|entry| entry.is_live())
            .and_then(|entry| entry.to_form(keys))
    }
}

/// Build the arena of a class (MEM-16): terminal class 0 and catch-all
/// keep the `NodeEntry` (full form, key/value run unchanged); an exact
/// class `d` gets the monomorphic inline arena `InlineEntry<d>`.
pub(crate) fn new_arena<A: KeyAlgebra + 'static, Payload>(
    class: u32,
    map: &ClassMap,
) -> Box<dyn ClassArenaDyn<A, Payload>>
where
    Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static,
{
    if class == 0 || class == map.catch_all {
        Box::new(ClassArena::<NodeEntry<A::Key, Payload>>::new())
    } else {
        new_exact_arena::<A, Payload>(class)
    }
}

/// Macro-generated dispatch over the exact degrees: one monomorphic
/// inline arena per degree `1..=EXACT_MAX_DEGREE`.
macro_rules! exact_arena_dispatch {
    ($($degree:literal),+ $(,)?) => {
        pub(crate) fn new_exact_arena<A: KeyAlgebra + 'static, Payload>(
            degree: u32,
        ) -> Box<dyn ClassArenaDyn<A, Payload>>
        where
            Payload: Clone + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static,
        {
            match degree {
                $($degree => Box::new(ClassArena::<InlineEntry<$degree>>::new()),)+
                _ => unreachable!("exact degree within the frozen window"),
            }
        }
    };
}

exact_arena_dispatch!(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Static test (MEM-16): the inline entry of an exact class `d` is
    /// exactly `HEADER + 8·d` bytes — the stride criterion of the issue.
    #[test]
    fn test_inline_entry_stride_8d_plus_header() {
        assert_eq!(INLINE_HEADER, 8, "header: plan + live + level, padded");
        macro_rules! check_stride {
            ($($d:literal),+) => {
                $(assert_eq!(
                    size_of::<InlineEntry<$d>>(),
                    INLINE_HEADER + 8 * $d,
                    "stride of exact class {}",
                    $d
                );)+
            };
        }
        check_stride!(1, 2, 3, 4, 5, 6, 7, 8, 16, 64, 254);
        // The arc slot itself is 8 bytes: narrowed u32 key + NodeId.
        assert_eq!(size_of::<(u32, NodeId)>(), 8);
    }

    /// Static test: the slab is filled at the exact stride —
    /// `slab_capacity` entries fit in 64 KiB, one more does not.
    #[test]
    fn test_slab_exact_stride() {
        let arena: ClassArena<NodeEntry<u64, bool>> = ClassArena::new();
        let entry = size_of::<NodeEntry<u64, bool>>();
        assert!(arena.slab_capacity() * entry <= SLAB_SIZE);
        assert!((arena.slab_capacity() + 1) * entry > SLAB_SIZE);
        let inline: ClassArena<InlineEntry<2>> = ClassArena::new();
        let entry = size_of::<InlineEntry<2>>();
        assert!(inline.slab_capacity() * entry <= SLAB_SIZE);
        assert!((inline.slab_capacity() + 1) * entry > SLAB_SIZE);
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
