//! A/B/C differential harness (M1-7), plugged in from the first commit:
//! - path A = reference interpreter `lisage-lang` (the truth);
//! - path B = DD with fixed order (placeholder until M1-1..M1-6 is delivered);
//! - path C = Lisage + planner (placeholder likewise).
//!
//! The real differential tests are written **red** (ignored) before
//! the core implementation and un-ignored in the issue that delivers the path.

use std::collections::BTreeSet;

use lisage_lang::interpreter;
use lisage_lang::value::Value;

/// Path A (reference): explicit set counting, source of truth.
pub fn voie_a_count(ensemble: &BTreeSet<Value>) -> u64 {
    interpreter::count(ensemble)
}

/// Path B (fixed-order DD): placeholder until the M1-1..M1-6 core exists.
pub fn voie_b_count(_ensemble: &BTreeSet<Value>) -> u64 {
    0
}

/// Path C (Lisage + planner): placeholder likewise.
pub fn voie_c_count(_ensemble: &BTreeSet<Value>) -> u64 {
    0
}
