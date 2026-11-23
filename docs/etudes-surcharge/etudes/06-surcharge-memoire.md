---
name: "etude-surcharge-memoire"
title: "Étude « surcharge » VI : référendum, N-reines et le rung mémoire"
type: "text/markdown"
---

# Étude « surcharge » VI : référendum, N-reines et le rung mémoire

Sixième volet de la série. Les parties I à V avaient étudié des langages dont toute la sémantique se lit dans une fenêtre bornée : anneaux, protocoles à jetons, workflows, grilles 2D. Le partage y était une question de coût, et l'annotation « surcharge » monnayait la régularité (coutures, niveaux partagés, arcs valués, grappes). Cette partie attaque le rung resté vide de la taxonomie : la **mémoire**, sur deux familles repérées dans le catalogue MCC au tour précédent (Referendum, NQueens), où la pathologie n'est ni la couture ni le span de garde, mais l'information qui n'est pas dans la fenêtre.

Deux proxys académiques volontairement opposés :

- **Référendum** : dynamique libre, requête globale. N votants en anneau (topologie de la partie I), chaque votant passe de « non-voté » à oui ou non, vote figé. La dynamique est sans garde ; la requête (« le scrutin est-il accepté ? ») est un agrégat de comptage sur toute la configuration.
- **N-reines** : garde filtrante globale, requête triviale. Grille N×N, poser une reine exige l'absence de conflit ligne, colonne ou diagonale avec toute reine présente (portée N, non bornée) ; retrait libre.

Moteurs comme dans les parties précédentes : **plat** = MDD positionnel exact en ordre naturel ; **pool** = classes de signatures locales (fenêtre bornée). S'y ajoutent deux instruments propres à la mémoire : les classes **avec multiplicités** (multiset des types locaux, l'équivalent d'arcs valués de comptage) et l'**empreinte de garde** (registre d'occupation : lignes, colonnes, diagonales occupées).

## 1. Référendum : la fenêtre sait, mais seulement si on la laisse compter

| N | \|S\| = 3^N | plat | plat+requête | compteur | pool multiset | pool set | amb. set | déc. amb. set |
|---|---|---|---|---|---|---|---|---|
| 2 | 9 | 2 | 7 | 6 | 6 | 6 | 0 | 0 |
| 4 | 81 | 4 | 15 | 15 | 24 | 24 | 0 | 0 |
| 6 | 729 | 6 | 25 | 28 | 127 | 127 | 0 | 0 |
| 7 | 2 187 | 7 | 31 | 36 | 300 | 291 | 6 | 2 |
| 8 | 6 561 | 8 | 37 | 45 | 726 | 702 | 14 | 2 |
| 9 | 19 683 | 9 | 44 | 55 | 1 706 | 1 583 | 74 | 20 |
| 10 | 59 049 | 10 | 51 | 66 | 3 945 | 3 563 | 205 | 34 |

Quatre lectures, dont deux contre-intuitives :

1. **Le plat est imbattable** : N nœuds (10 pour 59 049 états à N=10). Langage sans garde = produit cartésien parfaitement partagé. Comme le plan de la partie V, mais à l'état pur : la dynamique seule ne laisse rien à monnayer.
2. **Le plat absorbe la requête sans exploser** : ajouter la transition « conclure » (accepté si oui strictement majoritaire, refusé sinon) multiplie le plat par ~N/2 (51 à N=10), pas par 3^N. Le MDD positionnel paie le comptage en quadratique léger, en partageant les chemins par leur solde partiel. La pathologie attendue (un « éclatement du comptage ») n'existe pas au niveau de l'espace.
3. **Les types sans multiplicités sont incorrects** : dès N=7, une même classe locale (frozenset des triplets voisins) contient des configurations au compte différent ; à N=10, 205 classes ambiguës dont 34 avec décision ambiguë, jusqu'à 5 valeurs de compte dans une même classe. La fenêtre bornée « brute » ne porte pas l'agrégat : c'est une perte d'expressivité, pas de performance.
4. **Les types avec multiplicités sont corrects** : ambiguïté 0 pour tout N, parce que chaque cellule est exactement une fois « centre » d'un triplet : la marge centrale du multiset donne le compte exact. Mais le multiset réencode presque toute la configuration (3 945 classes à N=10, croissance ×2,4 par pas) : ce n'est pas un moteur, c'est une preuve.

Le vrai rung est le **compteur partagé** : (nombre de votes, nombre d'oui), soit (N+1)(N+2)/2 classes, exactement la sémantique d'arcs valués à la EV-MDD. Il est de l'ordre de grandeur du plat+requête (66 contre 51 à N=10) : ici la surcharge mémoire n'achète pas de l'espace, elle achète une garantie : **la décision de comptage calculable en local** (évaluer « conclure » en O(1) par transitions valuées, au lieu de traverser la configuration).

Verdict VI-A : pour une requête agrégative globale, la fenêtre bornée sans multiplicités est fausse ; la multiplicité (le comptage sur les arcs) la répare. Le compteur est une surcharge d'expressivité, quasi gratuite en espace.

## 2. N-reines : la fenêtre bornée est fausse, et personne ne compresse

| N | \|S\| | pleines | plat (lignes) | plat (diagonales) | pool fenêtre | erreurs fenêtre | empreintes |
|---|---|---|---|---|---|---|---|
| 4 | 87 | 2 | 135 | 135 | 164 | 20,9 % | 86 |
| 5 | 462 | 10 | 573 | 559 | 290 | 26,6 % | 452 |
| 6 | 2 635 | 4 | 2 896 | 2 786 | 290 | 34,2 % | 2 590 |
| 7 | 16 870 | 40 | 15 180 | 14 220 | 290 | 39,9 % | 16 648 |

Contrôles : |S| énuméré sans aucun conflit ; les configurations à N reines égalent les solutions classiques (2, 10, 4, 40) à chaque N, en accord avec la référence externe usuelle du comptage des N-reines ([OEIS A000170](https://oeis.org/A000170)) : double validation de l'énumération.

Lectures, toutes inverses du référendum :

1. **Le plat n'explose pas : il s'effondre.** Aux petits N il est *plus gros que l'espace d'états* (135 nœuds pour 87 états en N=4) ; à N=7 il frôle la liste (15 180 pour 16 870). La contrainte de conflit traverse tout l'échiquier : le partage de suffixes est quasi nul. À comparer au plan de la partie V : 249 nœuds pour 55 447 états ; la différence n'est pas la finesse du langage, c'est la **portée des gardes** dans l'ordre des variables.
2. **L'ordonnancement n'y peut rien** : parcourir les cellules diagonale par diagonale ne gagne que 6 % (14 220 contre 15 180). Contrairement au FMS de la partie IV (où le Θ(K²) était un artefact d'ordre), ici aucune géométrie de balayage ne localise une contrainte de portée N.
3. **La mémoire de garde ne compresse pas les états** : l'empreinte d'occupation (lignes, colonnes, deux diagonales occupées) est quasi injective (98,7 % à N=7). Elle est nécessaire pour évaluer les gardes futures, mais deux configurations distinctes gardent presque toujours des empreintes distinctes, parce que le retrait exige la position exacte. Un « nœud mémoire » séparé n'existe pas : la mémoire et l'état coïncident.
4. **La fenêtre bornée est incorrecte, et son plateau le prouve** : le pool de signatures sature à 290 dès N=5 (vérifié identique en N=6 et N=7) alors que l'espace fait ×6 par pas ; dans le même temps, le taux de faux positifs de la fenêtre (elle croit un ajout légal, la sémantique le refuse) croît : 20,9 %, 26,6 %, 34,2 %, 39,9 %. Un moteur local serait compact, faux, et de plus en plus faux : le pire assemblage possible.

La courbe des portées clôt le diagnostic (part des paires (configuration, cellule) mal prédites par une fenêtre de Chebyshev de rayon R) :

| rayon R | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| N=6 | 34,2 % | 7,1 % | 1,2 % | 0,07 % | 0 % | |
| N=7 | 39,9 % | 9,3 % | 1,9 % | 0,2 % | 0,01 % | 0 % |

L'erreur décroît d'un facteur ~4-5 par unité de portée, mais ne s'annule qu'à R = N-1 : la portée de correction est exactement la taille de la grille. C'est structurel : une reine menace l'autre bout de sa diagonale.

Verdict VI-B : pour un filtre global ensembliste, la fenêtre bornée est fausse (et son plateau, signe d'aveuglement, pas de maîtrise), l'empreinte de garde est exact mais non compressante, et le plat seul tient le langage en portant lui-même la mémoire des diagonales. La surcharge utile n'est pas une annexe du diagramme, elle **est** le diagramme.

## 3. Le rung mémoire : deux formes, un diagnostic

La taxonomie des surcharges des parties I à V (couture, niveau partagé, arc valué, synthèse de grappe) se complète d'un rung à deux visages :

- **Compteur** (référendum) : l'information globale est une somme. La surcharge = multiplicités sur les arcs (arcs valués, EV-MDD). Peu coûteuse, articulée sur le plat, corrige l'incorrection de la fenêtre.
- **Registre d'occupation** (N-reines) : l'information globale est un ensemble de sièges occupés. La surcharge = le partage des niveaux même, ce que la construction positionnelle fait déjà implicitement quand l'ordre des variables coupe bien les contraintes. Elle n'est pas séparable de l'état (les retraits exigent la position).

Dans les deux cas la leçon commune dépasse les deux familles : **quand la garde (ou la requête) est globale, la fenêtre bornée n'est pas chère, elle est fausse.** Les parties I à V n'ont jamais vu cela parce que leurs gardes restaient de portée bornée dans la géométrie de l'ordre des variables ; le moment où cette condition lâche, l'économie des signatures bascule de « rentable » (I, II, V-tore) ou « déficitaire mais correcte » (V-plan) à « incorrecte » (VI).

## 4. Synthèse croisée I à VI

Cinquième loi de la série, après (I) l'ancrage seul ne suffit jamais, (II) l'écart plat/pool explose sur les langages filtrés mondiaux, (III-IV) les arcs valués et la synthèse de grappe sur les workflows denses, (V) la couture, pas la symétrie, porte le gain :

**La portée des gardes dans l'ordre des variables décide de tout.** Gardes à portée bornée (I à V) : le plat partage bien ou mal selon les coutures, et la surcharge annote, factorise, monnaye. Gardes ou requêtes à portée N (VI) : le partage du plat lui-même dépend de la largeur de coupe ; la fenêtre bornée perd l'expressivité ; seul le rung mémoire (compteur ou registre) rétablit la correction, et il se confond parfois avec le plat.

Conséquence pour lisage : le contrôleur doit classifier les gardes **avant** de choisir le rung : portée bornée (fenêtre), agrégation (compteur valué), ensemble diffus (registre d'occupation). Un contrôleur qui annoterait N-reines comme il annote Diffusion2D produirait un diagramme compact et faux.

## 5. Limites

- Proxys académiques, pas les instances MCC réelles (le Referendum de MCC répartit ses votants et son décideur dans un réseau ; NQueens est une grille-puzzle) : à confirmer sur les vraies familles via lisage-bench ;
- le N=8 des N-reines (|S| attendu ~100 000) n'a pas été énuméré, les tendances étant stables dès N=7 ; pas de double formule combinatoire fermée pour |S| partiel (contrôle par invariants : zéro conflit énuméré, solutions complètes exactes) ;
- le taux d'erreur de fenêtre mesure l'incorrection d'un prédicteur local simple, pas d'une architecture complète à mémoire locale apprise ;
- le plat+requête du référendum mesure la taille, pas le coût d'évaluation des transitions, qui est précisément là que le compteur valué gagne (à la Ciardo/Siminiceanu) ;
- comme toujours, le pool compte des classes, pas un moteur déployable ; le jalon M7 (confrontation OxiDD, oracle du critère de rentabilité C4) reste l'arbitre.

## 6. Suites

- Appliqué, le diagnostic contrôle la suite : la classification automatique de la portée des gardes (détection « fenêtre / compteur / registre » au niveau du contrôleur lisage-lang) devient le candidat naturel de la partie VII, avec le vrai FMS de la partie III comme terrain de test du rung mémoire ;
- confrontation lisage-bench M7 : mesurer si les arcs valués (compteur) réduisent l'évaluation des transitions du référendum réel de N votants, comme le suggère le verdict VI-A.

Données de la partie VI conservées avec le projet (results6.json : référendum N=2 à 10, N-reines N=4 à 7, portées de fenêtre N=6 et N=7 ; banc quelques secondes, RSS max 43 Mo).