//! Vue mémoire et quotient (M4-5, doc liseuse § 5 « le pool en fond » et
//! « le zoom sémantique ») : pools groupés par plan avec leurs métriques
//! de GC, badges de multiplicité (replicas), classes du quotient
//! (formes × ancres), et mode comparaison naïf/DD/Lisage. Rendu SVG
//! statique, comme les autres vues.

use lisage_core::biplan::MetriquesPool;

/// Taille de police du titre.
const TAILLE_TITRE: u32 = 14;
/// Taille de police du corps.
const TAILLE_CORPS: u32 = 11;
/// Marge gauche en pixels.
const MARGE_GAUCHE: u32 = 16;
/// Hauteur d'une ligne de pool en pixels.
const HAUTEUR_LIGNE_POOL: u32 = 18;
/// Hauteur d'une ligne de classe du quotient en pixels.
const HAUTEUR_LIGNE_CLASSE: u32 = 18;
/// Hauteur d'une barre de comparaison en pixels.
const HAUTEUR_BARRE: u32 = 22;
/// Largeur maximale d'une barre de comparaison en pixels.
const LARGEUR_BARRE_MAX: u32 = 400;
/// Couleur des barres naïf/DD.
const COULEUR_BARRE_NEUTRE: &str = "#999";
/// Couleur de la barre Lisage (mise en avant).
const COULEUR_BARRE_LISAGE: &str = "#2a7";
/// Couleur du badge de multiplicité.
const COULEUR_BADGE: &str = "#c60";
/// Décalage vertical entre panneaux en pixels.
const DECALAGE_PANNEAU: u32 = 24;
/// Hauteur réservée au titre en pixels.
const HAUTEUR_TITRE: u32 = 40;
/// Marge basse en pixels.
const MARGE_BAS: u32 = 20;
/// Largeur du panneau de comparaison au-delà de la barre max, en pixels.
const LARGEUR_PANNEAU: u32 = 120;
/// Position verticale du premier panneau en pixels.
const Y_PREMIER_PANNEAU: u32 = 44;
/// Décalage horizontal du badge ×k dans une ligne de classe, en pixels.
const DECALAGE_BADGE: u32 = 220;
/// Hauteur intérieure d'une barre de comparaison en pixels.
const HAUTEUR_BARRE_INTERIEURE: u32 = 14;
/// Décalage horizontal des barres après le libellé, en pixels.
const DECALAGE_BARRE: u32 = 60;
/// Décalage vertical du rectangle de barre sous la ligne de texte, en pixels.
const DECALAGE_TEXTE_BARRE: u32 = 12;
/// Décalage horizontal de la valeur après la barre, en pixels.
const DECALAGE_VALEUR: u32 = 6;

/// Badge de multiplicité : un handle et combien de réplicas positionnels
/// il représente (badge ×k affiché quand k > 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BadgeMultiplicite {
    /// Libellé du handle (triple contrôleur/ancre/forme réduit).
    pub handle: String,
    /// Nombre de réplicas positionnels représentés.
    pub multiplicite: u64,
}

/// Classe du quotient : une forme sous une ancre, avec sa multiplicité
/// (formes × ancres — C1 : même forme sous deux ancres = deux classes).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClasseQuotient {
    /// Libellé de la forme.
    pub forme: String,
    /// Libellé de l'ancre.
    pub ancre: String,
    /// Nombre de réplicas dans la classe.
    pub multiplicite: u64,
}

/// Vue mémoire d'un pool : métriques de GC et vivants par plan.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VuePool {
    /// Nom du pool (contrôleur).
    pub nom: String,
    /// Métriques du pool (créations, partages, vivants, morts, GC,
    /// compactions, taux de hit).
    pub metriques: MetriquesPool,
    /// Vivants du plan principal.
    pub vivants_principal: u64,
    /// Vivants du plan d'ombre.
    pub vivants_ombre: u64,
}

impl ClasseQuotient {
    /// Badge de multiplicité de la classe (handle = « forme @ ancre »).
    pub fn badge(&self) -> BadgeMultiplicite {
        BadgeMultiplicite {
            handle: format!("{} @ {}", self.forme, self.ancre),
            multiplicite: self.multiplicite,
        }
    }
}

impl VuePool {
    /// Vue d'un pool depuis ses métriques et les vivants par plan.
    pub fn depuis_metriques(
        nom: &str,
        metriques: MetriquesPool,
        vivants_principal: u64,
        vivants_ombre: u64,
    ) -> Self {
        VuePool {
            nom: nom.to_string(),
            metriques,
            vivants_principal,
            vivants_ombre,
        }
    }
}

/// Comparaison naïf/DD/Lisage : nœuds vivants par mode de représentation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComparaisonVue {
    /// Nœuds vivants en énumération naïve (explicitation complète).
    pub naif: u64,
    /// Nœuds vivants en DD plat.
    pub dd: u64,
    /// Nœuds vivants en Lisage (formes partagées, O(1) + bord).
    pub lisage: u64,
}

/// Largeur d'une barre de comparaison, proportionnelle au compte (max =
/// `LARGEUR_BARRE_MAX`).
fn largeur_barre(compte: u64, max: u64) -> u32 {
    if max == 0 {
        0
    } else {
        (compte as u128 * LARGEUR_BARRE_MAX as u128 / max as u128) as u32
    }
}

/// Rendu SVG statique de la vue mémoire et quotient : panneau des pools
/// (métriques GC + vivants par plan), panneau des classes du quotient
/// (formes × ancres avec badges ×k), barres de comparaison naïf/DD/Lisage.
pub fn rendre_svg_vue_memoire(
    pools: &[VuePool],
    classes: &[ClasseQuotient],
    comparaison: &ComparaisonVue,
) -> String {
    let hauteur_pools = pools.len() as u32 * HAUTEUR_LIGNE_POOL + DECALAGE_PANNEAU;
    let hauteur_classes = classes.len() as u32 * HAUTEUR_LIGNE_CLASSE + DECALAGE_PANNEAU;
    let hauteur_comparaison = 3 * HAUTEUR_BARRE + DECALAGE_PANNEAU;
    let hauteur = HAUTEUR_TITRE + hauteur_pools + hauteur_classes + hauteur_comparaison + MARGE_BAS;
    let largeur = MARGE_GAUCHE * 2 + LARGEUR_BARRE_MAX + LARGEUR_PANNEAU;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"{TAILLE_TITRE}\">vue mémoire et quotient</text>\n"
    ));
    // Panneau des pools.
    let mut y = Y_PREMIER_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">pools (plan principal / ombre, GC)</text>\n"
    ));
    for pool in pools {
        y += HAUTEUR_LIGNE_POOL;
        let metriques = &pool.metriques;
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{} : vivants={}/{} morts={} gc={} compactions={} hit={}%</text>\n",
            pool.nom,
            pool.vivants_principal,
            pool.vivants_ombre,
            metriques.morts,
            metriques.gc,
            metriques.compactions,
            metriques.taux_hit()
        ));
    }
    // Panneau des classes du quotient.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">classes du quotient (formes × ancres)</text>\n"
    ));
    for classe in classes {
        y += HAUTEUR_LIGNE_CLASSE;
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{} @ {}</text>\n",
            classe.forme, classe.ancre
        ));
        let badge = classe.badge();
        if badge.multiplicite > 1 {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\" fill=\"{COULEUR_BADGE}\">×{}</text>\n",
                MARGE_GAUCHE + DECALAGE_BADGE,
                badge.multiplicite
            ));
        }
    }
    // Barres de comparaison naïf/DD/Lisage.
    y += DECALAGE_PANNEAU;
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">comparaison naïf / DD / Lisage (nœuds vivants)</text>\n"
    ));
    let max = comparaison.naif.max(comparaison.dd).max(comparaison.lisage);
    for (libelle, compte, couleur) in [
        ("naïf", comparaison.naif, COULEUR_BARRE_NEUTRE),
        ("DD", comparaison.dd, COULEUR_BARRE_NEUTRE),
        ("Lisage", comparaison.lisage, COULEUR_BARRE_LISAGE),
    ] {
        y += HAUTEUR_BARRE;
        let largeur = largeur_barre(compte, max);
        svg.push_str(&format!(
            "<text x=\"{MARGE_GAUCHE}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{libelle}</text>\n"
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{largeur}\" height=\"{HAUTEUR_BARRE_INTERIEURE}\" fill=\"{couleur}\"/>\n",
            MARGE_GAUCHE + DECALAGE_BARRE,
            y - DECALAGE_TEXTE_BARRE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{y}\" font-family=\"serif\" font-size=\"{TAILLE_CORPS}\">{compte}</text>\n",
            MARGE_GAUCHE + DECALAGE_BARRE + largeur + DECALAGE_VALEUR
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
