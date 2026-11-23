---
name: "etude-grilles-2d"
title: "Étude « surcharge » V : grilles 2D, coutures croisées et question du coin"
type: "text/markdown"
---

# Étude « surcharge » V : grilles 2D, coutures croisées et question du coin

Cinquième volet de la série d'études sur l'annotation « surcharge » dans les diagrammes de décision de lisage. Après les anneaux (I), les protocoles (II), les workflows denses (III) et la synthèse FMS++ (IV), cette partie passe en 2 dimensions : c'est le pendant exact des philosophes de la partie I, où la symétrie translative de la chaîne devient une symétrie de grille, et où les coutures d'enroulement se croisent.

## 1. Modèle et variantes de frontière

Langage étudié : ensembles indépendants d'une grille n×n. Cellule {0,1}, garde locale (ajouter un 1 exige les 4 voisins à 0), retrait libre. Depuis l'état vide, tous les ensembles indépendants sont atteignables.

Pour isoler l'effet propre de chaque couture d'enroulement, quatre variantes de frontière :

| Variante | Axes enroulés | Coutures | Symétrie d'orbite |
|---|---|---|---|
| tore | les deux | 2 coutures croisées + coin | n² translations |
| cylindre | axe des lignes | 1 couture | n translations |
| plan | aucun | bords libres | aucune |
| tore pressé | les deux | idem tore + singularité | ancre = cellule irrégulière |

Le tore « pressé » reprend le protocole de la partie I : la cellule (0,0) a un alphabet {0,1,2} (module singleton, le « gaucher » 2D). La singularité casse la translation : l'ancre est la cellule irrégulière elle-même, plus aucune orbite n'est légitime.

Moteurs comme dans les parties précédentes : **plat** = MDD positionnel exact en ordre ligne-par-ligne ; **pool** = nombre de signatures distinctes (distances plafonnées aux 4 bords, W=2 ; lettre d'ancre ; fenêtre 5 cellules ; sentinelles -1 hors enroulement) sur les configurations ancrées par translation canonique (lex-min). Le pool compte les classes du moteur annoté, pas un moteur déployable.

**Question du coin** (motivation de la partie V) : quand deux coutures d'enroulement se croisent, la cellule de coin devient-elle une singularité qui oblige à payer une ancre par coin, ou le coin est-il bénin ? Protocole : partition des signatures en **A** (cellule entièrement plafonnée, intérieur) et **B** (touche au moins un bord plafonné).

## 2. Tore : deux coutures croisées

| n | \|S\| | ancré | plat | pool | A (int) | B (bord) |
|---|---|---|---|---|---|---|
| 2 | 7 | 3 | 8 | 11 | 0 | 11 |
| 3 | 34 | 6 | 39 | 43 | 0 | 43 |
| 4 | 743 | 56 | 225 | 158 | 0 | 158 |
| 5 | 25 531 | 1 027 | 1 159 | 300 | 17 | 283 |

Ratios plat successifs : 4,9 / 5,8 / 5,1. Taux ×5,5 constant : c'est la loi du « souffle de frontière » de la partie I, mais avec une base ~5,5 au lieu de ~2 : en 2D enroulée, le plat gonfle exponentiellement en la largeur. Le pool, lui, décélère nettement (×3,9 / ×3,7 / ×1,9) : il converge vers son plateau de saturation, comme pour les philosophes.

À n=6, la matrice de transfert donne |S| exact sans énumération : 2 406 862 (soit ×94 par rapport à n=5, alors que plat ne croît que de ×5,5 et le pool de ×2). L'énumération complète à n=6 a dépassé le budget du banc ; le comptage exact est néanmoins validé par double calcul (BFS et matrice de transfert coïncident à chaque n vérifié).

## 3. Cylindre puis plan : une seule couture suffit, zéro couture inverse la tendance

**Cylindre** (1 couture, axe des lignes) :

| n | \|S\| | ancré | plat | pool | A | B |
|---|---|---|---|---|---|---|
| 2 | 7 | 4 | 8 | 13 | 0 | 13 |
| 3 | 43 | 15 | 43 | 52 | 0 | 52 |
| 4 | 933 | 244 | 241 | 174 | 0 | 174 |
| 5 | 36 211 | 7 243 | 1 407 | 299 | 17 | 282 |

Ratios plat : 5,4 / 5,6 / 5,8. Quasi identique au tore, partout : même taux exponentiel, même pool (299 contre 300), mêmes classes intérieures. **Ajouter la seconde couture (crossing) ne coûte rien** : le coin du tore est bénin.

|S| n=6 exact (transfert) : 3 557 711.

**Plan** (bords libres, aucune couture) :

| n | \|S\| | ancré | plat | pool | A | B |
|---|---|---|---|---|---|---|
| 2 | 7 | 7 | 8 | 22 | 0 | 22 |
| 3 | 63 | 63 | 32 | 107 | 0 | 107 |
| 4 | 1 234 | 1 234 | 95 | 277 | 0 | 277 |
| 5 | 55 447 | 55 447 | 249 | 519 | 34 | 485 |

Ratios plat : 4,0 / 3,0 / 2,6, nettement décroissants : sans couture, le plat devient quasi polynomial (249 nœuds pour 55 447 états, record de la série). Et le pool croît plus vite que le plat : **dès n=3, 107 classes pour 32 nœuds plats** ; à n=5, 519 contre 249. Inversion complète : sans couture, il n'y a rien à monnayer, l'annotation est un lose. La translation n'est d'ailleurs plus une symétrie (aucune orbite), donc l'ancrage lui-même n'a plus de sens : le plan est simplement le territoire du plat.

## 4. Tore pressé : la singularité payante pour le pool, gratuite pour le plat

| n | \|S\| | plat | pool | A | B |
|---|---|---|---|---|---|
| 2 | 9 | 8 | 29 | 0 | 29 |
| 3 | 41 | 39 | 143 | 0 | 143 |
| 4 | 920 | 225 | 535 | 0 | 535 |
| 5 | 31 213 | 1 159 | 978 | 51 | 927 |

Deux résultats contre-intuitifs :

1. **Plat strictement identique au tore** (8/39/225/1 159) alors que |S| a grandi de 22 % (25 531 → 31 213). La lettre 2 force ses voisins à 0, donc le sous-arbre en aval de la valeur 2 est une copie contrainte d'un sous-arbre qui existait déjà : le MDD positionnel le partage gratuitement. Le plat absorbe la surcharge sans le savoir.
2. Le pool, lui, paie ×3,3 (978 contre 300). En détail : A intérieur = 51 = 3 × 17 exactement, la lettre d'ancre multiplie chaque classe ; B bord = 927 contre 283. La signature choisie (fenêtre 5 cellules autour de la cellule courante) ne voit pas la distance à la singularité : elle est sous-dimensionnée pour l'absorber.

C'est l'**inversion exacte du philosophes pressé de la partie I** (où l'annotation réduisait les entrées de 18 à 13). Là-bas, le pressé étendait les arcs et l'ancre-singularité monnayait la régularité ; ici, la lettre 2 contraint et le plat en profite, tandis que le pool, privé de translation légitime, ancre chaque configuration sur la singularité et paie plein tarif.

Leçon de conception : rentabilité de la surcharge ≠ propriété de la surcharge seule, mais du trio (surcharge, garde, moteur). Une lettre qui contraint enrichit le plat ; une lettre qui étend enrichit les classes. Et une ancre n'est utile que si elle factorise : ancre-singularité sans orbite = compteur de configurations, pas compresseur.

## 5. Réponse à la question du coin

**Le coin est bénin.** Trois preuves convergentes :

- **A** (signatures d'intérieur, entièrement plafonnées) n'existe qu'à partir de n=5 (première cellule à distance ≥ 2 des quatre bords) et reste minuscule : 17 (tore/cylindre), 34 (plan), 51 (pressé). Sur les 4 variantes, A plafonne dès son apparition.
- **B** (signatures touchant un bord) concentre la variété (plus de 92 % du pool à n=5) mais sa croissance décélère partout (dernier ratio ~1,75-1,9) : croissance résiduelle polynomiale le long du périmètre, pas singularité. Le pool plafonne car l'espace des signatures est fini par construction ; la convergence est déjà visible.
- **Cylindre ≈ tore** sur toutes les métriques : croiser une seconde couture sur la première n'ajoute ni taux, ni classes, ni aucune métrique de nuisance. La singularité redoutée du coin n'existe pas.

Conséquence pour la construction canonique : pas besoin d'ancres multiples par niveau ni de détection de coins au contrôleur. Une seule couture suffit à déclencher le souffle, et une seule ancre suffit à le monnayer. Le contrôleur ne doit détecter que la présence/l'absence d'enroulement, pas sa géométrie fine.

## 6. Synthèse croisée I à V

Quatrième loi de la série (après : l'ancrage seul ne suffit jamais ; l'écart plat/pool explose sur les langages filtrés mondiaux ; les arcs valués sont la surcharge utile des workflows denses) :

**En 2D, la couture, pas la symétrie, porte le gain.** La symétrie translative (grandes orbites) est déjà absorbée par l'ancrage classique ; ce qui fait gonfler le plat exponentiellement (×5,5/pas) est la couture d'enroulement, et une seule suffit. Symétriquement, là où il n'y a pas de couture (plan), le plat est quasi polynomial et l'annotation devient déficitaire dès n=3.

| Variante n=5 | plat | pool | plat/pool |
|---|---|---|---|
| tore | 1 159 | 300 | 3,9 |
| cylindre | 1 407 | 299 | 4,7 |
| tore pressé | 1 159 | 978 | 1,2 |
| plan | 249 | 519 | 0,5 |

Classement des taxonomies de surcharge complété : couture topologique (I, II, V), niveau partagé (II), arc valué (III, IV), synthèse de grappe (IV), et maintenant deux contre-exemples précieux : surcharge bénigne pour le plat (lettre contraignante, V) et ancre non factorisante (singularité sans orbite, V). Le contrôleur de lisage doit classer la rentabilité de l'annotation sur le trio (surcharge, garde, moteur) et non sur la seule régularité du modèle.

## 7. Limites et suites

Limites habituelles, plus spécifiques :

- n ≤ 5 en énumération complète (n=6 tore : 2,4 M états, hors budget du banc Python) ; les comptages exacts n=6 par matrice de transfert sont quand même validés par double source à n où l'énumération passe ;
- le pool est un compteur de classes, pas un moteur déployable : la question de la détection au contrôleur (partie "géométrie" de la série) reste ouverte sur le vrai pipeline ;
- la signature pressé (fenêtre 5) est sous-dimensionnée pour la singularité : enrichir la fenêtre d'une distance à l'ancre réduirait B, à tester ;
- proxy académique (ensembles indépendants), pas un modèle MCC réel : les diffusion 2D du MCC (Diffusion2D) ajoutent des quantités continues et des grilles rectangulaires, ce qui sort du banc ;
- à confirmer dans lisage-bench au jalon M7 face à OxiDD, comme pour les parties précédentes.

Suite annoncée (partie VI) : familles à surcharge mémoire, Referendum et NQueens, où la pathologie n'est ni la couture ni le span de garde mais l'historique (rung manquant).

Données et script de la partie V conservés avec le projet (results5.json, quatre variantes n=2 à 5, RSS max 25 Mo, moins de 2 s par n).