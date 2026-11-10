//! Politique d'erreurs du cœur (feuille de route § 10) : les invariants
//! internes de `core/` et `algebra/` sont des `assert!` (panic), les erreurs
//! de frontière du port pool sont typées (`PoolError`, cause préservée).

use lisage_algebra::interface::PoolError;

/// Erreur typée de frontière du cœur.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CoreError {
    /// Erreur remontée du port pool.
    Pool(PoolError),
    /// Opération demandée sur un pool vide.
    EmptyArena,
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::Pool(error) => write!(formatter, "erreur du pool: {error}"),
            CoreError::EmptyArena => write!(formatter, "le pool est vide"),
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Pool(error) => Some(error),
            CoreError::EmptyArena => None,
        }
    }
}

impl From<PoolError> for CoreError {
    fn from(error: PoolError) -> Self {
        CoreError::Pool(error)
    }
}
