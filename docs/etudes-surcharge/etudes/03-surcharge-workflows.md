---
name: "etude-surcharge-workflows"
title: "Étude « surcharge » III : workflows à jetons denses (Kanban, FMS & co)"
type: "text/markdown"
---

# Étude « surcharge d'homogénéité » — partie III : Kanban, FMS, HouseConstruction, SwimmingPool, ViralEpidemic

Troisième morphologie de scaling : ici, le paramètre fait croître la **largeur de marquage** (les valeurs des compteurs), pas la topologie. Kanban et compagnie sont des files de production à capacité bornée, pas des réseaux de modules. Le protocole de mesure s'adapte : on mesure les **arcs** en plus des nœuds, car c'est là que vit le coût.

## 1. Familles (proxys fidèles à l'esprit des modèles MCC)

| Famille | Proxy | Paramètre | Variables |
|---|---|---|---|
| kanban | 3 étages en ligne, (attente, fait) par étage, capacité kanban K | K | 6 compteurs |
| fms | 3 étages, 2 types de pièces A/B partagent une machine par étage | K | 9 variables |
| house | chaîne de 4 étages de chantier, H maisons-jetons, conservation | H | 5 compteurs |
| pool | (file, bassin ≤ 3, sortis), H nageurs | H | 3 compteurs |
| viral | (S, I, R), S+I+R = N, init (N−1, 1, 0) | N | 3 compteurs |

Toutes les invariants vérifiés à chaque point : capacité kanban, capacité d'étage FMS, conservation (house, pool, viral), bassin ≤ 3.

## 2. Résultats

**Kanban** — |S| = T(K+2,2)³ (27, 216, 1000, 3375, 9261 : cube du triangular), nœuds = 3K+6 :

| K | \|S\| | nœuds | arcs |
|---|---|---|---|
| 1 | 27 | 9 | 15 |
| 3 | 1 000 | 15 | 42 |
| 5 | 9 261 | 21 | 81 |

**FMS** — |S| = (T(K+1)+K(K+1))³ (125, 1728, 10648), nœuds ≈ 12K²+3K+30 :

| K | \|S\| | nœuds | arcs |
|---|---|---|---|
| 1 | 125 | 45 | 78 |
| 2 | 1 728 | 84 | 181 |
| 3 | 10 648 | 147 | 392 |

**HouseConstruction** — |S| = C(H+4,4) (5 … 495), nœuds = 4H+5 :

| H | \|S\| | nœuds | arcs |
|---|---|---|---|
| 4 | 70 | 21 | 55 |
| 8 | 495 | 37 | 153 |

**SwimmingPool** — |S| = 2H²+2, nœuds = 2H+3 :

| H | \|S\| | nœuds | arcs |
|---|---|---|---|
| 4 | 34 | 11 | 34 |
| 8 | 130 | 19 | 84 |

**ViralEpidemic** — |S| = N(N+3)/2, nœuds = 2N+2 :

| N | \|S\| | nœuds | arcs |
|---|---|---|---|
| 10 | 65 | 22 | 86 |
| 40 | 860 | 82 | 941 |

## 3. Lecture : l'enjeu n'est ni les nœuds, ni le pli

1. **Le plat tient très bien.** Nœuds linéaires ou quasi en paramètre (3K+6, 2H+3, 2N+2…), quelques dizaines de nœuds pour |S| qui explose en polynôme de degré 2 à 6 : le rapport |S|/nœuds atteint déjà 441 sur le kanban du jouet. Contrairement à petN (partie II), rien ne part en super-linéaire : la structure à étages garde des contraintes couplées simples, portables dans la table du plat.
2. **Le fold n'a rien à plier.** Aucune classe de modules répétables : chaque variable a un rôle distinct. Le classifieur rend « plan vide », et il a raison : le quotient à fenêtre serait identique au plat. La zéro-injection s'applique, par inutilité et non par nécessité.
3. **Le vrai coût, ce sont les arcs.** Les arcs croissent en Θ(param²) (15 → 81 pour kanban K=5 ; 941 pour viral N=40) : c'est la largeur de marquage qui paie, arc par arc, valeur par valeur (un arc par valeur de compteur). À l'échelle MCC (bornes k en dizaines, centaines), Θ(K²) arcs devient la ligne de coût dominante.

## 4. La surcharge qui s'applique ici : valuée, pas topologique

Pour ces familles, l'annotation utile n'est pas une couture de module mais une **annotation valuée des arcs** : les K+1 arcs frères d'un nœud (une par valeur du compteur) se replient en un seul arc annoté d'une transduction de plage. Mesure : arcs annotés = le nombre de nœuds, donc exactement la ligne « arcs annotés » des tableaux (9 vs 15 à K=1 ; 82 vs 941 à N=40 : **×11 d'arcs en moins**, et le rapport croît linéairement avec le paramètre : ~param/2).

C'est la lignage des arcs valués des EV-MDD de Ciardo ([Ciardo & Siminiceanu](https://link.springer.com/chapter/10.1007/3-540-36126-X_16)) — et c'est nativement le terrain de lisage : les transductions attribuées des arcs sont déjà le format du moteur. La partie III dit donc : sur ces familles, la surcharge à prévoir dans lisage n'est pas topologique mais**dense**, et elle est déjà dans le langage.

## 5. Synthèse des trois parties

| | I. Anneaux (Philo, Jeton) | II. Protocoles (petN) | III. Workflows denses (Kanban et co) |
|---|---|---|---|
| Ce qui croît | la topologie | les gardes universelles + n−1 niveaux | la largeur de marquage |
| plat | Θ(n) | super-linéaire (explose) | Θ(param) : tient ! |
| fenêtre / fold | **O(1)** : plateau | polynomial : écrase le plat | inutile (rien à plier) |
| croisement | n ≈ 30 | n ≈ 3 | aucune : le plat suffit |
| surcharge utile | couture topologique | niveau des victimes / jeton | **arcs valués** (×param/2 arcs en moins) |
| verdict classifieur | plan ancre sémantique | 1 classe modulo rotation | « rien à faire » |

Conséquence pour le classement MCC : Kanban, FMS, HouseConstruction, SwimmingPool et ViralEpidemic forment une **troisième catégorie**, distincte des anneaux (O(1) gagnant) et des protocoles (polynomial écrasant) : des modèles où tout moteur DD marque des points faciles à l'examen StateSpace (|S| en milliards, dont l'explicite ne voit rien), et dont le différenciateur n'est pas le nombre de nœuds mais la façon de payer les arcs : le bord de lisage y est ses transductions, pas ses coutures.

## 6. Limites

- Proxys réduits : le Kanban et le FMS réels du MCC ont plus de places (le FMS de Ciardo en a ~22) ; les degrés exacts changent, pas les lois (nœuds ≤ Θ(param), arcs Θ(param²), |S| polynôme de haut degré).
- Bordée réelle des instances MCC : K jusqu'à plusieurs dizaines et structures légèrement couplées entre étages ; les formes fermées ci-dessus restent à confirmer sur les PNML réels.
- Le nœud quadratique de FMS (12K²+3K+30) montre que le couplage par machine dégrade le plat : la limite entre « workflows faciles » et « protocoles durs » est floue et se joue au cas par cas sur les gardes partagées.