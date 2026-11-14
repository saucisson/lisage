//! First-class typed orders (M3-1, claim C3, spec § 10 Q1):
//! [`Order<T>`] is a **value** — the permutation π of the canonical keys
//! of type `T`, with the type's order constraints embedded and
//! checked at construction. **Alignment as typing**: two operands
//! compose only under a common order, imposed by the type parameter
//! [`MarqueOrdre`] (and checked by value via [`Order::alignement`]).
//!
//! This is the junction point between transduction computation (T-EXPAND
//! consults the order: the reinserted level takes its declared place) and
//! the representation plan.

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use lisage_lang::Atom;

/// Orderable type: canonical keys of the type + order constraints
/// (`before`/`after` pairs required in any valid permutation).
pub trait TypeOrdre {
    /// Ordered key of a level of the type.
    type Key: Clone + Ord + Hash + Debug;

    /// Canonical sequence of keys (the type's default order).
    fn keys_canoniques(&self) -> Vec<Self::Key>;

    /// Order constraints of the type: `before` must precede `after` in π.
    fn contraintes(&self) -> Vec<(Self::Key, Self::Key)> {
        Vec::new()
    }
}

/// Descriptor of type `Atom(D)` / `Set(D)`: order over the atoms of the
/// bounded domain (canonical = sorted atoms, no constraint).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DomainAtomes(pub Vec<Atom>);

impl TypeOrdre for DomainAtomes {
    type Key = Atom;

    fn keys_canoniques(&self) -> Vec<Atom> {
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

/// Descriptor of type `Map(K, V)`: per-key constraint — presence
/// precedes value (universe sweep, spec § 2.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeysMap(pub Vec<Atom>);

impl KeysMap {
    /// Sorted canonical keys (no duplicates).
    fn keys_triees(&self) -> Vec<Atom> {
        let mut keys = self.0.clone();
        keys.sort();
        keys.dedup();
        keys
    }
}

impl TypeOrdre for KeysMap {
    type Key = KeyMap;

    fn keys_canoniques(&self) -> Vec<KeyMap> {
        let mut levels = Vec::new();
        for key in self.keys_triees() {
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

    fn contraintes(&self) -> Vec<(KeyMap, KeyMap)> {
        self.keys_triees()
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

/// Typed error of order construction or alignment (cause preserved).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErreurOrdre<Key> {
    /// π is not a permutation of the canonical keys (missing key, duplicate,
    /// or foreign to the type).
    PasUnePermutation,
    /// Type constraint violated: `before` must precede `after` in π.
    ContrainteViolee {
        /// Key that should precede.
        avant: Key,
        /// Key that should follow.
        apres: Key,
    },
    /// Two orders of the same type are not aligned (different π).
    OrdresNonAlignes,
}

impl<Key: Debug> std::fmt::Display for ErreurOrdre<Key> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurOrdre::PasUnePermutation => {
                write!(formatter, "π is not a permutation of the canonical keys")
            }
            ErreurOrdre::ContrainteViolee { avant, apres } => {
                write!(
                    formatter,
                    "constraint violated: {avant:?} must precede {apres:?}"
                )
            }
            ErreurOrdre::OrdresNonAlignes => {
                write!(formatter, "orders not aligned (different π)")
            }
        }
    }
}

impl<Key: Debug> std::error::Error for ErreurOrdre<Key> {}

/// Strategy adopted by [`Order::avec_aval_runtime`] (M5-1, C3): the runtime
/// order was accepted, or we fell back to the canonical order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrategieAval {
    /// Valid runtime π: order adopted as-is.
    OrdreRuntime,
    /// Invalid runtime π: fallback to the canonical order (degraded).
    RepliCanonique,
}

/// Total order over the keys of a type `T` — **first-class value**
/// (M3-1, C3): the permutation π of the canonical keys, the type's
/// constraints being embedded and checked at construction.
#[derive(Debug)]
pub struct Order<T: TypeOrdre> {
    /// Canonical keys of the type (π = identity on this sequence).
    canoniques: Vec<T::Key>,
    /// π: ordered sequence of keys (permutation of the canonicals).
    sequence: Vec<T::Key>,
    /// Order constraints of the type, embedded.
    contraintes: Vec<(T::Key, T::Key)>,
    marqueur: PhantomData<fn() -> T>,
}

// Manual implementations: derives would add bounds on `T` itself,
// whereas only the keys `T::Key` participate in comparisons.
impl<T: TypeOrdre> Clone for Order<T> {
    fn clone(&self) -> Self {
        Order {
            canoniques: self.canoniques.clone(),
            sequence: self.sequence.clone(),
            contraintes: self.contraintes.clone(),
            marqueur: PhantomData,
        }
    }
}

impl<T: TypeOrdre> PartialEq for Order<T> {
    fn eq(&self, other: &Self) -> bool {
        self.canoniques == other.canoniques
            && self.sequence == other.sequence
            && self.contraintes == other.contraintes
    }
}

impl<T: TypeOrdre> Eq for Order<T> {}

impl<T: TypeOrdre> Order<T> {
    /// Canonical order of the type (π = identity).
    pub fn canonique(descripteur: &T) -> Self {
        let canoniques = descripteur.keys_canoniques();
        Order {
            sequence: canoniques.clone(),
            canoniques,
            contraintes: descripteur.contraintes(),
            marqueur: PhantomData,
        }
    }

    /// Order given by permutation π, checked: π must be a
    /// permutation of the canonical keys and satisfy the type's constraints.
    /// type.
    pub fn new(descripteur: &T, pi: Vec<T::Key>) -> Result<Self, ErreurOrdre<T::Key>> {
        let canoniques = descripteur.keys_canoniques();
        let mut pi_triee = pi.clone();
        pi_triee.sort();
        let mut canoniques_triees = canoniques.clone();
        canoniques_triees.sort();
        if pi_triee != canoniques_triees {
            return Err(ErreurOrdre::PasUnePermutation);
        }
        let ordre = Order {
            canoniques,
            sequence: pi,
            contraintes: descripteur.contraintes(),
            marqueur: PhantomData,
        };
        for (avant, apres) in &ordre.contraintes {
            let satisfait = match (ordre.position(avant), ordre.position(apres)) {
                (Some(position_avant), Some(position_apres)) => position_avant < position_apres,
                _ => false,
            };
            if !satisfait {
                return Err(ErreurOrdre::ContrainteViolee {
                    avant: <T::Key as Clone>::clone(avant),
                    apres: <T::Key as Clone>::clone(apres),
                });
            }
        }
        Ok(ordre)
    }

    /// Order with **runtime downstream** (M5-1, C3): try to adopt the
    /// permutation π provided at runtime; if invalid
    /// (not a permutation or constraint violated), we **fall back to
    /// the canonical order** (degraded reordering as fallback) and report it
    /// via the returned strategy.
    pub fn avec_aval_runtime(descripteur: &T, pi_runtime: Vec<T::Key>) -> (Self, StrategieAval) {
        match Order::new(descripteur, pi_runtime) {
            Ok(ordre) => (ordre, StrategieAval::OrdreRuntime),
            Err(_cause) => (Order::canonique(descripteur), StrategieAval::RepliCanonique),
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
    pub fn canoniques(&self) -> &[T::Key] {
        &self.canoniques
    }

    /// π as an array: index in the canonical sequence → position in π.
    pub fn permutation(&self) -> Vec<usize> {
        self.canoniques
            .iter()
            .map(|key| self.position(key).expect("canonical key present in π"))
            .collect()
    }

    /// Order constraints embedded in `Order<T>`.
    pub fn contraintes(&self) -> &[(T::Key, T::Key)] {
        &self.contraintes
    }

    /// Two orders of the same type are aligned iff same π.
    pub fn alignes(&self, other: &Self) -> bool {
        self.sequence == other.sequence
    }

    /// Alignment checked between two orders of the same type, typed error
    /// otherwise (C3: two operands compose only under a common order).
    pub fn alignement(&self, other: &Self) -> Result<(), ErreurOrdre<T::Key>> {
        if self.alignes(other) {
            Ok(())
        } else {
            Err(ErreurOrdre::OrdresNonAlignes)
        }
    }
}

/// Order mark: zero-sized type carrying the [`Order<T>`] of a type `T`.
/// Alignment becomes a typing fact: two operands compose only under the
/// same mark `O` (C3).
pub trait MarqueOrdre<T: TypeOrdre>: Sized + Send + Sync {
    /// The order (first-class value) associated with the mark.
    fn ordre(descripteur: &T) -> Order<T>;
}

/// Operand aligned on the order `O` of type `T`. Composition
/// ([`Aligne::compose`]) requires the same `O` parameter on both sides:
/// operands under different orders do not compile.
pub struct Aligne<T: TypeOrdre, O: MarqueOrdre<T>, V> {
    value: V,
    marqueur: PhantomData<fn() -> (T, O)>,
}

impl<T: TypeOrdre, O: MarqueOrdre<T>, V> Aligne<T, O, V> {
    /// New operand, aligned on mark `O`.
    pub fn new(value: V) -> Self {
        Aligne {
            value,
            marqueur: PhantomData,
        }
    }

    /// Value carried by the operand.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Composition under common order: `other` shares the same `(T, O)` —
    /// alignment is imposed by typing, not checked at runtime.
    pub fn compose<U, W>(
        &self,
        other: &Aligne<T, O, U>,
        combinaison: impl FnOnce(&V, &U) -> W,
    ) -> Aligne<T, O, W> {
        Aligne::new(combinaison(&self.value, other.value()))
    }
}
