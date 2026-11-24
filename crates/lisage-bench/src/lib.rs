//! Lisage bench — differential, property-based, MCC, not-local events.

pub mod confrontation;
pub mod differential;
pub mod hsdd;
pub mod pnml;
pub mod sdd;
pub mod triptych;
pub mod witness_bdd;

/// Path A: naive reference interpreter (source of truth).
pub fn lane_a(x: u32) -> u32 {
    x + 1
}

/// Path B: fixed-order DD (placeholder simulation).
pub fn lane_b(x: u32) -> u32 {
    x + 1
}

/// Path C: Lisage + planner (placeholder simulation).
pub fn lane_c(x: u32) -> u32 {
    x + 1
}

