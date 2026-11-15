//! Vtree as a first-class order (M5-3, claim C6): Darwiche's vtree is a
//! [`TypeOrder`] — canonical order = flat traversal of the vtree
//! (left before right), constraints = every left variable precedes every
//! right variable at each internal node. Sentential SDDs thus install as
//! an instance of the existing framework: the vtree is an order
//! **parameter**, not an *ad hoc* extension (proposal § 1.2, contribution n° 6).

use lisage_algebra::vtree::Vtree;
use lisage_lang::Atom;

use crate::order::TypeOrder;

impl TypeOrder for Vtree {
    type Key = Atom;

    fn keys_canoniques(&self) -> Vec<Atom> {
        self.order_plat()
    }

    fn contraintes(&self) -> Vec<(Atom, Atom)> {
        self.contraintes_order()
    }
}
