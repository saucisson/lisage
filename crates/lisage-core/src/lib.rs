//! Lisage core — pool, handles, GC, two-plane, placement.
//!
//! Layer 3: blind canonical storage. The crate implements the
//! [`lisage_algebra::interface::Pool`] port ([`arena::Arena`]) and exposes
//! the instrumentation port [`observer::Observer`]. It re-exports the
//! contract's interface types so that crates not depending on `algebra`
//! (Liseuse) can read the diagrams.

pub mod add;
pub mod arena;
pub mod bdd;
pub mod biplan;
pub mod cdd;
pub mod classes;
pub mod dbm;
pub mod ddd;
pub mod error;
pub mod fold;
pub mod integral;
pub mod lookup;
pub mod memory;
pub mod observer;
pub mod operations;
pub mod parallel_ops;
pub mod region;
pub mod saturate;
pub mod schema;
pub mod surplace;
pub mod transduction;
pub mod transfer;
mod unique;
pub mod workshop;

pub use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, MemoryPolicy, NodeId,
    PathPredicate, Pool, PoolError,
};

/// Dummy placeholder to validate CI (M0-1).
pub fn base() -> u32 {
    42
}
