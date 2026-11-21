//! Level 1 values: bounded atoms, sets, maps (bounded instance of
//! reference, spec § 1).

use std::collections::{BTreeMap, BTreeSet};

/// Bounded atom: distinct type (ADR-0006), finite enumerable domain declared
/// by the schema (M1-3).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Atom(pub u64);

/// Value of the reference language (type algebra Σ, bounded instance).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Value {
    Atom(Atom),
    Set(BTreeSet<Value>),
    Map(BTreeMap<Value, Value>),
    /// Ordered sequence of elements (M2-1, `list` constructor).
    Seq(Vec<Value>),
    /// Typed union: discriminant + payload (M2-2, `Tag` constructor).
    Tag(Atom, Box<Value>),
}
