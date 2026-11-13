//! Lisage algebra — level 2: extended controller, transductions, reductions.
//!
//! This crate carries the contract of the core ([`interface`]): the low-level
//! pool (`lisage-core`) is only a canonical storage implementing this
//! contract.

pub mod interface;
pub mod reduction;
pub mod relative;
pub mod vtree;
