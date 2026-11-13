//! Core errors (roadmap § 10): internal invariants = `assert!`;
//! pool port boundaries = typed `PoolError`, cause preserved.

use lisage_algebra::interface::{Level, PoolError};

/// Typed error at the core boundary.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CoreError {
    /// Error raised from the pool port.
    Pool(PoolError),
    /// Operation requested on an empty pool.
    EmptyArena,
    /// Unexpected schema at the boundary of an operation.
    SchemaUnexpected {
        /// Expected schema form.
        expected: &'static str,
        /// Received schema form.
        recu: &'static str,
    },
    /// Level of a node outside the schema (incompatible diagram).
    LevelOutSchema(Level),
    /// Memory effect produced while the controller's policy is
    /// `No` (M2-3, T3).
    MemoryNotAutorisee,
    /// Fixed-point loop (`saturate`, M2-4) exceeding the safety
    /// bound (T2 guarantees termination on a bounded schema; exceeding the
    /// bound signals a regression).
    NotTerminaison {
        /// Iteration bound exceeded.
        bound: u32,
    },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::Pool(error) => write!(formatter, "pool error: {error}"),
            CoreError::EmptyArena => write!(formatter, "the pool is empty"),
            CoreError::SchemaUnexpected { expected, recu } => {
                write!(
                    formatter,
                    "unexpected schema: expected {expected}, got {recu}"
                )
            }
            CoreError::LevelOutSchema(level) => {
                write!(formatter, "level outside schema: {level:?}")
            }
            CoreError::MemoryNotAutorisee => {
                write!(formatter, "memory effect produced with policy No")
            }
            CoreError::NotTerminaison { bound } => {
                write!(
                    formatter,
                    "saturation did not terminate after {bound} iterations"
                )
            }
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Pool(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PoolError> for CoreError {
    fn from(error: PoolError) -> Self {
        CoreError::Pool(error)
    }
}
