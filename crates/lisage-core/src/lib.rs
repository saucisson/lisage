//! Lisage core — pool, handles, GC, bi-plan, placement.
//!
//! Niveau 3 : stockage canonique aveugle. La crate implémente le port
//! [`lisage_algebra::interface::Pool`] ([`arena::Arena`]) et expose le port
//! d'instrumentation [`observer::Observer`]. Elle ré-exporte les types
//! d'interface du contrat pour que les crates qui ne dépendent pas
//! d'`algebra` (Liseuse) puissent lire les diagrammes.

pub mod add;
pub mod arena;
pub mod bdd;
pub mod biplan;
pub mod dbm;
pub mod error;
pub mod lookup;
pub mod memoire;
pub mod observer;
pub mod operations;
pub mod pli;
pub mod region;
pub mod saturate;
pub mod schema;
pub mod transduction;
pub mod transfert;

pub use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, PolitiqueMemoire,
    Pool, PoolError,
};

/// Placeholder factice pour valider la CI (M0-1).
pub fn socle() -> u32 {
    42
}
