//! Lisage core — pool, handles, GC, bi-plan, placement.
//!
//! Niveau 3 : stockage canonique aveugle. La crate implémente le port
//! [`lisage_algebra::interface::Pool`] ([`arena::Arena`]) et expose le port
//! d'instrumentation [`observer::Observer`]. Elle ré-exporte les types
//! d'interface du contrat pour que les crates qui ne dépendent pas
//! d'`algebra` (Liseuse) puissent lire les diagrammes.

pub mod arena;
pub mod error;
pub mod lookup;
pub mod observer;
pub mod pli;
pub mod schema;
pub mod transduction;

pub use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, Pool, PoolError,
};

/// Placeholder factice pour valider la CI (M0-1).
pub fn socle() -> u32 {
    42
}
