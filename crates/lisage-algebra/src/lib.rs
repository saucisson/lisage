//! Lisage algebra — niveau 2 : contrôleur étendu, transductions, réductions.
//!
//! Cette crate porte le contrat du cœur ([`interface`]) : le pool de bas
//! niveau (`lisage-core`) n'est qu'un stockage canonique implémentant ce
//! contrat.

pub mod interface;
pub mod reduction;
