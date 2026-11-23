---
name: "etude-surcharge-protocoles"
title: "Étude « surcharge » II : protocoles moins réguliers (Peterson & co)"
type: "text/markdown"
---

# Étude « surcharge d'homogénéité » — partie II : protocoles moins réguliers

Suite de la partie I (anneaux : Philo, Pressé, Jeton). Même machinerie (plat = MDD exact positionnel, ancré = MDD exact sur l'ensemble ancré par rotation, fenêtre = quotient à distance bornée), appliquée cette fois aux familles « tier-2 » : exclusion mutuelle par mémoire partagée.

## 1. Familles

| Famille | Quoi | Asymétrie |
|---|---|---|
| **pet2** | Peterson exact, 2 processus (flags + tour) | n figé : swap P0/P1 + flip du tour |
| **dek2** | Dekker exact, 2 processus | idem |
| **petN** | Peterson généralisé n processus = lock « filtre » : n−1 niveaux, tableau de victimes V[ℓ] partagé, gardes ID-dépendantes (V[ℓ] == i) | le tableau partagé + circulation du « dernier arrivé » : le tier-2 du débat (Anderson, Eisenberg-McGuire) |
| **petD** | petN + chaîne privée de longueur i+1 par processus | **irrégularité diffuse** : n classes singleton, contrôle négatif |

Validations : exclusion mutuelle vérifiée sur tous les états atteints (jamais violée) ; pour petN, la symétrie rotationnelle est **exacte** : |S| = n · |ancré-sans-doublon| à tous les n (5/10, 27/81, 219/876, 2273/11365, 28123/168738), aucune orbite dégénérée.

## 2. Instances figées (pet2, dek2)

| Modèle | \|S\| | plat |
|---|---|---|
| pet2 | 21 | 21 |
| dek2 | 25 | 20 |

Lecture : à n = 2 constant, il n'y a rien à plier. Le quotient par swap achète au mieux un facteur 2 sur les états. Ces instances servent de garde-fous (l'exclusion mutuelle y est vérifiée), pas de courbes.

## 3. Peterson généralisé (petN) : le résultat fort

| n | \|S\| | plat | ancré (exact) | pool fenêtre | formes |
|---|---|---|---|---|---|
| 2 | 10 | 7 | 5 | 10 | 3 |
| 3 | 81 | 48 | 25 | 42 | 4 |
| 4 | 876 | 328 | 117 | 152 | 5 |
| 5 | 11 365 | 2 753 | 739 | 415 | 6 |
| 6 | 168 738 | 27 462 | 5 949 | 556 | 7 |

Trois enseignements nets :

1. **Le plat explose.** Pente fictive 5 761 nœuds/n : ce n'est plus linéaire, c'est superlinéaire. À n = 6 le MDD exact garde 27 462 nœuds pour 168 738 états (ratio 16 %) : le partage par suffixe ne compresse presque plus rien dès que les gardes quantifient sur **tous** les processus (plus de localité). Même l'ancré exact dégringole (5 949) : l'ancrage sans fenêtre ne sauve pas plus ici que sur les anneaux.
2. **Le quotient paie immédiatement.** Point de croisement entre n = 3 (48 contre 42) : là où les anneaux de la partie I demandaient n ≈ 30, les protocoles paient dès les tailles de jouet. Et l'écart devient brutal : ×6,6 en nœuds à n = 5, **×49 à n = 6**, et il s'élargit super-exponentiellement (plat suit |S| de près, le pool suit une loi polynomiale).
3. **Mais le pool ne plafonne plus.** Le plateau O(1) de la partie I est parti : 10 → 556, croissance ~quadratique. La raison est structurelle, pas un raté : le nombre de **niveaux** du filtre croît avec n (n−1 victimes, formes = n+1). Le paramètre d'échelle de petN est **structurel**, pas topologique : l'alphabet lui-même est fonction de n.

## 4. Contrôle négatif (petD) : irrégularité diffuse

| n | \|S\| | plat | pool fenêtre |
|---|---|---|---|
| 2 | 14 | 18 | 14 |
| 3 | 166 | 129 | 48 |
| 4 | 2 512 | 1 042 | 120 |
| 5 | 44 248 | 10 052 | 245 |

Nuance honnête : même le diffus voit le pool croître bien plus lentement que le plat (245 contre 10 052 à n = 5). Mais le décompte de classes est ici **optimiste** : sans symétrie de rotation, la représentation à fenêtre ne se re-déploie pas en un moteur exact sans décorer chaque position (ce que le classifieur refuse, verdict « flat »). Le gain apparent du diffus n'est pas monnayable ; celui de petN l'est (symétrie exacte vérifiée ci-dessus).

## 5. Verdicts du classifieur (avant construction)

| Famille | Diagnostic statique | Plan rendu |
|---|---|---|
| pet2, dek2 | 2 modules, 1 classe à renommage (swap + flip du tour) près | ancre-swap : facteur 2, rien d'autre |
| petN | 1 classe à rotation près (les gardes ne référencent l'ID que via V[ℓ] == i) ; le porteur du plus haut niveau circule | **ancre dynamique** + **surcharge du niveau partagé** (tableau de victimes), pas de duplication positionnelle |
| petD | n classes singleton (chaînes privées de longueurs distinctes) | **flat assumé, aucune injection** : la décoration coûterait plus qu'elle n'achète |

À rapprocher de la partie I : les trois plans ont à nouveau été rendus avant tout BFS, et le diagnostic de petN (« décorer le niveau de l'objet partagé, laisser le reste se replier ») est exactement la surcharge de niveau discutée : ici elle ne remplace pas un plateau constant, elle tient le pool en polynomial pendant que le plat part en super-linéaire.

## 6. Synthèse croisée avec la partie I

| | Anneaux (partie I) | Protocoles (partie II) |
|---|---|---|
| plat | Θ(n), pentes 2 à 5 | **super-linéaire** (explose avec \|S\|) |
| pool fenêtre | O(1) plateau (56 à 183) | polynomial (~n²), pas de plateau |
| croisement | n ≈ 29 à 46 | **n ≈ 3** |
| alphabets | constants en n | croissent avec n (paramètre structurel) |

Le tableau des « très bons scores » se raffine donc : **quotient O(1) pour les familles topologiquement scalables** (anneaux, grilles : alphabet constant) ; **quotient polynomial mais écrasant le plat pour les protocoles à mémoire partagée** — et c'est précisément dans ces familles que le moteur plat-mêne-avec-partage meurt, donc que l'écart se creuse le plus vite. Peterson & co sont moins réguliers, mais c'est là que la surcharge a le plus de marge.

## 7. Limites et suites

- pool = comptage de classes (fenêtre bornée W = 2, mentions de victime plafonnées à 3 niveaux), pas un compte de `created` d'un moteur déployable ; l'exactitude reste à confirmer dans `lisage-bench` (M7), où petN est le candidat prioritaire.
- Filter lock modélisé avec écriture de victime à l'arrivée seulement ; les variantes : suite logique = Eisenberg-McGuire, Szymanski, et le Peterson en tournoi (arbre : le pool devrait passer en O(log n), un entre-deux intéressant).
- petD confirme que la détection est le garde-fou : injecter dans le diffus coûterait des décorations non rentables ; garder le plan-vide de petN et le verdict « flat » de petD comme tests de non-régression du classifieur, aux côtés de la zéro-injection de la partie I.