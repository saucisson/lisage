//! Vtree as a first-class order (M5-3, claim C6): Darwiche's vtree is a
//! [`TypeOrdre`] — canonical order = flat traversal of the vtree
//! (left before right), constraints = every left variable precedes every
//! right variable at each internal node. Sentential SDDs thus install as
//! an instance of the existing framework: the vtree is an order
//! **parameter**, not an *ad hoc* extension (proposal § 1.2, contribution n° 6).

use lisage_algebra::vtree::Vtree;
use lisage_lang::Atom;

use crate::ordre::TypeOrdre;

impl TypeOrdre for Vtree {
    type Cle = Atom;

    fn cles_canoniques(&self) -> Vec<Atom> {
        self.ordre_plat()
    }

    fn contraintes(&self) -> Vec<(Atom, Atom)> {
        self.contraintes_ordre()
    }
}
