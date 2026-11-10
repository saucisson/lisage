//! Lisage lang — niveau 1 : types, interprète de référence.
//!
//! L'interprète de référence (M1-7) est la vérité sémantique, en pur
//! `lang/` sans aucun diagramme de décision : voie A du triptyque.

pub mod error;
pub mod interpreter;
pub mod value;

pub use error::{KeyName, LangError};
pub use value::{Atom, Value};
