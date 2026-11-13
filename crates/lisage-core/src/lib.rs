//! Lisage core — pool, handles, GC, two-plane, placement.
//!
//! Level 3: blind canonical storage. The crate implements the port
//! [`lisage_algebra::interface::Pool`] ([`arena::Arena`]) and exposes the
//! instrumentation port [`observer::Observer`]. It re-exports the contract's
//! interface types so that crates that do not depend on `algebra` (Reader)
//! can read the diagrams.

pub mod add;
pub mod arena;
pub mod bdd;
pub mod biplan;
pub mod cdd;
pub mod dbm;
pub mod ddd;
pub mod error;
pub mod fold;
pub mod integral;
pub mod lookup;
pub mod memory;
pub mod observer;
pub mod operations;
pub mod region;
pub mod saturate;
pub mod schema;
pub mod transduction;
pub mod transfer;
pub mod workshop;

pub use lisage_algebra::interface::{
    Anchor, Controller, ControllerId, Form, Handle, KeyAlgebra, Level, NodeId, PolicyMemory, Pool,
    PoolError,
};

/// Dummy placeholder to validate CI (M0-1).
pub fn base() -> u32 {
    42
}
