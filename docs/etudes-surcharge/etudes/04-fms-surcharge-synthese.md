---
name: "etude-fms-surcharge-synthese"
title: "Étude « surcharge » IV : FMS — synthèse de grappes et arcs valués"
type: "text/markdown"
---

# Étude « surcharge » — partie IV : FMS, peut-on faire mieux ?

Retour sur l'anomalie de la partie III : le FMS était le seul workflow dont les nœuds plats croissaient en ~12K² (les autres sont linéaires). Question : est-ce une dureté du modèle, ou un artefact ? Et l'idée d'annotation de surcharge, sous une autre forme, peut-elle faire mieux que la couture et la valuation ?

## 1. Diagnostic : un artefact d'ordonnancement, pas une dureté

La contrainte d'étage du FMS-lite (a + b + machine-occupée ≤ K) porte sur trois variables que l'ordre plat entrelace : (a0, a1, a2, b0, b1, b2, m0, m1, m2). Le MDD doit donc porter le contexte (sommes partielles (a_i, b_i)) en travers des niveaux intermédiaires : Θ(K²) classes de nœuds. La preuve que ce n'est pas intrinsèque : le **jeu accessible factorise exactement**, même après ajout d'une **grue partagée** (capacité C entre les étages) :

**|S| = f(K)³ · g(C)², vérifié exactement aux trois points** : f(K) = T(K+1)+K(K+1) états d'étage (5, 12, 22), g(C) = (C+1)(C+2)/2 états de saut. Mesuré : 4 500, 62 208, 383 328 avec C = 2. La grue couple les transitions, pas les états.

## 2. L'escalier de représentations (K = 1, 2, 3 ; C = 2)

| rung | représentation | nœuds (K=1/2/3) | arcs (K=1/2/3) | loi |
|---|---|---|---|---|
| R0 | plat entrelacé (l'ordre de la partie III) | 53 / 92 / 155 | 96 / 199 / 410 | Θ(K²) |
| R1 | grappes contiguës (étage, saut, étage, saut, étage) | 23 / 26 / 29 | 45 / 57 / 72 | Θ(K) |
| R2 | **agrégré par grappe** (5 super-variables) | **5 / 5 / 5** | 27 / 48 / 78 | **O(1) nœuds** |
| R3 | R2 + **arcs valués** | 5 | **5** | O(1), tout |

Les trois pentes mesurées : R0 = quadratique (51 nœuds de pente apparente sur 3 points), R1 = 3K+20, R2 = 5, exactement 5 à chaque K — un nœud par grappe plus le terminal.

## 3. Les deux nouvelles surcharges

Ce que la partie IV ajoute à l'idée d'annotation de surcharge :

- **Surcharge de synthèse (R2).** Ici l'annotation ne casse pas l'homogénéité d'un niveau : elle **contracte une grappe de variables en un seul nœud annoté de son alphabet composé** — l'étage (a, b, m) devient UNE variable à f(K) valeurs, le saut de grue (x, y) une variable à g(C) valeurs. En terms lisage : c'est la transduction synthétisée — le module ferme localement son contrat et n'exporte que son interface. Nœuds : Θ(K²) → 5.
- **Surcharge valuée (R3).** Il reste les arcs : le nœud d'étage a f(K) ≈ Θ(K²) arcs frères (un par valeur de l'alphabet composé). Une transduction de plage les replie en un arc annoté. Arcs : Θ(K²) → 1 par nœud, 5 au total. C'est la lignée des arcs valués, déjà nommée en partie III.

Combinées : **5 nœuds et 5 arcs annotés pour représenter 383 328 états** (à K=3, C=2) — et la taille de la représentation ne dépend plus du tout de K.

## 4. Détection au contrôleur : le critère, c'est le span

Le signal statique qui annonce toute l'affaire : le **span des gardes** — la distance maximale, dans l'ordre des variables, entre membres d'une même contrainte. Dans R0, la contrainte d'étage a un span de 6 (a0 … m0) et les déchargements machine→grue traversent presque tout le fichier. Règle de classifieur qui en découle, lisible avant construction :

1. span faible et grappes déjà contiguës → rien à faire (les autres workflows de la partie III) ;
2. spans larges sur des grappes identifiables → **réordonnancer**, gratuit (R1 : facteur 5 à 6 sur les nœuds des petites instances, et le changement de loi K² → K) ;
3. grappe dotée d'un alphabet composé localement fermé → **surcharge de synthèse** (R2 : loi K → 5) ;
4. arcs plages restants → **surcharge valuée** (R3 : 5).

Même morale que les parties précédentes : le plan sort du contrôleur avant tout BFS, ici par simple lecture des spans de gardes.

## 5. Réponse à la question, et sa limite

**Oui, on peut faire beaucoup mieux sur le FMS** — mais le mieux n'est pas une couture : c'est la **contraction**. La taxonomie des surcharges, complétée :

| surcharge | cible | partie |
|---|---|---|
| couture topologique | asymétrie de module/marquage sur topologie régulière | I, II |
| niveau partagé (victimes/jeton) | objet global circulante | II |
| valuée (plage d'arcs) | largeur de marquage | III, IV |
| **synthèse (grappe → 1 variable annotée)** | contraintes localement fermées éclatées par l'ordre | **IV** |

La limite honnête : le FMS-lite-ci dessus factorise (l'assert exacte le prouve), donc R2 retombe sur un pur produit. Le **vrai FMS** de Ciardo garde des pools de machines partagés par plusieurs flux (les machines M1 à M4 servent des opérations d'étages différents) : la factorisation n'y sera jamais totale. Le rung manquant, prévu mais non mesuré ici, est la **surcharge mémoire** : porter la capacité résiduelle du pool comme attribut d'arc (transduction mémoire de lisage) entre grappes agrégées — quelques nœuds, quelques arcs, un attribut compté. C'est la manipulation à bencher en premier dans `lisage-bench` (M7) sur le PNML réel du FMS du MCC, avec l'escalier R0→R3 comme protocole de mesure.

## 6. Limites

- Toy : 3 étages, grue à deux compteurs, K ≤ 3 (le BFS explicite plafonne à ~4·10⁵ états ici) ; les constantes (5, 27, 78) sont celles du proxy.
- Les arcs R2 (27, 48, 78 = f(K)+g(C)+f(K)+g(C)+f(K)) suivent exactement les tailles d'alphabet : la mesure est fermée confirmée.
- Le span est un critère heuristique : il détecte l'opportunité d'ordonnancement et d'agrégation ; le choix entre R1 et R2 se joue sur la taille de l'alphabet composé vs le contexte porté, à trancher par l'oracle de rentabilité (C4), pas par règle fixe.