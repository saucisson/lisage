//! Differential harness A/B/C (M1-7), wired in from the first commit:
//! - lane A = reference interpreter `lisage-lang` (the truth);
//! - lane B = fixed-order DD (mock until M1-1..M1-6 is delivered);
//! - lane C = Lisage + planner (ditto).
//!
//! The real differential tests are written **red** (ignored) before the
//! core is implemented and one-ignored in the issue that delivers the lane.

use std::collections::BTreeSet;

use lisage_lang::interpreter;
use lisage_lang::value::Value;

/// Lane A (reference): explicit set counting, source of truth.
pub fn lane_a_count(set: &BTreeSet<Value>) -> u64 {
    interpreter::count(set)
}

/// Lane B (fixed-order DD): mock until the M1-1..M1-6 core exists.
pub fn lane_b_count(_set: &BTreeSet<Value>) -> u64 {
    0
}

/// Lane C (Lisage + planner): ditto.
pub fn lane_c_count(_set: &BTreeSet<Value>) -> u64 {
    0
}
