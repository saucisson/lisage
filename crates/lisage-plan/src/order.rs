//! First-class typed orders (M3-1, statement C3, spec § 10 Q1):
//! [`Order<T>`] is a **value** — the permutation π of the canonical keys
//! of type `T`, with the type's order constraints embedded and checked at
//! construction. **Alignment as typing**: two operands only composed under
//! a common order, imposed by the [`MarkedOrder`] type parameter (and
//! checked by value by [`Order::alignment`]).
//!
//! This is the junction point between the transduction computation
//! (T-EXPAND consults the order: the reinserted level takes its declared
//! place) and the representation plan.

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use lisage_lang::Atom;

/// Orderable type: canonical keys of the type + order constraints
/// (`before`/`after` pairs required in every valid permutation).
pub trait TypeOrder {
    /// Ordered key of a level of the type.
    type Key: Clone + Ord + Hash + Debug;

    /// Canonical sequence of keys (the type's default order).
    fn keys_canonical(&self) -> Vec<Self::Key>;

    /// Order constraints of the type: the first element must precede the
    /// second in π.
    fn constraints(&self) -> Vec<(Self::Key, Self::Key)> {
        Vec::new()
    }
}

/// Descriptor of the `Atom(D)` / `Set(D)` type: order over the atoms of
/// the bounded domain (canonical = sorted atoms, no constraint).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DomainAtoms(pub Vec<Atom>);

impl TypeOrder for DomainAtoms {
    type Key = Atom;

    fn keys_canonical(&self) -> Vec<Atom> {
        let mut keys = self.0.clone();
        keys.sort();
        keys.dedup();
        keys
    }
}

/// Phase of a `Map(K, V)` level: key presence, then value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Phase {
    /// Key presence level (δ = absent).
    Presence,
    /// Key value level.
    Value,
}

/// Key of a `Map(K, V)` level: (key, phase).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KeyMap {
    /// Key of the map.
    pub key: Atom,
    /// Level phase: presence or value.
    pub phase: Phase,
}

/// Descriptor of the `Map(K, V)` type: per-key constraint — presence
/// precedes value (universe sweep, spec § 2.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeysMap(pub Vec<Atom>);

impl KeysMap {
    /// Canonical keys, sorted (deduplicated).
    fn keys_sorted(&self) -> Vec<Atom> {
        let mut keys = self.0.clone();
        keys.sort();
        keys.dedup();
        keys
    }
}

impl TypeOrder for KeysMap {
    type Key = KeyMap;

    fn keys_canonical(&self) -> Vec<KeyMap> {
        let mut levels = Vec::new();
        for key in self.keys_sorted() {
            levels.push(KeyMap {
                key,
                phase: Phase::Presence,
            });
            levels.push(KeyMap {
                key,
                phase: Phase::Value,
            });
        }
        levels
    }

    fn constraints(&self) -> Vec<(KeyMap, KeyMap)> {
        self.keys_sorted()
            .into_iter()
            .map(|key| {
                (
                    KeyMap {
                        key,
                        phase: Phase::Presence,
                    },
                    KeyMap {
                        key,
                        phase: Phase::Value,
                    },
                )
            })
            .collect()
    }
}

/// Typed error for order construction or alignment (cause preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorOrder<Key> {
    /// π is not a permutation of the canonical keys (missing key, duplicate,
    /// or foreign to the type).
    NotOnePermutation,
    /// Type constraint violated: `before` must precede `after` in π.
    ConstraintViolated {
        /// Key that had to precede.
        before: Key,
        /// Key that had to follow.
        after: Key,
    },
    /// Two orders of the same type are not aligned (different π).
    OrdersNotAlignes,
}

impl<Key: Debug> std::fmt::Display for ErrorOrder<Key> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorOrder::NotOnePermutation => {
                write!(formatter, "π is not a permutation of the canonical keys")
            }
            ErrorOrder::ConstraintViolated { before, after } => {
                write!(
                    formatter,
                    "constraint violated: {before:?} must precede {after:?}"
                )
            }
            ErrorOrder::OrdersNotAlignes => {
                write!(formatter, "orders not aligned (different π)")
            }
        }
    }
}

impl<Key: Debug> std::error::Error for ErrorOrder<Key> {}

/// Strategy adopted by [`Order::with_downstream_runtime`] (M5-1, C3): the
/// runtime order was accepted, or we fell back to the canonical order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrategieDownstream {
    /// Valid runtime π: order adopted as-is.
    OrderRuntime,
    /// Invalid runtime π: fallback to the canonical order (degraded).
    FoldCanonical,
}

/// Total order over the keys of a type `T` — **first-class value**
/// (M3-1, C3): the permutation π of the canonical keys, the type's
/// constraints being embedded and checked at construction.
#[derive(Debug)]
pub struct Order<T: TypeOrder> {
    /// Canonical keys of the type (π = identity on this sequence).
    canonical: Vec<T::Key>,
    /// π: ordered sequence of keys (permutation of the canonical keys).
    sequence: Vec<T::Key>,
    /// Order constraints of the type, embedded.
    constraints: Vec<(T::Key, T::Key)>,
    marker: PhantomData<fn() -> T>,
}

// Manual implementations: derives would add bounds on `T` itself, whereas
// only the keys `T::Key` participate in comparisons.
impl<T: TypeOrder> Clone for Order<T> {
    fn clone(&self) -> Self {
        Order {
            canonical: self.canonical.clone(),
            sequence: self.sequence.clone(),
            constraints: self.constraints.clone(),
            marker: PhantomData,
        }
    }
}

impl<T: TypeOrder> PartialEq for Order<T> {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
            && self.sequence == other.sequence
            && self.constraints == other.constraints
    }
}

impl<T: TypeOrder> Eq for Order<T> {}

impl<T: TypeOrder> Order<T> {
    /// Canonical order of the type (π = identity).
    pub fn canonical(descripteur: &T) -> Self {
        let canonical = descripteur.keys_canonical();
        Order {
            sequence: canonical.clone(),
            canonical,
            constraints: descripteur.constraints(),
            marker: PhantomData,
        }
    }

    /// Order given by the permutation π, checked: π must be a permutation
    /// of the canonical keys and satisfy the type's constraints.
    pub fn new(descripteur: &T, pi: Vec<T::Key>) -> Result<Self, ErrorOrder<T::Key>> {
        let canonical = descripteur.keys_canonical();
        let mut pi_sorted = pi.clone();
        pi_sorted.sort();
        let mut canonical_sorted = canonical.clone();
        canonical_sorted.sort();
        if pi_sorted != canonical_sorted {
            return Err(ErrorOrder::NotOnePermutation);
        }
        let order = Order {
            canonical,
            sequence: pi,
            constraints: descripteur.constraints(),
            marker: PhantomData,
        };
        for (before, after) in &order.constraints {
            let satisfies = match (order.position(before), order.position(after)) {
                (Some(position_before), Some(position_after)) => position_before < position_after,
                _ => false,
            };
            if !satisfies {
                return Err(ErrorOrder::ConstraintViolated {
                    before: <T::Key as Clone>::clone(before),
                    after: <T::Key as Clone>::clone(after),
                });
            }
        }
        Ok(order)
    }

    /// Order with **runtime downstream** (M5-1, C3): try to adopt the
    /// permutation π supplied at runtime; if it is invalid (not a
    /// permutation or constraint violated), **fall back to the canonical
    /// order** (degraded reordering as fallback) and report it via the
    /// returned strategy.
    pub fn with_downstream_runtime(
        descripteur: &T,
        pi_runtime: Vec<T::Key>,
    ) -> (Self, StrategieDownstream) {
        match Order::new(descripteur, pi_runtime) {
            Ok(order) => (order, StrategieDownstream::OrderRuntime),
            Err(_cause) => (
                Order::canonical(descripteur),
                StrategieDownstream::FoldCanonical,
            ),
        }
    }

    /// Position (0-based) of a key in π.
    pub fn position(&self, key: &T::Key) -> Option<usize> {
        self.sequence.iter().position(|c| c == key)
    }

    /// Key at a position of π.
    pub fn key(&self, position: usize) -> Option<&T::Key> {
        self.sequence.get(position)
    }

    /// Number of keys in the order.
    pub fn size(&self) -> usize {
        self.sequence.len()
    }

    /// Ordered sequence of keys (π itself).
    pub fn sequence(&self) -> &[T::Key] {
        &self.sequence
    }

    /// Canonical keys of the type (π = identity on this sequence).
    pub fn canonical_keys(&self) -> &[T::Key] {
        &self.canonical
    }

    /// π as an array: index in the canonical sequence → position in π.
    pub fn permutation(&self) -> Vec<usize> {
        self.canonical
            .iter()
            .map(|key| self.position(key).expect("canonical key present in π"))
            .collect()
    }

    /// Order constraints embedded in `Order<T>`.
    pub fn constraints(&self) -> &[(T::Key, T::Key)] {
        &self.constraints
    }

    /// Two orders of the same type are aligned iff same π.
    pub fn alignes(&self, other: &Self) -> bool {
        self.sequence == other.sequence
    }

    /// Checked alignment between two orders of the same type, typed error
    /// otherwise (C3: two operands only composed under a common order).
    pub fn alignment(&self, other: &Self) -> Result<(), ErrorOrder<T::Key>> {
        if self.alignes(other) {
            Ok(())
        } else {
            Err(ErrorOrder::OrdersNotAlignes)
        }
    }
}

/// Order mark: zero-sized type carrying the [`Order<T>`] of a type `T`.
/// Alignment becomes a typing fact: two operands only composed under the
/// same mark `O` (C3).
pub trait MarkedOrder<T: TypeOrder>: Sized + Send + Sync {
    /// The order (first-class value) associated with the mark.
    fn order(descripteur: &T) -> Order<T>;
}

/// Operand aligned on the order `O` of type `T`. Composition
/// ([`Aligne::composed`]) requires the same `O` parameter on both sides:
/// operands under different orders do not compiled.
pub struct Aligne<T: TypeOrder, O: MarkedOrder<T>, V> {
    value: V,
    marker: PhantomData<fn() -> (T, O)>,
}

impl<T: TypeOrder, O: MarkedOrder<T>, V> Aligne<T, O, V> {
    /// New operand, aligned on the mark `O`.
    pub fn new(value: V) -> Self {
        Aligne {
            value,
            marker: PhantomData,
        }
    }

    /// Value carried by the operand.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Composition under a common order: `other` shares the same `(T, O)` —
    /// alignment is imposed by typing, not checked at runtime.
    pub fn composed<U, W>(
        &self,
        other: &Aligne<T, O, U>,
        combinaison: impl FnOnce(&V, &U) -> W,
    ) -> Aligne<T, O, W> {
        Aligne::new(combinaison(&self.value, other.value()))
    }
}
