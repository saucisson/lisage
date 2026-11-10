//! Valeurs du niveau 1 : atomes bornés, ensembles, maps (instance de
//! référence bornée, spec § 1).

use std::collections::{BTreeMap, BTreeSet};

/// Atome borné : type distinct (ADR-0006), domaine fini énumérable déclaré
/// par le schéma (M1-3).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Atom(pub u64);

/// Valeur du langage de référence (algèbre de types Σ, instance bornée).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Value {
    Atom(Atom),
    Set(BTreeSet<Value>),
    Map(BTreeMap<Value, Value>),
    /// Séquence ordonnée d'éléments (M2-1, constructeur `list`).
    Seq(Vec<Value>),
}
