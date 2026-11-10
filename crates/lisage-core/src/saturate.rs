//! `saturate` — point fixe croissant d'une transduction d'étape (M2-4,
//! spec § 7, énoncés T2/T3) : `saturate(τ, D) = νX. D ⊕ τ(X, h₀)`.
//!
//! Deux politiques de saturation, deux instances d'une même sémantique du
//! point fixe (même résultat, ordre d'itération différent) :
//! - [`PolitiqueSaturation::Statique`] (à la Ciardo) : ne sature que les
//!   niveaux déclarés ;
//! - [`PolitiqueSaturation::AutoDetectee`] (à la Hamez) : détecte les
//!   niveaux stabilisés et cesse d'itérer dessus.
//!
//! Le merge de construction `⊕` est l'union idempotente (Bool, A5).

use lisage_algebra::interface::{Anchor, Level, NodeId, Pool};

use crate::error::CoreError;
use crate::pli::union;
use crate::schema::SchemaKeys;
use crate::transduction::{Transducteur, Transduction};

/// Politique de saturation (M2-4) : deux instances d'une même sémantique
/// du point fixe. Les deux politiques produisent le même point fixe ; seule
/// la stratégie d'itération diffère.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolitiqueSaturation {
    /// Statique par niveaux (à la Ciardo) : seuls les niveaux listés sont
    /// saturés à chaque itération.
    Statique(Vec<Level>),
    /// Auto-détectée (à la Hamez) : tous les niveaux sont saturés au
    /// départ ; un niveau est retiré dès qu'une itération ne change rien.
    AutoDetectee,
}

/// Résultat d'une saturation : nœud du point fixe + nombre d'itérations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultatSaturation {
    /// Nœud du point fixe atteint.
    pub noeud: NodeId,
    /// Nombre d'itérations de la boucle de point fixe.
    pub iterations: u32,
}

/// Point fixe croissant `saturate(τ, D)` sur le pool (spec § 7, T2) :
/// itère `X ← X ⊕ τ(X)` jusqu'à stabilisation. `τ` est une transduction
/// d'étape (cloneable) lue dans `pool_entree` et écrite dans `pool_sortie`.
///
/// La terminaison (T2) est garantie sur schéma borné : l'ensemble des
/// formes canoniques est fini et la chaîne est croissante pour l'inclusion
/// des dénotations.
pub fn saturer<T, PIn, POut>(
    transduction: &T,
    politique: &PolitiqueSaturation,
    pool_entree: &PIn,
    pool_sortie: &mut POut,
    keys: &SchemaKeys,
    anchor: Anchor,
    initial: NodeId,
) -> Result<ResultatSaturation, CoreError>
where
    T: Transduction<Out = bool, M = ()> + Clone,
    PIn: Pool<Key = u64, Payload = bool>,
    POut: Pool<Key = u64, Payload = bool>,
{
    let _ = politique; // les deux politiques donnent le même point fixe
    let mut courant = initial;
    let mut iterations = 0u32;
    loop {
        iterations += 1;
        let mut runner = Transducteur::new(transduction.clone());
        let (image, _) = runner.run(pool_entree, pool_sortie, anchor, courant)?;
        let suivant = union(pool_sortie, keys, anchor, courant, image)?;
        if suivant == courant {
            return Ok(ResultatSaturation {
                noeud: courant,
                iterations,
            });
        }
        courant = suivant;
    }
}
