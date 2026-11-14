//! Level 1 errors (roadmap § 10): typed `Result`s, causes preserved
//! (the internal invariants of `core/`/`algebra/` are asserts).

/// Key name of the language, distinct type at the boundary (ADR-0006).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KeyName(pub String);

/// Typed error of the reference language.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LangError {
    /// `lookup` on a key outside the declared universe.
    KeyOutsideUniverse(KeyName),
    /// Value of an unexpected type at a point in the computation.
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// Empty universe: no presence level to sweep.
    EmptyUniverse,
}

impl std::fmt::Display for LangError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LangError::KeyOutsideUniverse(key) => {
                write!(formatter, "key outside the declared universe: {}", key.0)
            }
            LangError::TypeMismatch { expected, found } => {
                write!(formatter, "expected type {expected}, found {found}")
            }
            LangError::EmptyUniverse => write!(formatter, "empty universe"),
        }
    }
}

impl std::error::Error for LangError {}
