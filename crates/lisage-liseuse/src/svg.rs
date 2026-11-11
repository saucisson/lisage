//! Liseuse v0 — rendu **SVG statique** du diagramme (M1-9) : les niveaux en
//! lignes horizontales (niveau = clé de l'ordre), les nœuds rangés en x par
//! ordre de parcours déterministe, les terminaux en bas. Aucune dépendance
//! externe ; échappement XML minimal.

use std::collections::BTreeMap;

use lisage_core::{Form, Level, NodeId, Pool};

/// Largeur d'un nœud en pixels.
pub(crate) const LARGEUR_NOEUD: u32 = 56;
/// Hauteur d'un niveau en pixels.
pub(crate) const HAUTEUR_NIVEAU: u32 = 90;
/// Marge haute (titre) en pixels.
pub(crate) const MARGE_HAUT: u32 = 40;
/// Marge gauche en pixels.
pub(crate) const MARGE_GAUCHE: u32 = 40;
/// Demi-largeur d'un nœud.
pub(crate) const DEMI_LARGEUR: u32 = LARGEUR_NOEUD / 2;

/// Échappement XML minimal.
pub(crate) fn echapper_xml(texte: &str) -> String {
    texte
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Position d'un nœud : `(x, y)` du centre.
pub(crate) struct Placement {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

/// Collecte les nœuds atteignables depuis `racine`, groupés par niveau, et
/// calcule un placement déterministe (parcours en profondeur, x croissant
/// par ordre de première visite dans chaque niveau).
pub(crate) fn placer<P: Pool<Key = u64, Payload = bool>>(
    pool: &P,
    racine: NodeId,
) -> (BTreeMap<NodeId, Placement>, u32) {
    let mut niveaux: BTreeMap<u32, Vec<NodeId>> = BTreeMap::new();
    let mut visites: Vec<NodeId> = Vec::new();
    let mut pile = vec![racine];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(noeud) = pile.pop() {
        if !deja_vu.insert(noeud) {
            continue;
        }
        visites.push(noeud);
        if let Ok(Form::Node { level, arcs }) = pool.resolve(noeud) {
            niveaux.entry(level.0).or_default().push(noeud);
            // Empile dans l'ordre inverse pour visiter les arcs en ordre croissant.
            for cible in arcs.values().rev() {
                pile.push(*cible);
            }
        }
    }
    let nombre_niveaux = niveaux.len() as u32;
    let mut placement = BTreeMap::new();
    for (index_niveau, (_niveau, noeuds)) in niveaux.iter().enumerate() {
        let y = MARGE_HAUT + index_niveau as u32 * HAUTEUR_NIVEAU + HAUTEUR_NIVEAU / 2;
        for (index, noeud) in noeuds.iter().enumerate() {
            let x = MARGE_GAUCHE + index as u32 * (LARGEUR_NOEUD + 24) + DEMI_LARGEUR;
            placement.insert(*noeud, Placement { x, y });
        }
    }
    // Terminaux : ligne du bas.
    let y_terminal = MARGE_HAUT + nombre_niveaux * HAUTEUR_NIVEAU + HAUTEUR_NIVEAU / 2;
    let mut index_terminal = 0u32;
    for noeud in &visites {
        if !placement.contains_key(noeud) {
            let x = MARGE_GAUCHE + index_terminal * (LARGEUR_NOEUD + 24) + DEMI_LARGEUR;
            placement.insert(*noeud, Placement { x, y: y_terminal });
            index_terminal += 1;
        }
    }
    (placement, nombre_niveaux)
}

/// Rend le sous-diagramme atteignable depuis `racine` en SVG statique.
pub fn rendre_svg<P: Pool<Key = u64, Payload = bool>>(pool: &P, racine: NodeId) -> String {
    let (placement, nombre_niveaux) = placer(pool, racine);
    let largeur =
        MARGE_GAUCHE * 2 + placement.values().map(|p| p.x).max().unwrap_or(0) + DEMI_LARGEUR;
    let hauteur = MARGE_HAUT + (nombre_niveaux + 1) * HAUTEUR_NIVEAU + 20;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{largeur}\" height=\"{hauteur}\" viewBox=\"0 0 {largeur} {hauteur}\">\n"
    );
    svg.push_str(&format!(
        "<text x=\"{MARGE_GAUCHE}\" y=\"24\" font-family=\"serif\" font-size=\"14\">diagramme (niveaux = clés)</text>\n"
    ));
    // Arcs d'abord (sous les nœuds).
    let mut pile = vec![racine];
    let mut deja_vu = std::collections::BTreeSet::new();
    while let Some(noeud) = pile.pop() {
        if !deja_vu.insert(noeud) {
            continue;
        }
        if let Ok(Form::Node { level, arcs }) = pool.resolve(noeud) {
            let depart = &placement[&noeud];
            for (cle, cible) in arcs {
                let arrivee = &placement[cible];
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#555\"/>\n",
                    depart.x,
                    depart.y + 14,
                    arrivee.x,
                    arrivee.y - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"11\" fill=\"#777\">{}</text>\n",
                    (depart.x + arrivee.x) / 2,
                    (depart.y + arrivee.y) / 2,
                    echapper_xml(&cle.to_string())
                ));
                pile.push(*cible);
            }
            let _ = level;
        }
    }
    // Nœuds ensuite.
    for (noeud, position) in &placement {
        match pool.resolve(*noeud) {
            Ok(Form::Terminal(payload)) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"#eee\" stroke=\"#333\"/>\n",
                    position.x - DEMI_LARGEUR,
                    position.y - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">{}</text>\n",
                    position.x,
                    position.y + 4,
                    if *payload { "⊤" } else { "⊥" }
                ));
            }
            Ok(Form::Node { level, .. }) => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{LARGEUR_NOEUD}\" height=\"28\" fill=\"#fff\" stroke=\"#333\"/>\n",
                    position.x - DEMI_LARGEUR,
                    position.y - 14
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"12\" text-anchor=\"middle\">ℓ{} n{}</text>\n",
                    position.x,
                    position.y + 4,
                    level.0,
                    noeud.0
                ));
            }
            Err(_) => {}
        }
    }
    svg.push_str("</svg>\n");
    svg
}

/// Règle de niveau affichée (pour les tests et la légende).
pub fn etiquette_niveau(niveau: Level) -> String {
    format!("ℓ{}", niveau.0)
}
