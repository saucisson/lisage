//! Erreurs du cœur (roadmap § 10) : invariants internes = `assert!` ;
//! frontières du port pool = `PoolError` typé, cause préservée.

use lisage_algebra::interface::{Level, PoolError};

/// Erreur typée de frontière du cœur.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CoreError {
    /// Erreur remontée du port pool.
    Pool(PoolError),
    /// Opération demandée sur un pool vide.
    EmptyArena,
    /// Schéma inattendu à la frontière d'une opération.
    SchemaInattendu {
        /// Forme de schéma attendue.
        attendu: &'static str,
        /// Forme de schéma reçue.
        recu: &'static str,
    },
    /// Niveau d'un nœud hors du schéma (diagramme incompatible).
    NiveauHorsSchema(Level),
    /// Effet mémoire produit alors que la politique du contrôleur est
    /// `Aucune` (M2-3, T3).
    MemoireNonAutorisee,
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::Pool(error) => write!(formatter, "erreur du pool: {error}"),
            CoreError::EmptyArena => write!(formatter, "le pool est vide"),
            CoreError::SchemaInattendu { attendu, recu } => {
                write!(
                    formatter,
                    "schéma inattendu: attendu {attendu}, reçu {recu}"
                )
            }
            CoreError::NiveauHorsSchema(niveau) => {
                write!(formatter, "niveau hors schéma: {niveau:?}")
            }
            CoreError::MemoireNonAutorisee => {
                write!(formatter, "effet mémoire produit avec la politique Aucune")
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
