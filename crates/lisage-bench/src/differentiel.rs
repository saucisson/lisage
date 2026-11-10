//! Harnais différentiel A/B/C (M1-7), branché dès le premier commit :
//! - voie A = interprète de référence `lisage-lang` (la vérité) ;
//! - voie B = DD à ordre fixe (factice tant que M1-1..M1-6 n'est pas livré) ;
//! - voie C = Lisage + planner (factice idem).
//!
//! Les tests différentiels réels sont écrits **rouges** (ignorés) avant
//! l'implémentation du cœur et dé-ignorés dans l'issue qui livre la voie.

use std::collections::BTreeSet;

use lisage_lang::interpreter;
use lisage_lang::value::Value;

/// Voie A (référence) : comptage ensembliste explicite, source de vérité.
pub fn voie_a_count(ensemble: &BTreeSet<Value>) -> u64 {
    interpreter::count(ensemble)
}

/// Voie B (DD ordre fixe) : factice tant que le cœur M1-1..M1-6 n'existe pas.
pub fn voie_b_count(_ensemble: &BTreeSet<Value>) -> u64 {
    0
}

/// Voie C (Lisage + planner) : factice idem.
pub fn voie_c_count(_ensemble: &BTreeSet<Value>) -> u64 {
    0
}
