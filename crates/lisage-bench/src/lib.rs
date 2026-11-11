//! Lisage bench — différentiel, property-based, MCC, événements non locaux.

pub mod differentiel;
pub mod pnml;
pub mod sdd;
pub mod temoin_bdd;
pub mod triptyque;

/// Voie A : interprète de référence naïf (source de vérité).
pub fn voie_a(x: u32) -> u32 {
    x + 1
}

/// Voie B : DD à ordre fixe (simulation factice).
pub fn voie_b(x: u32) -> u32 {
    x + 1
}

/// Voie C : Lisage + planner (simulation factice).
pub fn voie_c(x: u32) -> u32 {
    x + 1
}
