//! Core errors (roadmap § 10): internal invariants = `assert!`; pool port
//! boundaries = typed `PoolError`, cause preserved.

use lisage_algebra::interface::{Level, PoolError};

/// Typed core boundary error.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CoreError {
    /// Error raised from the pool port.
    Pool(PoolError),
    /// Operation requested on an empty pool.
    EmptyArena,
    /// Unexpected schema at an operation boundary.
    SchemaInexpected {
        /// Expected schema shape.
        expected: &'static str,
        /// Received schema shape.
        recu: &'static str,
    },
    /// Node level outside the schema (incompatible diagram).
    LevelHorsSchema(Level),
    /// Memory effect produced while the controller's policy is `NoMemory`
    /// (M2-3, T3).
    MemoryNonAutorisee,
    /// Fixpoint loop (`saturate`, M2-4) exceeding the safety bound (T2
    /// guarantees termination on a bounded schema; exceeding the bound
    /// signals a regression).
    NonTerminaison {
        /// Iteration bound exceeded.
        borne: u32,
    },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::Pool(error) => write!(formatter, "pool error: {error}"),
            CoreError::EmptyArena => write!(formatter, "the pool is empty"),
            CoreError::SchemaInexpected { expected, recu } => {
                write!(
                    formatter,
                    "unexpected schema: expected {expected}, got {recu}"
                )
            }
            CoreError::LevelHorsSchema(level) => {
                write!(formatter, "level outside schema: {level:?}")
            }
            CoreError::MemoryNonAutorisee => {
                write!(formatter, "memory effect produced with the NoMemory policy")
            }
            CoreError::NonTerminaison { borne } => {
                write!(
                    formatter,
                    "saturation did not terminate after {borne} iterations"
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
