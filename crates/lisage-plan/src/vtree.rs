//! Vtree as a first-class order (M5-3, statement C6): Darwiche's vtree is
//! a [`TypeOrder`] — canonical order = flat traversal of the vtree (left
//! before right), constraints = every left variable precedes every right
//! variable at each internal node. Sentential SDDs thus fit as an instance
//! of the existing framework: the vtree is an order **parameter**, not an
//! *ad hoc* extension (proposal § 1.2, contribution no. 6).

use lisage_algebra::vtree::Vtree;
use lisage_lang::Atom;

use crate::order::TypeOrder;

impl TypeOrder for Vtree {
    type Key = Atom;

    fn keys_canonical(&self) -> Vec<Atom> {
        self.order_flat()
    }

    fn constraints(&self) -> Vec<(Atom, Atom)> {
        self.constraints_order()
    }
}
