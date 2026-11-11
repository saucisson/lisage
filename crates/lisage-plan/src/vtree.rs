//! Vtree comme ordre de première classe (M5-3, énoncé C6) : le vtree de
//! Darwiche est un [`TypeOrdre`] — ordre canonique = parcours plat du
//! vtree (gauche avant droite), contraintes = toute variable de gauche
//! précède toute variable de droite à chaque nœud interne. Les SDD
//! sententiels s'installent ainsi comme instance du cadre existant :
//! le vtree est un **paramètre** d'ordre, pas une extension *ad hoc*
//! (proposition § 1.2, contribution n° 6).

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
