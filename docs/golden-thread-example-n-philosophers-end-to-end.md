# Exemple fil rouge : n philosophes, de bout en bout

*Déroulé complet du cadre sur le cas régulier : modèle, contrôleur, encodage, transductions, saturation, opérations, puis la variante irrégulière du philosophe gaucher (§ 9). Chiffres vérifiés par simulation : appartenance exhaustive pour n ≤ 7, comptage contre le polynôme d'indépendance du cycle jusqu'à n = 20, mesures du prototype plat jusqu'à n = 200 (§ 6.2, § 9.5). Résultats affichés : l'espace atteignable, exponentiel en n, tient en 7 nœuds indépendants de n ; l'explosion intermédiaire est dans la policy naïve, pas dans la structure ; la variante à un philosophe irrégulier tient aussi en 7 nœuds.*

---

## 0. Le résultat en une table

| Représentation de R_n | Nœuds | n = 50 |
|---|---|---|
| Énumération explicite | ≈ (1+√3)^n ≈ 2,73^n états | ≈ 7·10²¹ états : impossible |
| MDD plat, ordre fixe, réductions § 2.3 seules | Θ(n) | ~200 |
| SDD hiérarchique (Hamez) | compact : la hiérarchie capture la régularité (c'est son cas d'école) | à mesurer (jalon 4) |
| **Lisage plat + formes relatives + D-abs** | **7 nœuds, pour tout n ≥ 3** | **7** |

Les 7 nœuds : 5 formes + 2 terminaux. Le paramètre n ne vit dans aucun nœud : il vit sur le shift de l'arc d'entrée (le dépliage). R_5 et R_50 partagent exactement les mêmes nœuds physiques ; changer n, c'est changer l'ancre, pas le graphe. Les 5 formes sont l'automate minimal du langage des cellules légales : C1 (minimalité par handle) exige ces nœuds, ni un de moins, ni un de plus. Vérifications : |R_3| = 20, |R_4| = 56, |R_5| = 152, |R_6| = 416, |R_8| = 3104, |R_20| = 536 754 176. La variante irrégulière (§ 9) tient aussi en 7 nœuds : seul le nœud racine change (7 clés au lieu de 5), l'intérieur est inchangé.

---

## 1. Le modèle au niveau 1 (le langage)

```
type Philo = Tag { think, hungry, eat }
type Fork  = Tag { free, me, prev }        // me : tenue par le philosophe de la cellule ; prev : par le précédent
type Cell  = { pc : Philo, left : Fork }    // left : la fourchette entre i-1 et i
type State = Seq Cell                       // anneau de longueur n, n ≥ 3

take    : ∀i, pc_i = hungry ∧ left_i = free ∧ left_{i+1} = free
          → pc_i = eat, left_i = me, left_{i+1} = prev     // saisie atomique des deux fourchettes
release : ∀i, pc_i = eat → pc_i = think, left_i = free, left_{i+1} = free
hungry  : ∀i, pc_i = think → pc_i = hungry

let init     = map (i -> { pc : think, left : free }) (ring n)
let reach    = fix (take ∪ release ∪ hungry) init    // saturation, policy auto-détectée
let n_states = count reach                            // semi-anneau (ℕ,+,×) : 152 si n = 5
let stuck    = filter (s -> ¬ enabled s) reach        // vide : think → hungry est toujours armé
```

Saisie atomique : pas d'interblocage dans ce modèle. La variante «un philosophe gaucher» (saisie en deux temps) est le terrain falsifiable : livrée au § 9.

---

## 2. Le contrôleur C_philo (instanciation de § 1.2 de la proposition)

```
C_philo = ( δ           : borné, Tag{think,hungry,eat} × Tag{free,me,prev}   // 9 clés, 5 placements légaux
          , Σ            : Seq(Cell), longueur n ≥ 3
          , décomp + O   : positionnel : cellule i avant cellule i+1 ; l'anneau est déroulé,
                          la couture 0/n est explicite (racine = position 0, ancre = position n)
          , R            : R-collapse, R-default, R-share (spec § 2.3) ; D-abs, D-trans armées (spec § 2.4)
          , défauts      : déduits du type : la cellule (think, free)
          , D^déf        : (think, free), absorbable sur tout sous-domaine
          , encodage     : relatif dans l'intérieur (shift sur l'arc) ; absolu aux coutures
          , attribution   : hérité = (p, shift), p = «le précédent mange» ;
                            synthétisé = semi-anneau (Bool pour la saturation, (ℕ,+,×) pour le comptage) ;
                            mémoire = cache (événement, forme, ancre)
          )
```

Le choix qui porte tout le reste : l'**intercalage** (pc_i, left_i). Chaque fourchette vit dans la cellule de son consommateur de droite, donc les trois événements sont locaux (largeur ≤ 2, § 5). Variante d'encodage «Seq Philo puis Seq Fork» : take_i devient non local et réclame la mémoire de niveau (T3) ; le cadre l'exprime aussi, mais l'oracle de rentabilité préfère l'intercalage. **La localité des événements est une propriété de l'instance, pas du cadre.**

---

## 3. L'espace atteignable comme langage

La cohérence fourchettes/philosophes est locale à la cellule (typage par inclusion : Val(x) dépend de l'hérité p) :

| Hérité p («le précédent mange») | Cellules légales (pc, left) |
|---|---|
| p = 0 | (think, free), (hungry, free), (eat, me) |
| p = 1 | (think, prev), (hungry, prev) ; (eat, ·) non placée |

Un état = une suite de n cellules légales sans deux «eat» adjacents circulairement, et :

> **R_n = mots circulaires de n cellules sans «eat eat» adjacent = 2^n · I(C_n, ½)**, avec I le polynôme d'indépendance du cycle.

Preuve en deux temps (vérifiée par BFS explicite jusqu'à n = 8) : tout état atteignable respecte l'invariant (les gardes l'imposent) ; toute configuration indépendante est atteinte (saisies dans n'importe quel ordre, puis passages hungry). La bijection pc-mots ↔ (E, H) : E = mangeurs (indépendant dans le cycle), H = affamés, le reste pense.

---

## 4. La représentation : 5 formes, 2 terminaux, 1 ancre

Le diagramme SVG de référence est dans le canevas « Les 7 formes des philosophes : diagramme SVG » ; l'ASCII qui suit reste la source de vérité des clés.

```
racine S (position 0, encodage absolu ; sa clé left encode déjà pc_{n-1})
  ├─ (t,f) ──▶ A₀        ┐
  ├─ (h,f) ──▶ A₀        │ promesse : pc_{n-1} ≠ eat
  ├─ (e,m) ──▶ A₁        ┘ (entrée p = 1 : je mange)
  ├─ (t,p) ──▶ B₀        ┐
  └─ (h,p) ──▶ B₀        ┘ promesse : pc_{n-1} = eat

intérieurs (cellules 1..n-1, shift sur l'arc, ancre = position n)
  A₀ : (t,f)→A₀  (h,f)→A₀  (e,m)→A₁        B₀ : (t,f)→B₀  (h,f)→B₀  (e,m)→B₁
  A₁ : (t,p)→A₀  (h,p)→A₀  (e,·) non placée   B₁ : (t,p)→B₀  (h,p)→B₀  (e,·) non placée
  à l'ancre : A₀ → ⊤, A₁ → ⊥               à l'ancre : B₀ → ⊥, B₁ → ⊤
```

Lecture des mécanismes :

- **Handle** : (C_philo, forme, ancre). L'ancre est symbolique («la fin de l'anneau») ; le shift vit sur l'arc, +1 par cellule ; R_n = dépliage de S sur n cellules. Les mêmes nœuds servent toutes les valeurs de n.
- **Formes relatives (spec § 2.4.3)** : A₀ et B₀ sont le motif auto-similaire «une cellule, puis la suite» ; leur boucle (t,f) est le segment régulier, déplié jusqu'à l'ancre. Déplier A₀ donne exactement ce que la hiérarchie SDD encode en un nœud : la conjecture d'unfold (spec § 2.4.4) en acte.
- **D-abs (spec § 2.4.1)** : (think, free) = D^déf ; une course de cellules par défaut de longueur quelconque se lit comme le seul arc (t,f). L'état initial (tout le monde pense) = un arc, pas un arbre.
- **Typage par inclusion** : l'arc (eat, ·) de A₁/B₁ n'existe pas : la clé n'est pas dans Val(x) quand le précédent mange.
- **Couture** : la fourchette de la cellule 0 encode déjà pc_{n-1}, donc S lit une **promesse** sur la fin de l'anneau, et l'ancre la vérifie. La ligne A (promesse «le dernier ne mange pas») est le cœur ; la ligne B (promesse «le dernier mange») et le terminal ⊥ sont le terme de bord : O(1). A₀ et B₀ ont le même motif d'enfants, elles diffèrent par le verdict d'ancre : distinctes sous C1, confondues par aucun quotient.
- **Minimalité** : S, A₀, A₁, B₀, B₁ = les 5 états de l'automate minimal du langage (départ + (p, promesse)) ; la canonicité est la minimisation de coalgèbre (§ 1.3 de la proposition), ce document en est la vérification concrète. Vérifié le 10 novembre 2026 : les 5 états sont deux à deux distingués par des suffixes de longueur ≤ 2 (le seul témoin de longueur 2 est S ≠ A₀, suffixe [(t,p),(e,m)] : seule S atteint la ligne B depuis l'entrée) ; aucun nœud fusionnable, ni un de plus, ni un de moins.

Trois lectures de contrôle (n = 5) : eat en 2 seul : S→A₀, (e,m)→A₁, (t,p)→A₀, boucles, ancre(A₀) → ⊤. Eat en 0 seul : S→A₁, (t,p)→A₀, boucles, ancre(A₀) → ⊤. Eat en 0 et 4 : S→A₁, (t,p)→A₀, boucles, (e,m)→A₁, ancre(A₁) → ⊥ (0 et 4 sont adjacents dans l'anneau : rejeté). Ces trois traces sont dans la simulation exhaustive.

---

## 5. Les événements comme transductions attribuées

| Transduction | Largeur | Canaux | Signature de portée |
|---|---|---|---|
| hungry | 1 cellule | hérité (p, shift) | lit (i) |
| take | 2 cellules | hérité (p, shift) ; garde remontante | lit (i, i+1) |
| release | 2 cellules | hérité (p, shift) | lit (i, i+1) |

Les 3n événements = **3 classes × n shifts** : chaque classe est invariante par translation, la transduction descend avec le shift dans le canal hérité, le computed cache porte la clé (événement, forme, ancre). Le travail est fait une fois par paire, pas une fois par position : c'est le partage de travail par invariance de translation (spec § 2.4). Les événements de couture (take/release aux positions 0 et n-1) sont les seuls à écrire dans la ligne B : le bord, encore O(1).

---

## 6. La saturation : O(1) cœur + couture

Départ : init = un arc (D-abs). Point fixe : les 7 nœuds, en trois à quatre rondes :

1. hungry partout : arcs (h,f) sur S, A₀, B₀ ;
2. take : arcs (e,m) sur A₀, B₀ et S ; découverte de A₁, B₁ ;
3. take/release près de la couture : clés (t,p), (h,p) de S (découverte de la ligne B) puis des lignes A/B ; convergence.

La policy «statique par niveaux» (Ciardo) balaye des positions ; ici les positions ont disparu : l'itération porte sur les usages (événement × forme × ancre), l'index niveau → usages vivant dans le plan d'ombre (la voie porteuse discutée pour la compatibilité Hamez). La policy «auto-détectée» (Hamez) reconnaît le motif régulier et fait exactement ce balayage par formes. Coût attendu : constant par classe d'événement + la couture. C'est la prédiction falsifiable de la contribution 8 : si la saturation coûte Θ(n) par classe, le partage de travail est faux.

### 6.1 Ce que fait un tir (l'opération élémentaire)

Un tir de l'événement e sur le nœud courant (prise d'image, puis union) :

1. **descente** : la transduction suit le canal hérité jusqu'à la position i ; dans le prototype plat ce sont i niveaux, en Lisage le shift est déjà sur l'arc et la forme est relue dans le pool ;
2. **garde** : lecture d'au plus 2 clés (take : (h,f) ∧ (·,f)) ; une clé manquante coupe la branche ;
3. **ré-écriture locale** : re-clé de 2 nœuds au plus ((h,f)→(e,m) et (·,f)→(·,p)) ;
4. **union** avec le résultat partiel, puis re-réduction : le hash-consing fusionne les nœuds égaux à l'écriture (R-share gratuit) ;
5. **cache** : clé (événement, nœud) dans le prototype, (événement, forme, ancre) en Lisage : le second tir sur la même paire est gratuit.

La couture : dans le plat, take_{n-1} relit la position 0 depuis la profondeur n-1, une descente de span n payée à chaque application ; en Lisage la couture est un arc du pool, O(1). C'est le seul écart de complexité par tir entre le plat et Lisage.

### 6.2 Mesuré : où ça explose (et où ça n'explose pas)

Prototype : MDD plat hash-consé en Python, ordre fixe, mêmes événements ; oracles : BFS explicite (sat == BFS pour n = 4..7) et pli (== comptage exact jusqu'à n = 20). Mesures d'un prototype interprété : les ratios et ordres de grandeur sont porteurs, les constantes non.

| n | 5 | 10 | 20 | 30 |
|---|---|---|---|---|
| résultat final (nœuds vivants, ⊤ compris) | 16 | 36 | 76 | 116 |
| saturation : image max en cours | 15 | 35 | 75 | 115 |
| BFS naïf : image max en cours | 26 | 126 | 553 | 1 276 |
| saturation : nœuds créés (table unique) | 140 | 655 | 2 885 | 6 715 |
| BFS naïf : nœuds créés | 432 | 4 634 | 44 959 | 162 884 |
| saturation : firings / passes | 45 / 2 | 90 / 2 | 180 / 2 | 270 / 2 |
| BFS naïf : firings / rounds | 120 / 8 | 480 / 16 | 1 860 / 31 | 4 140 / 46 |

Trois lectures :

- **L'explosion intermédiaire existe, et elle est dans la policy, pas dans la structure.** Le résultat final est linéaire (4n − 4), mais le BFS naïf gonfle son image en cours à ≈ 1,4 n² : jusqu'à 11 × le résultat final (1 276 contre 116 à n = 30). La saturation garde l'image ≤ résultat + 1 : elle n'écrit jamais beaucoup plus que ce qu'elle garde.
- **Le coût de création suit** : ≈ 7,8 n² en saturation (Θ(n²) : chaque paire (événement, nœud) écrite au plus une fois), ≈ 6 n³ en naïf. En pool Lisage, ces paires se quotientent en (événement, forme, ancre) : la cible est O(1) par classe + couture (contribution 8).
- **Le quotient est déjà visible dans le plat** : les 15 nœuds internes vivants à n = 5 ne portent que 5 signatures distinctes, avec multiplicités ×6, ×5, ×2, ×1, ×1 : le plat stocke des réplicas positionnels des 5 formes du § 4 (⊥ y est implicite : clé absente). La table du § 0 est donc à moitié mesurée : le plat paie n copies, Lisage paie une forme et facture le dépliage à la lecture.

(À n = 100 : 396 vivants, 78 325 créés, 900 firings, 2 passes ; à n = 200 : 796 vivants, 316 625 créés.)

### 6.3 Le même exercice «à la DDD» et «à la Hamez» : ce que la policy change, ce qu'elle ne change pas

Le prototype **est** un moteur d'homomorphismes à la DDD (Couvreur) : application locale d'un événement, mémo (événement, nœud), union mémoïsée (nœud, nœud), merge ⊕ idempotent hash-consé. Les chiffres du § 6.2 se lisent donc tels quels comme des mesures DDD. Mesures complémentaires (moteur reconstruit, cohérent à ±2 % avec le § 6.2) : le **pic de structure vivante** (ce qu'un moteur à GC et caches évictibles doit réellement retenir en vol : racine + image courante) et la **mémoire retenue sans éviction** (table unique + les deux mémos, jamais purgés) :

| n | 10 | 20 | 30 |
|---|---|---|---|
| **Saturation** : pic vivant | 68 (≈2,0 × final) | 148 (≈1,95) | 228 (≈1,97) |
| **Saturation** : retenue sans éviction | 2 033 (≈20 n²) | 9 003 (≈23 n²) | 20 973 (≈23 n²) |
| **Fixpoint naïf** : pic vivant | 299 (8,3 × final) | 1 299 (17,1 ×) | 2 971 (25,6 ×) |
| **Fixpoint naïf** : retenue sans éviction | 14 942 (≈15 n³) | 142 692 (≈18 n³) | 513 092 (≈19 n³) |

Trois lectures :

- **Le pic vivant est gouverné par la policy, pas par la représentation.** En saturation, le moteur ne retient jamais beaucoup plus que le résultat (≈2× : le résultat, plus l'image en vol qui partage presque tout) ; en fixpoint rond par rond, le pic est quadratique (≈3,3 n², ratio au final ≈ 0,85 n, croissant) : c'est le *peak effect* de la littérature, mesuré. La bibliothèque SDD de Hamez dit exactement cela et implémente la saturation **dans** la bibliothèque (fixpoint h*, policy auto-détectée) ; la hiérarchie en classes bornées ne fait jamais exister de front d'onde global : l'intermédiaire est la classe courante.
- **Mais la mémoire et le travail restent indexés par position.** Même en saturation, un moteur plat retient Θ(n²) : Θ(n) événements × Θ(n) nœuds (mémo (événement, nœud) ≈ 8 n², union ≈ 7,8 n², table ≈ 7,4 n²), et sans GC rien ne rend ces entrées. En naïf c'est Θ(n³) retenu. La hiérarchie de Hamez réduit les paires à (événement, classe) : Θ(n) ; le folding récursif des modèles réguliers (Instantiable Petri Nets) partage les classes identiques et pousse les philosophes à l'échelle de plusieurs milliers. Aucun des deux ne quotientte les paires elles-mêmes.
- **La cible Lisage est le quotient des paires** : (classe d'événement, forme, ancre) = 3 × 5 × 1 entrées, O(1) + bord, et un pool borné à 7 nœuds pendant toute la construction (la séquence de découverte du § 6). Le tableau du § 0 dit que le résultat y est O(1) ; cette table ajoute que l'intermédiaire et la mémoire de travail y sont O(1) aussi, là où le meilleur moteur plat-hiérarchique est Θ(n) vivant / Θ(n) paires. C'est le protocole du jalon 4 : mesurer SDD/HSDD (nœuds, pic, mémoires) sur le même exemple, la ligne de base attendue étant pic ≈ final et travail quasi-linéaire.

Le gaucher ne change aucune des courbes : pic vivant 70/150/230 (sat, +2 nœuds), 316/1 333/3 029 (naïf, +1 %), mémos +1,2 %. L'irrégularité coûte une constante dans toutes les policies plates ; côté HSDD, elle casse le folding des modules identiques et réclame un re-parenthésage autour de la couture ; côté Lisage, elle remplace S par S' (§ 9) et rien d'autre.

Sources : le point peak effect / saturation interne est dans Hamez, Thierry-Mieg, Kordon, *Building Efficient Model Checkers using Hierarchical Set Decision Diagrams and Automatic Saturation*, Fundamenta Informaticae 94(3-4), 2009, et sa version ICATPN 2008 (benchmark philosophes, folding récursif, table 2 : 2n philosophes) ; le folding régulier est dans Thierry-Mieg, Poitrenaud, Hamez, Kordon, *Hierarchical Set Decision Diagrams and Regular Models*, TACAS 2009. Les nombres exacts de ces tables sont à rapatrier pour le benchmark du jalon 4 (accès HAL limité lors de la rédaction) ; les qualités affirmées ici sont celles des abstracts et du texte extrait.

---

## 7. Les opérations dérivées (une structure, des semi-anneaux)

- **count** (synthétisé, (ℕ,+,×)) : le pli porte sur les formes, pas sur les positions. Sur la ligne A : a₀(ℓ) = 2a₀(ℓ-1) + a₁(ℓ-1), a₁(ℓ) = 2a₀(ℓ-1) (matrice [[2,1],[2,0]], valeurs propres 1 ± √3) ; la ligne B est la même matrice avec conditions initiales échangées (ancre ⊥/⊤) ; |R_n| = 2a₀(n-1) + a₁(n-1) + 2b₀(n-1), vérifié exact jusqu'à n = 20. Croissance (1+√3)^n : le comptage est une puissance de matrice, O(log n), pas O(n). Ce pli est l'explicitation de D-abs (dualité réduction↔explicitation, (A6) spec § 5.2) : la course régulière de (t,f) absorbée en un seul arc est re-dépliée en récurrence linéaire sur l'hérité, et la régularité de l'absorbé achète l'exponentiation ; contre-épreuve : le gaucher (§ 9.4), même matrice enrichie, ratio 1,2887 constant.
- **exists** (synthétisé, Bool) : «la cellule 0 peut-elle manger ?» : la projection ne garde que la branche (e,m) de S : le résultat est un objet du même pool, à peine plus petit.
- **member** (hérité) : un état explicite contre R_n : on descend avec le shift, O(n) sur le dépliage, stockage nul.
- **deadlock** : vide (hungry toujours armé depuis think) : filter + exists, une ligne de langage.

---

## 8. Ce que l'exemple prouve, et ce qu'il ne prouve pas

**Prouvé ici** : la structure de R_n (triple caractérisation vérifiée), l'existence d'un encodage à 7 nœuds indépendant de n, le comptage exact par pli sur les formes. Ces faits ne dépendent pas du cadre : ils sont la cible que le cadre doit atteindre.

**La constructibilité ascendante est vérifiée aussi (simulation, 10 novembre 2026)** : niveau après niveau en partant du terminal, avec l'ancre en table de verdicts (4 entrées) et l'argument sur l'arc, la construction crée les 4 formes au premier niveau, puis **zéro nœud à chaque niveau suivant**, puis S au niveau n : convergence vers exactement les 7 nœuds, S identique pour n = 3..50 (R_5 et R_50 partagent tout). Le même montage en absolu (verdicts d'ancre matérialisés en nœuds, ⊥ implicite) crée 3 puis 4 nœuds par niveau pour toujours : **4n − 4 vivants, exactement la ligne du prototype plat (§ 6.2)** ; la convergence est donc une propriété de la convention d'ancre (l'ancre symbolique), pas du terrain, et le bord du régime absolu est la même couture que celle du plat (détail élégant : au premier niveau, seul B₁ manque à l'appel, son nœud de bord est tout-⊥ et se réduit à ⊥ : promesse «le dernier mange» déjà morte à une cellule de la fin). Contrôles : table dérivée du modèle (§ 1, § 3) == table du § 4 ; verdicts exhaustifs sur les 5^n mots de clés pour n ≤ 6 ; les états légaux acceptés pour n ≤ 8 ; |R_n| par trois voies indépendantes (DP sur les pc-mots, chemins du DD, pli du § 7) égaux jusqu'à n = 20 (536 754 176) et n = 50 ; les 5 états deux à deux distingués par des suffixes de longueur ≤ 2 (C1).

**Non prouvé, prévu** :

1. **La régularité est le meilleur cas.** L'anneau est le terrain choisi par la lignée SDD précisément parce qu'il est régulier ; HSDD y partage déjà stockage et travail. La conjecture d'unfold pose que le plat+shift de Lisage est le dépliage exact de leur hiérarchie : la comparaison mesurée (nœuds, temps, cache) est le benchmark du jalon 4.
2. **La valeur ajoutée visible ici** : pas de hiérarchie à construire ni à maintenir ; le même pool héberge le contrôleur témoin (BDD classique pour le différentiel CUDD) ; l'irrégularité est absorbée par le bord : le philosophe gaucher (§ 9) n'ajoute **aucune** forme intérieure, seule la racine s'enrichit (7 clés au lieu de 5) et le comptage se refait par le même pli ; la hiérarchie SDD, elle, est à reconstruire.
3. **La dégradation est prévue** : topologie quelconque, les formes perdent leur auto-similarité, la représentation redevient Θ(n) puis le MDD plat reprend l'avantage ; c'est le critère de l'oracle (§ 1.2 de la proposition) : répétition régulière → plat+shift ; imbriquée irrégulière → tour de contrôleurs.
4. **Cible de design, partiellement mesurée** : le prototype plat valide la structure du langage (sat == BFS, pli == comptage), les intermédiaires (§ 6.2) et le quotient en 5 signatures ; mais les 7 nœuds en pool et le O(1)+bord restent des prédictions : le property-based du jalon 1 doit vérifier que la saturation converge vers exactement ces formes (C1, minimalité par handle), et le jalon 4 doit les mesurer contre l'encodage SDD hiérarchique. Si la mesure contredit la table du § 0, la contribution 8 est falsifiée : c'est le protocole.

Note de méthode : la première version de cette structure (avant simulation) ratait la promesse portée par la fourchette de la cellule 0 et rejetait des états valides ; c'est exactement le genre d'erreur que le test différentiel du jalon 1 attrape au premier commit. L'exemple valait donc déjà son poste.

---

## 9. La variante irrégulière : le philosophe gaucher

Le cas régulier est le terrain favorable de la lignée SDD. Test de la promesse «l'irrégularité est absorbée par le bord» : le philosophe 0 saisit en deux temps, tous les autres restent atomiques.

### 9.1 Le modèle

```
take1_0 : pc_0 = hungry ∧ left_1 = free → pc_0 = g, left_1 = prev   // premier temps, largeur 2
take2_0 : pc_0 = g ∧ left_0 = free → pc_0 = eat, left_0 = me        // second temps, largeur 1
release_0 : pc_0 = eat → pc_0 = think, left_0 = free, left_1 = free // comme au § 1
philosophes 1..n-1 : modèle du § 1 inchangé
```

L'état g : le philosophe 0 tient la fourchette partagée avec la cellule 1 (encodée left_1), attend celle de sa propre cellule (left_0). L'interblocage reste impossible : un seul philosophe tient-et-attend, la chaîne d'attente ne peut pas se refermer (les autres saisissent atomiquement). Le type local de pc_0 passe à {think, hungry, g, eat} : un type de cellule changé à une position, pas un cadre changé.

### 9.2 Le langage atteignable

Mots circulaires de pc, sans «eat eat» adjacent, sans (pc_0 = g ∧ pc_1 = eat) : le voisin de droite du gaucher ne peut pas lui prendre sa fourchette tenue. Les fourchettes se dérivent des pc : left_1 = prev si pc_0 ∈ {eat, g} ; left_0 = me si pc_0 = eat ; sinon comme au § 3. Vérifié exhaustivement : dérivation + ces deux interdits == langage atteignable (n = 4..7 : 72/196/536/1464 états ; forme == BFS == langage).

### 9.3 La structure : toujours 7 nœuds, et le même intérieur

```
racine S' (7 clés, promesse sur pc_{n-1} comme S)
  (t,f)→A₀  (h,f)→A₀  (g,f)→A₁  (e,m)→A₁     // ligne A : pc_{n-1} ≠ eat
  (t,p)→B₀  (h,p)→B₀  (g,p)→B₁               // ligne B : pc_{n-1} = eat

A₀, A₁, B₀, B₁ : IDENTIQUES au cas régulier, ancres comprises (A₀→⊤, A₁→⊥, B₀→⊥, B₁→⊤)
```

Zéro nouvelle forme intérieure : la clé g n'existe dans aucune forme intérieure (le gaucher ne quitte pas sa position), l'irrégularité est entière dans S'. Mesuré sur les nœuds vivants du prototype plat : les signatures intérieures du gaucher sont exactement celles du cas régulier (mêmes 4 ensembles de clés ; une réplique de plus pour la signature (t,p)/(h,p) du bord, et la racine porte 7 clés au lieu de 5). Coût total : +1 nœud plat vivant (4n − 3 au lieu de 4n − 4), +3 firings, même couture, passes toujours 2.

Trace de découverte (mesurée) : les 8 premières signatures créées par la saturation du gaucher sont, dans l'ordre, celles du cas régulier ; les clés (g,f) et (g,p) n'apparaissent qu'aux rangs 9 et 12. L'irrégularité se greffe sur du matériel déjà en place, elle ne restructure rien.

### 9.4 Le comptage : même pli, racine plus riche

Mêmes matrices qu'au § 7 ; la racine S' ajoute une entrée A₁ ((g,f)) et une entrée B₁ ((g,p)) :

> |R'_n| = 2a₀(n-1) + 2a₁(n-1) + 2b₀(n-1) + b₁(n-1), avec a₀(ℓ) = 2a₀(ℓ-1) + a₁(ℓ-1), a₁(ℓ) = 2a₀(ℓ-1), a₀(0) = 1, a₁(0) = 0, et b idem avec b₀(0) = 0, b₁(0) = 1.

|R'_4| = 72, |R'_5| = 196, |R'_6| = 536, |R'_7| = 1464 (vérifiés par énumération), |R'_{20}| = 691 701 760. Ratio |R'_n| / |R_n| = 1,2887, constant sur n = 4..20 : même base (1+√3)^n, la classe irrégulière décale la constante, pas l'exposant.

### 9.5 Mesures plat : la même leçon avec l'irrégularité

| n | 5 | 10 | 20 | 30 |
|---|---|---|---|---|
| résultat final (nœuds vivants, ⊤ compris) | 17 | 37 | 77 | 117 |
| saturation : image max en cours | 16 | 36 | 76 | 116 |
| BFS naïf : image max en cours | 31 | 137 | 578 | 1 317 |
| saturation : firings / passes | 48 / 2 | 93 / 2 | 183 / 2 | 273 / 2 |

sat == BFS pour n = 4..7, pli == comptage BFS jusqu'à n = 10 (au-delà, le BFS explicite explose en mémoire). À n = 100 : 397 vivants, 78 625 créés (régulier : 78 325, donc +0,4 %) ; à n = 200 : 797 vivants. La leçon du § 6.2 tient avec l'irrégularité : l'image intermédiaire du naïf reste quadratique (1 317 contre 117 à n = 30), la saturation reste à résultat + 1, et l'irrégularité coûte une constante.

### 9.6 k gauchers (argument, à mesurer)

k positions irrégulières : des gauchers identiques partagent la même forme de racine (mêmes clés), mais le handle (contrôleur, ancre, forme) distingue les positions : O(k) handles ; les formes intérieures restent partagées entre tous les philosophes, réguliers ou non. La dégradation est linéaire dans le nombre de points irréguliers, pas dans la taille de l'anneau ; la hiérarchie SDD, elle, doit être redessinée pour chaque nouveau motif. C'est la prédiction falsifiable : si le jalon 4 mesure autre chose que O(k), la promesse du bord est fausse.