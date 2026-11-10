# Spike — FFI CUDD/Sylvan (M1-8, risque technique n° 1)

Statut : **spike documentaire** — aucune dépendance ajoutée (liste
d'escalation AGENTS.md : toute nouvelle dépendance exige un `needs-human`).
La question structurée est posée en commentaire de l'issue #14.

## Pourquoi des liaisons (rappel de la roadmap § risques)

L'interprète de référence ne voit que la sémantique, jamais la forme. La
« reproduction à l'identique » (C6) se mesure en **nombre de nœuds** contre
une bibliothèque indépendante : un mini-BDD maison partagerait les
malentendus du même auteur. Sylvan ajoute un second témoin indépendant
(portant plus tard saturation et LDD).

## Ce qui est livré sans FFI (mode dégradé, cette PR)

- Configuration du contrôleur en **BDD classique** (`lisage-core::bdd`) :
  arcs 0/1 explicites, pas de R-default, R-collapse + R-share seuls.
- **Mini-BDD témoin indépendant** pur Rust (`lisage-bench::temoin_bdd`),
  écrit exprès pour le différentiel : PBT C6 (nombre de nœuds identique +
  sémantique identique sur 2^n affectations, fonctions aléatoires).
- **Ligne de base de bench** : BDD classique n=12 (table LCG déterministe),
  comptage de nœuds ancré par criterion en CI.

## La vraie question : le build en CI

CUDD et Sylvan sont des bibliothèques C/C++ : il faut les compiler dans le
conteneur CI (ou fournir des images précompilées), sur toutes les
plateformes visées. Le hors-processus ne supprime pas le risque. En
in-process, le PBT différentiel est immédiat (fonction aléatoire → deux
constructions → comptes comparés sur place).

## Séquencement proposé (roadmap)

- **M1** : CUDD seul (témoin BDD classique, C6).
- **M2** : Sylvan quand la saturation (M2-4, à la Hamez) l'exige.

## Options pour la dépendance

| Option | Avantage | Inconvénient |
| --- | --- | --- |
| `cudd-sys` (bindgen, build CUDD en CI) | témoin externe réel, C6 fort | build CI lourd, plateformes |
| conteneur CI précompilé + binaire de comparaison hors-processus | CI simple | pas de PBT in-process, comparaison par fichiers |
| rester sur le mini-BDD interne (mode dégradé) | zéro dépendance | C6 affaibli (même auteur) |

## Impact si la question reste sans réponse

M1-8 est livré en mode dégradé (mini-BDD interne) ; les baselines externes
qui conditionnent les mesures publiées (M4-4, M5-2) restent à établir avec
le témoin FFI choisi.
