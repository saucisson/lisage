//! Lisage lang — level 1: types, reference interpreter.
//!
//! The reference interpreter (M1-7) is the semantic truth, in pure
//! `lang/` without any decision diagram: path A of the triptych.

pub mod error;
pub mod interpreter;
pub mod value;

pub use error::{KeyName, LangError};
pub use value::{Atom, Value};
