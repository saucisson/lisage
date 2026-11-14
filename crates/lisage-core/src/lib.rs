//! Lisage core — pool, handles, GC, two-plane, placement.
//!
//! Layer 3: blind canonical storage. The crate implements the
//! [`lisage_algebra::interface::Pool`] port ([`arena::Arena`]) and exposes
//! the instrumentation port [`observer::Observer`]. It re-exports the
//! contract's interface types so that crates not depending on `algebra`
//! (Liseuse) can read the diagrams.

pub mod add;
pub mod arena;
pub mod atelier;
pub mod bdd;
pub mod biplan;
pub mod cdd;
pub mod dbm;
pub mod ddd;
pub mod error;
pub mod integrale;
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

/// Dummy placeholder to validate CI (M0-1).
pub fn socle() -> u32 {
    42
}
