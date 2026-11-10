# Deuxième exemple fil rouge : le protocole à fenêtre glissante

*Le protocole le plus vérifié du génie logiciel, et sa mécanique interne est la mécanique Lisage : la fenêtre glisse (le shift), la base de séquence est l'argument sur l'arc, l'ISN est éliminé par la section canonique, le passé réglé et le futur non envoyé sont deux courses absorbées. Les philosophes étaient la vitrine du pool ; celui-ci est la vitrine du contrôleur. Vérifié par simulation le 10 novembre 2026 : BFS exhaustif contre le langage, verdicts exhaustifs sur tous les mots, construction ascendante depuis l'horizon, pli de comptage par trois voies, fermeture par les événements, trace de l'ancre mobile.*

---

## 0. Le résultat en une table

| Représentation du SWP (côté émetteur, SACK) | Nœuds | Ce qu'il faut décider |
|---|---|---|
| Énumération explicite | infinie : la base n'est pas bornée ; 2^{k+1} − 1 états par base, toute base | rien à décider : impossible |
| MDD plat, base bornée à B | constant aussi (le prédicat y est trivial) | la borne B, l'ISN, la fenêtre k : chaque triplet est un modèle compilé de plus, et la glisse déplace la base au fond du domaine |
| **Lisage plié + ancre composée** | **4 nœuds (EN_VOL, LIBRE, ⊤, ⊥), pour toute base, toute fenêtre k, tout ISN** | **rien : aucun domaine borné, aucune re-compilation** |

Le slogan du deuxième exemple : **la sûreté est un typage, l'infini est le défaut, la fenêtre glisse parce que l'ancre glisse.**

---

## 1. Le modèle au niveau 1 (le langage)

```
type Cell = Tag { réglé, pend, acquitté, libre }   // l'état d'un numéro de séquence
          // pend : émis, non acquitté (en vol) ; acquitté : émis, le récepteur l'a
          // réglé : délivré et sorti de la fenêtre ; libre : jamais émis

envoi   : la plus petite seq libre < horizon → pend     // la frontière d'émission avance
acquit  : ∀i dans la fenêtre, seq i = pend → acquitté   // SACK : n'importe quelle position
glisse  : seq base = acquitté → base +1                 // la base avance d'un pas

let init = (ISN, tout libre)                            // un arc (D-abs), pas un arbre
```

L'espace des séquences est non borné dans les deux directions utiles : la base croît sans limite, le futur non émis est infini. La fenêtre de l'émetteur est k ; les acquittements sont sélectifs (n'importe quel message en vol peut être acquitté).

---

## 2. Le contrôleur C_swp

```
C_swp = ( δ           : Tag{réglé, pend, acquitté, libre}
        , Σ            : l'espace des séquences ; la lecture n'en matérialise qu'un segment
        , décomp + O   : positionnel : cellule i avant cellule i+1, depuis la base
        , R            : R-collapse (l'absolu s'effondre en ⊤, vérifié), R-share, D-abs (les deux courses)
        , défauts      : deux, directionnels : derrière la base, réglé ; devant la frontière
                        d'émission, libre. Le défaut n'est pas une clé : c'est une direction.
        , D^déf        : libre, absorbable sur tout suffixe
        , encodage     : relatif partout : numéros relatifs à la base ; l'ISN est éliminé
                        par la section canonique (min = 0)
        , attribution  : hérité = (au-delà, shift), au-delà = «la cellule précédente est devant
                        la frontière d'émission» ; synthétisé = (ℕ,+,×) ;
                        mémoire = cache (événement, forme, ancre composée)
        , ancre        : composée {base : b, horizon : k}
        )
```

---

## 3. Le langage atteignable

> **SWP = ℕ × {réglé^b (pend|acquitté)^j libre^∞ : j ≤ k}**, et l'atteignable est exactement le bien-typé.

Vérifié par BFS exhaustif (k = 1..5, base ≤ 4) : l'ensemble atteignable est égal au langage bien formé, avec exactement 2^{k+1} − 1 configurations de fenêtre par base (3, 7, 15, 31, 63 : vérifiés). L'invariance par translation est totale : le reachable est un produit, aucune position de séquence n'est spéciale, la base est un rayon libre. C'est le théorème caché du mot « sliding » : le même état relatif à toute base.

---

## 4. La représentation : 2 formes, 2 terminaux, 2 courses absorbées, 1 ancre composée

```
tête de fenêtre : hash-égale EN_VOL (C1 fusionne l'entrée au premier nœud :
                  pas de couture, les clés pend, acquitté, libre y sont toutes placées)
  EN_VOL : pend → EN_VOL   acquitté → EN_VOL   libre → LIBRE    // dans le préfixe émis
  LIBRE  : libre → LIBRE                                     // la boucle ; pend, acquitté : non placées
  à l'horizon (shift = k) : EN_VOL → ⊤, LIBRE → ⊤             // les deux fins légales
```

Lecture des mécanismes :

- **Handle** : (C_swp, forme, {base : b, horizon : k}). L'ancre est composée : deux décalages nommés, un par fragment (la base, l'horizon).
- **La course réglé : l'instance canonique de M4-2**. L'arc d'entrée porte l'argument b, la longueur de la course : le premier nœud de fenêtre est à la position b, le minimum des clés non réglées, la section canonique min = 0. La normalisation que la spec § 2.4.5 énonçait comme instance canonique a ici son modèle vivant.
- **La course libre : le futur est le défaut**. La boucle LIBRE est absorbée : une course de longueur quelconque, y compris infinie, se lit comme le seul arc. La séquence infinie n'existe jamais matériellement ; la question ouverte 10 (le coinductif) n'est pas ouverte pour autant : ici l'infini est le défaut, pas un objet.
- **L'ISN et Wireshark**. TCP tire son ISN au hasard (0 à 2³²−1 pendant le SYN) ; Wireshark convertit par défaut tous les numéros en numéros relatifs qui commencent à 0 (wiki officiel). L'outil le plus répandu de la planète fait la section canonique min = 0 par défaut, pour lisibilité. Lisage en donne la théorie et l'exploite : le pool est indépendant de l'ISN, l'ISN vit sur l'arc d'entrée.
- **Minimalité** : 2 formes, ni plus, ni moins : l'automate minimal de (pend|acquitté)*libre*. EN_VOL ≠ LIBRE : le suffixe [pend] (longueur 1) les distingue. La tête fusionne avec EN_VOL : le réglé derrière la base et l'en-vol dans la fenêtre sont le même hérité, il n'y a pas de promesse de couture (le contre-pied des philosophes, où S était irréductible).
- **Le pool** : 4 nœuds (2 formes + 2 terminaux), pour toute base, toute fenêtre, tout ISN. Et le DD **absolu** s'effondre entièrement en ⊤ (vérifié) : le prédicat d'atteignabilité est trivial sur le domaine typé. Tout le contenu du protocole vit dans le contrôleur : typage, ancres, arguments. Les philosophes montraient un pool riche et un contrôleur simple ; ici l'inverse exact.

---

## 5. Les événements comme transductions

| Transduction | Largeur | Mécanique Lisage |
|---|---|---|
| envoi | 1 cellule | la frontière d'émission avance : une seule classe, pas k : la garde se lit sur l'hérité (au-delà = 0), la position n'existe pas |
| acquit(i) | 1 cellule | k classes : la descente suit le shift i ; la colonne vertébrale reste EN_VOL (pend et acquitté sont la même forme) : l'image du chemin est le même handle |
| glisse | 1 cellule + l'ancre | **le déplacement d'ancre** : {base : b} → {base : b+1} ; le pool ne bouge pas |

Trace vérifiée (k = 4) : l'ancre bouge, le pool jamais.

```
init       base = 0, fenêtre LLLL   ancre {base: 0, horizon: 4}
envoi      base = 0, fenêtre PLLL   {base: 0, horizon: 4}
envoi      base = 0, fenêtre PPLL   {base: 0, horizon: 4}
acquit(0)  base = 0, fenêtre APLL   {base: 0, horizon: 4}
glisse     base = 1, fenêtre PLLL   {base: 1, horizon: 4}   // l'argument s'incrémente
acquit(0)  base = 1, fenêtre ALLL   {base: 1, horizon: 4}
glisse     base = 2, fenêtre LLLL   {base: 2, horizon: 4}   // le pool n'a pas bougé une seule fois
```

L'événement le plus banal du protocole (la base avance) est, dans le vocabulaire Lisage, un incrément d'argument. Le nom même du protocole est le mécanisme : la fenêtre glisse parce que l'ancre glisse.

---

## 6. La saturation : zéro rondes

L'ensemble bien typé est déjà clos par les trois événements : vérifié exhaustivement (k = 1..5, base ≤ 3 : 764 tirs à k = 5, zéro sortie du langage, zéro blocage). Le point fixe est atteint sans exploration : **la preuve d'atteignabilité est un typage, pas une saturation**. Les philosophes découvraient leurs formes en deux à trois rondes ; ici le contrôleur savait tout, la découverte ne paie que ce que le typage ne garantit pas. La deadlock est vide (vérifié : envoi armé si j < k, acquit armé sur tout pend, glisse armée sur toute base acquittée).

---

## 7. Les opérations dérivées

- **count** (synthétisé, (ℕ,+,×)) : 2^{k+1} − 1 par base (vérifié par trois voies jusqu'à k = 24 : 2²⁵ − 1 = 33 554 431 ; par BFS jusqu'à k = 5). Le pli : n_E(ℓ) = 2n_E(ℓ−1) + n_L(ℓ−1), n_L(ℓ) = n_L(ℓ−1), la matrice [[2,1],[0,1]] : la plus simple du bestiaire, valeurs propres 2 et 1. La croissance 2 par cellule, la plus simple des croissances, portée par les plus simples des formes.
- **member** (hérité) : O(k) sur la fenêtre explicite, O(1) sur les deux courses absorbées : la longueur de la course est comparée à l'argument, pas parcourue.
- **exists** : «la fenêtre est-elle pleine ?» : l'hérité au dernier niveau de fenêtre, une lecture, aucun dépliage.
- **deadlock** : vide (ci-dessus).

---

## 8. Ce que l'exemple prouve, et ce qu'il ne prouve pas

**Vérifié (simulation, 10 novembre 2026)** : le langage (BFS == bien-typé == verdict DD, exhaustif sur les 3^k mots de fenêtre pour k ≤ 8) ; les 4 nœuds (construction ascendante depuis l'horizon : les 2 formes au premier niveau, zéro création à chaque niveau suivant, la tête fusionne avec EN_VOL, le même pool pour k = 1..50) ; le pli (trois voies) ; la fermeture (zéro sortie, zéro blocage) ; la trace de l'ancre mobile.

**Ce qu'il montre que les philosophes ne montraient pas** : (1) l'**ancre composée** {base, horizon} ; (2) l'**argument vivant** : la base bouge (glisse), l'ISN varie (section canonique) : l'instance canonique de M4-2 en conditions réelles, avec un précédent industriel (Wireshark) ; (3) **deux courses absorbées**, dont une infinie, sans ouvrir le coinductif ; (4) le **typage relatif à l'argument** (pend, acquitté non placées devant la frontière d'émission : la clause iv du contrat d'algèbre de la spec § 2.4.5) ; (5) un **contrôleur qui porte tout** : le pool est le plus petit possible, la sûreté se lit sur les clés déclarées.

**Non prouvé, prévu** : le côté récepteur et les canaux (perte, duplication, réordonnancement : le SWP complet). Là le contenu redevient non trivial et le pool redevient nécessaire : c'est le terrain de la preuve dynamique, la suite logique de cet exemple. La comparaison mesurée contre un encodeur existant reste à faire ; le SWP a été vérifié par PVS, par μCRL, par IOA et MONA (liens Springer rapatriés), et la lignée regular model checking (survey Abdulla, Jonsson et al., STTT) traite les familles infinies par ensembles rationnels : Lisage est le côté DD de la même intuition, à rapatrier dans l'état de l'art pour le positionnement.

**Rôles dans le portefeuille** : les philosophes = la vitrine du pool (partage, ancre symbolique, couture, comptage) ; la fenêtre = la vitrine du contrôleur (ancres composées, arguments, typage, courses). Le premier dit « le pool paie 1 au lieu de 4n » ; le second dit « le contrôleur sait déjà tout, le pool ne découvre rien ». Sprint Rennes : un exemple hors famille Petri, un objet d'ingénierie réseau reconnu, et les noms du protocole (seq, ack, fenêtre, ISN) sont le vocabulaire de l'industrie : la lecture sémantique de petrichor y lit l'hypothèse de symétrie comme un théorème (l'invariance par translation, § 3), la piste « navette » du backlog a son cas d'école.