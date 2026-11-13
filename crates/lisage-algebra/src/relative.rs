//! Relative forms (M4-2, statement C8, spec § 2.4.5): activation of the
//! **shift** (relative encoding carried by the anchor), **composed anchors**
//! (offsets named per fragment) and **key transformations** designated by the
//! controller (action group + canonical section of orbits).
//!
//! One single idea: the form is relative, the context carries the absolute.
//! `algebra/` side only — the pool stays blind and unchanged (C1): the
//! anchor (position) lives in the handle, the transformation term (value) is
//! an argument on the input edge, carried by the caller.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

use crate::interface::{Anchor, Handle, NodeId};

/// Named fragment of a factorized network: the composed anchor carries one
/// offset per fragment. Canonical order: lexicographic of the names.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Fragment(pub String);

/// Composed anchor (spec § 2.4.5): named record of offsets, one per fragment
/// (`{machine : +i, conveyor : +j}`). Zero = start of fragment. Stays in the
/// handle (C1 unchanged), the pool stays blind.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct AnchorComposed {
    decalages: BTreeMap<Fragment, i64>,
}

impl AnchorComposed {
    /// Null anchor: all fragments at the start (zero).
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds an anchor with the given offset for a fragment.
    pub fn with(mut self, fragment: Fragment, offset: i64) -> Self {
        self.decalages.insert(fragment, offset);
        self
    }

    /// Offset of a fragment; 0 if absent (zero = start of fragment).
    pub fn offset(&self, fragment: &Fragment) -> i64 {
        self.decalages.get(fragment).copied().unwrap_or(0)
    }

    /// New anchor, one fragment shifted by `delta` (the slide).
    pub fn decalee(&self, fragment: &Fragment, delta: i64) -> Self {
        let mut result = self.clone();
        let current = result.offset(fragment);
        result.decalages.insert(fragment.clone(), current + delta);
        result
    }

    /// Pointwise composition of offsets (absolute read = composition of
    /// arguments, O(1) per step).
    pub fn composed(&self, other: &AnchorComposed) -> Self {
        let mut result = self.clone();
        for (fragment, offset) in &other.decalages {
            let current = result.offset(fragment);
            result.decalages.insert(fragment.clone(), current + *offset);
        }
        result
    }

    /// True if all offsets are zero (start of each fragment).
    pub fn is_null(&self) -> bool {
        self.decalages.values().all(|&d| d == 0)
    }

    /// Iterator over (fragment, offset), lexicographic order of the names.
    pub fn fragments(&self) -> impl Iterator<Item = (&Fragment, i64)> {
        self.decalages.iter().map(|(f, &d)| (f, d))
    }
}

/// Relative anchor: activation of the shift (relative encoding). The
/// absolute position of the root reads `position + offset`; the offset is
/// the accumulated argument (shift +1 per cell, the anchor bounds the
/// unfolding).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AnchorRelative {
    /// Base position (absolute anchor of origin).
    pub position: u32,
    /// Accumulated offset (shift), may be negative.
    pub offset: i64,
}

/// Null offset: base position.
const DECALAGE_NUL: i64 = 0;

impl AnchorRelative {
    /// Relative anchor at `position`, with null offset.
    pub fn new(position: u32) -> Self {
        AnchorRelative {
            position,
            offset: DECALAGE_NUL,
        }
    }

    /// From an absolute anchor (M1): null offset.
    pub fn since_anchor(anchor: Anchor) -> Self {
        Self::new(anchor.0)
    }

    /// New relative anchor, shifted by `delta` (the sliding window advances
    /// the base by f(δ)).
    pub fn decalee(&self, delta: i64) -> Self {
        AnchorRelative {
            position: self.position,
            offset: self.offset + delta,
        }
    }

    /// Absolute position = base position + accumulated offset.
    pub fn absolute(&self) -> i64 {
        self.position as i64 + self.offset
    }

    /// Conversion to absolute anchor (saturated into `u32`).
    pub fn toward_anchor(&self) -> Anchor {
        Anchor(self.absolute().clamp(0, u32::MAX as i64) as u32)
    }
}

/// Group of key transformations designated by the controller (spec §
/// 2.4.5). Algebra contract: (i) the action is decidable and **invertible**
/// (a group: absolute read and product require t⁻¹) ; (ii) the
/// transformations are **bijections** ; (iii) the canonical section is
/// **confluent** (normal form, otherwise no hash-consing) ; (v) the family f
/// is closed under composition with a computable law:
/// f(n₁)∘f(n₂) = f(n₁ ⊕ n₂) ; (vi) the argument is stored **modulo the
/// kernel of f**. The function lives in the controller, the input edge
/// carries the argument; the pool stays blind.
pub trait GroupKeys {
    /// Stored key (domain on which G acts).
    type Key: Clone + Ord + Hash + Debug;
    /// Transformation argument (the term of the input edge).
    type Argument: Clone + Eq + Hash + Debug;

    /// Action: effective key = f(n)(stored key).
    fn apply(&self, key: &Self::Key, argument: &Self::Argument) -> Self::Key;

    /// Composition law of arguments: f(n₁)∘f(n₂) = f(n₁ ⊕ n₂).
    fn composed(&self, first: &Self::Argument, second: &Self::Argument) -> Self::Argument;

    /// Neutral argument (identity transformation).
    fn neutre(&self) -> Self::Argument;

    /// Inverse t⁻¹ of the transformation with the given argument.
    fn inverser(&self, argument: &Self::Argument) -> Self::Argument;

    /// Canonical section of orbits (confluent): normalizes the edges of a
    /// form modulo G. Returns `(argument, normalized edges)` — the argument
    /// lives on the input edge, the stored form is the section of the orbit.
    fn section_edges(
        &self,
        edges: &BTreeMap<Self::Key, NodeId>,
    ) -> (Self::Argument, BTreeMap<Self::Key, NodeId>);
}

/// Canonical instance: offset +n, G = (ℤ, +) acting on `u64` keys,
/// **"min = 0" section** — the smallest edge is labeled 0 and the argument
/// `n` is the minimum of the keys (Wireshark: relative sequence numbers,
/// industrial precedent of clause (vi)).
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupShift;

impl GroupKeys for GroupShift {
    type Key = u64;
    type Argument = i64;

    fn apply(&self, key: &u64, argument: &i64) -> u64 {
        // Saturated effective read: the absolute key stays in ℕ.
        key.saturating_add_signed(*argument)
    }

    fn composed(&self, first: &i64, second: &i64) -> i64 {
        first.saturating_add(*second)
    }

    fn neutre(&self) -> i64 {
        DECALAGE_NUL
    }

    fn inverser(&self, argument: &i64) -> i64 {
        argument.saturating_neg()
    }

    fn section_edges(&self, edges: &BTreeMap<u64, NodeId>) -> (i64, BTreeMap<u64, NodeId>) {
        match edges.keys().next() {
            // Form without edges: neutral argument, form unchanged.
            None => (self.neutre(), edges.clone()),
            // Min = 0 section: subtract the minimum from all keys.
            Some(&min) => (
                min as i64,
                edges
                    .iter()
                    .map(|(key, target)| (key - min, *target))
                    .collect(),
            ),
        }
    }
}

/// Input edge: the value of a DD is an **edge** (term f(n), handle), not a
/// bare node (spec § 2.4.5 "The root edge"). Two DDs sharing all their nodes
/// but differing in input are two distinct DDs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EdgeInput<Argument> {
    /// Transformation argument (term f(n) carried by the edge).
    pub argument: Argument,
    /// Handle of the root.
    pub handle: Handle,
}

impl<Argument: Clone> EdgeInput<Argument> {
    /// New input edge (argument, handle).
    pub fn new(argument: Argument, handle: Handle) -> Self {
        EdgeInput { argument, handle }
    }

    /// Absolute read of a stored key: effective key = f(n)(key).
    pub fn read_key<Key>(
        &self,
        group: &impl GroupKeys<Key = Key, Argument = Argument>,
        key: &Key,
    ) -> Key {
        group.apply(key, &self.argument)
    }

    /// Composition of two reads: the argument accumulates (n₁ ⊕ n₂), O(1)
    /// per step (invariant of Q6); the final handle is that of the second.
    pub fn composed(
        &self,
        group: &impl GroupKeys<Argument = Argument>,
        other: &EdgeInput<Argument>,
    ) -> EdgeInput<Argument> {
        EdgeInput {
            argument: group.composed(&self.argument, &other.argument),
            handle: other.handle,
        }
    }
}
