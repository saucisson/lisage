//! Relative forms (M4-2, claim C8, spec § 2.4.5): activation of the **shift**
//! (relative encoding carried by the anchor), **composite anchors** (offsets
//! named per fragment) and **key transformations** designated by the
//! controller (action group + canonical section of orbits).
//!
//! One idea: the form is relative, the context carries the absolute.
//! `algebra/` side only — the pool stays blind and unchanged (C1):
//! the anchor (position) lives in the handle, the transformation
//! term (value) is an argument on the incoming arc, carried by the caller.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

use crate::interface::{Anchor, Handle, NodeId};

/// Named fragment of a factorized network: the composite anchor carries one
/// offset per fragment. Canonical order: lexicographic of names.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Fragment(pub String);

/// Composite anchor (spec § 2.4.5): named record of offsets, one per
/// fragment (`{machine: +i, conveyor: +j}`). Zero = start of fragment.
/// Stays in the handle (C1 unchanged), the pool stays blind.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct AncreComposee {
    decalages: BTreeMap<Fragment, i64>,
}

impl AncreComposee {
    /// Null anchor: all fragments at start (zero).
    pub fn nouveau() -> Self {
        Self::default()
    }

    /// Builds an anchor with the given offset for a fragment.
    pub fn avec(mut self, fragment: Fragment, decalage: i64) -> Self {
        self.decalages.insert(fragment, decalage);
        self
    }

    /// Offset of a fragment; 0 if absent (zero = start of fragment).
    pub fn decalage(&self, fragment: &Fragment) -> i64 {
        self.decalages.get(fragment).copied().unwrap_or(0)
    }

    /// New anchor, one fragment shifted by `delta` (the slide).
    pub fn decalee(&self, fragment: &Fragment, delta: i64) -> Self {
        let mut resultat = self.clone();
        let courant = resultat.decalage(fragment);
        resultat.decalages.insert(fragment.clone(), courant + delta);
        resultat
    }

    /// Pointwise composition of offsets (absolute reading = composition of
    /// arguments, O(1) per step).
    pub fn composer(&self, autre: &AncreComposee) -> Self {
        let mut resultat = self.clone();
        for (fragment, decalage) in &autre.decalages {
            let courant = resultat.decalage(fragment);
            resultat
                .decalages
                .insert(fragment.clone(), courant + *decalage);
        }
        resultat
    }

    /// True if all offsets are zero (start of each fragment).
    pub fn est_nulle(&self) -> bool {
        self.decalages.values().all(|&d| d == 0)
    }

    /// Iterator over (fragment, offset), lexicographic order of names.
    pub fn fragments(&self) -> impl Iterator<Item = (&Fragment, i64)> {
        self.decalages.iter().map(|(f, &d)| (f, d))
    }
}

/// Relative anchor: activation of the shift (relative encoding). The
/// absolute position of the root reads `position + offset`; the offset is
/// the accumulated argument (shift +1 per cell, the anchor bounds the unfolding).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AncreRelative {
    /// Base position (original absolute anchor).
    pub position: u32,
    /// Accumulated offset (shift), may be negative.
    pub decalage: i64,
}

/// Null offset: base position.
const DECALAGE_NUL: i64 = 0;

impl AncreRelative {
    /// Relative anchor at `position`, with null offset.
    pub fn nouvelle(position: u32) -> Self {
        AncreRelative {
            position,
            decalage: DECALAGE_NUL,
        }
    }

    /// From an absolute anchor (M1): null offset.
    pub fn depuis_ancre(ancre: Anchor) -> Self {
        Self::nouvelle(ancre.0)
    }

    /// New relative anchor, shifted by `delta` (the sliding window's slide
    /// advances the base by f(δ)).
    pub fn decalee(&self, delta: i64) -> Self {
        AncreRelative {
            position: self.position,
            decalage: self.decalage + delta,
        }
    }

    /// Absolute position = base position + accumulated offset.
    pub fn absolue(&self) -> i64 {
        self.position as i64 + self.decalage
    }

    /// Conversion to absolute anchor (saturated into `u32`).
    pub fn vers_ancre(&self) -> Anchor {
        Anchor(self.absolue().clamp(0, u32::MAX as i64) as u32)
    }
}

/// Key transformation group designated by the controller (spec §
/// 2.4.5). Algebra contract: (i) the action is decidable and **invertible**
/// (a group: absolute reading and product require t⁻¹); (ii) the
/// transformations are **bijections**; (iii) the canonical section is
/// **confluent** (normal form, otherwise no hash-consing); (v) the
/// family f is closed under composition with a computable law:
/// f(n₁)∘f(n₂) = f(n₁ ⊕ n₂); (vi) the argument is stored **modulo the kernel
/// of f**. The function lives in the controller, the incoming arc carries
/// the argument; the pool stays blind.
pub trait GroupeCles {
    /// Stored key (domain on which G acts).
    type Cle: Clone + Ord + Hash + Debug;
    /// Transformation argument (the term of the incoming arc).
    type Argument: Clone + Eq + Hash + Debug;

    /// Action: effective key = f(n)(stored key).
    fn appliquer(&self, cle: &Self::Cle, argument: &Self::Argument) -> Self::Cle;

    /// Composition law of arguments: f(n₁)∘f(n₂) = f(n₁ ⊕ n₂).
    fn composer(&self, premier: &Self::Argument, second: &Self::Argument) -> Self::Argument;

    /// Neutral argument (identity transformation).
    fn neutre(&self) -> Self::Argument;

    /// Inverse t⁻¹ of the transformation with the given argument.
    fn inverser(&self, argument: &Self::Argument) -> Self::Argument;

    /// Canonical section of orbits (confluent): normalizes the arcs
    /// of a form modulo G. Returns `(argument, normalized arcs)` —
    /// the argument lives on the incoming arc, the stored form is the
    /// section of the orbit.
    fn section_arcs(
        &self,
        arcs: &BTreeMap<Self::Cle, NodeId>,
    ) -> (Self::Argument, BTreeMap<Self::Cle, NodeId>);
}

/// Canonical instance: the +n shift, G = (ℤ, +) acting on `u64` keys,
/// **section "min = 0"** — the smallest arc is labeled 0 and the
/// argument `n` is the minimum of the keys (Wireshark: relative sequence
/// numbers, industrial precedent for clause (vi)).
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupeShift;

impl GroupeCles for GroupeShift {
    type Cle = u64;
    type Argument = i64;

    fn appliquer(&self, cle: &u64, argument: &i64) -> u64 {
        // Saturated effective reading: the absolute key stays in ℕ.
        cle.saturating_add_signed(*argument)
    }

    fn composer(&self, premier: &i64, second: &i64) -> i64 {
        premier.saturating_add(*second)
    }

    fn neutre(&self) -> i64 {
        DECALAGE_NUL
    }

    fn inverser(&self, argument: &i64) -> i64 {
        argument.saturating_neg()
    }

    fn section_arcs(&self, arcs: &BTreeMap<u64, NodeId>) -> (i64, BTreeMap<u64, NodeId>) {
        match arcs.keys().next() {
            // Form without arc: neutral argument, form unchanged.
            None => (self.neutre(), arcs.clone()),
            // Section min = 0: subtract the minimum from all keys.
            Some(&min) => (
                min as i64,
                arcs.iter()
                    .map(|(cle, cible)| (cle - min, *cible))
                    .collect(),
            ),
        }
    }
}

/// Entry arc: the value of a DD is an **arc** (term f(n), handle),
/// not a bare node (spec § 2.4.5 "The root arc"). Two DDs sharing
/// all their nodes but differing in entry are two distinct DDs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ArcEntree<Argument> {
    /// Transformation argument (term f(n) carried by the arc).
    pub argument: Argument,
    /// Handle of the root.
    pub handle: Handle,
}

impl<Argument: Clone> ArcEntree<Argument> {
    /// New entry arc (argument, handle).
    pub fn nouvelle(argument: Argument, handle: Handle) -> Self {
        ArcEntree { argument, handle }
    }

    /// Absolute reading of a stored key: effective key = f(n)(key).
    pub fn lire_cle<Cle>(
        &self,
        groupe: &impl GroupeCles<Cle = Cle, Argument = Argument>,
        cle: &Cle,
    ) -> Cle {
        groupe.appliquer(cle, &self.argument)
    }

    /// Composition of two readings: the argument accumulates (n₁ ⊕ n₂),
    /// O(1) per step (invariant of Q6); the final handle is that of the second.
    pub fn composer(
        &self,
        groupe: &impl GroupeCles<Argument = Argument>,
        autre: &ArcEntree<Argument>,
    ) -> ArcEntree<Argument> {
        ArcEntree {
            argument: groupe.composer(&self.argument, &autre.argument),
            handle: autre.handle,
        }
    }
}
