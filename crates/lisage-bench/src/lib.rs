//! Lisage bench — differential, property-based, MCC, not-local events.

pub mod confrontation;
pub mod differential;
pub mod pnml;
pub mod sdd;
pub mod triptych;
pub mod witness_bdd;

/// Lane A: naive reference interpreter (source of truth).
pub fn lane_a(x: u32) -> u32 {
    x + 1
}

/// Lane B: fixed-order decision diagram (mock simulation).
pub fn lane_b(x: u32) -> u32 {
    x + 1
}

/// Lane C: Lisage + planner (mock simulation).
pub fn lane_c(x: u32) -> u32 {
    x + 1
}
