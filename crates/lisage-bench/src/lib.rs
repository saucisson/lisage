//! Lisage bench — differential, property-based, MCC, non-local events.

pub mod confrontation;
pub mod differentiel;
pub mod pnml;
pub mod sdd;
pub mod temoin_bdd;
pub mod triptyque;

/// Path A: naive reference interpreter (source of truth).
pub fn voie_a(x: u32) -> u32 {
    x + 1
}

/// Path B: fixed-order DD (placeholder simulation).
pub fn voie_b(x: u32) -> u32 {
    x + 1
}

/// Path C: Lisage + planner (placeholder simulation).
pub fn voie_c(x: u32) -> u32 {
    x + 1
}
