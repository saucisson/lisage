# État de l'art : structures de diagrammes de décision

Document de travail du projet « Généralisation et unification des diagrammes de décision ». Il prolonge la thèse de Linard, *Sémantique paramétrable des diagrammes de décision : une démarche vers l'unification* (UPMC, 2009), et **polyDD** (Linard, Paviot-Adet, Charron & Buchs, ACSD 2010).

---

## 1. Cadre de lecture

Un diagramme de décision (DD) encode une fonction sur un produit de variables, sous forme d'un DAG dont les nœuds internes testent une variable et dont les feuilles portent une valeur. Presque toutes les variantes existantes se distinguent le long des mêmes axes :

| Axe | Question sous-jacente | Exemples de positions |
|---|---|---|
| Domaine des variables | Booléen, entier fini, infini, continu ? | BDD / MDD / DDD / NDD |
| Valeurs des feuilles | Booléens, entiers, réels, fonctions ? | BDD / MTBDD / EVBDD |
| Valuation des arêtes | Aucune, entière, additive, ensembliste ? | BDD / EVBDD / SDD (Couvreur) |
| Hiérarchie | Les valeurs peuvent-elles être elles-mêmes des DD ? | HSDD (Hamez), SDD de Darwiche (vtree) |
| Canonicité | Forme unique sous un ordre donné ? | ROBDD, SDD ; non-canonique : DDD |
| Spécification des opérations | Bibliothèque codée en dur vs opérateurs déclaratifs | CUDD vs homomorphismes DDD |
| Stratégie d'exploration | Fire en largeur vs saturation | BFS classique vs saturation |

C'est précisément cette matrice qu'un cadre unifié doit couvrir : polyDD l'abordait par un **mécanisme de « Controller »** déclarant l'ordre, les valeurs d'arêtes et les opérateurs applicables. La section 9 refait l'exercice exhaustivement (novembre 2026) : une cinquantaine de familles, sources vérifiées, chacune instanciée dans les huit champs du contrôleur étendu de Lisage.

---

## 2. Souche classique : BDD et variantes booléennes et pseudo-booléennes

- **BDD / ROBDD** — Bryant, 1986. Forme canonique sous ordre des variables ; le point de départ de tout le domaine.
- **EVBDD** — Lai & Sastry, 1992. Arêtes valuées additivement (fonctions pseudo-booléennes).
- **ZDD (Zero-suppressed)** — Minato, 1993. Supprime les nœuds dont le fils « vrai » est vide : optimal pour les ensembles creux (combinatoire).
- **ADD (Algebraic DD)** — Bahar et al., 1993. Feuilles à valeurs arbitraires ; base de l'arithmétique symbolique.
- **MTBDD** — Fujita, McGeer & Yang, 1997. Feuilles multi-terminales pour fonctions à valeurs réelles/entières (analyse de Markov, PRISM).
- **BMD (Binary Moment Diagrams)** — Bryant & Chen, 1995. Arêtes affines, pour vérification arithmétique.
- **Bibliothèques** : CUDD (Somenzi), BuDDy (Lind-Nielsen), et **Sylvan** — van Dijk & Pol, 2016, framework **multi-cœur** pour BDD *et* MDD ([DOI](https://doi.org/10.1007/s10009-016-0433-2)). Sylvan montre qu'un framework générique peut absorber la dimension parallèle — un axe absent de polyDD.

**Leçon pour l'unification** : cette souche est bien balisée ; les axes « domaines » et « valeurs de feuilles » y ont été explorés systématiquement. La difficulté d'unification commence avec les opérations (chaque bibliothèque code Apply, restrict, etc. en dur) et le partage sous le vent (les bibliothèques ne partagent rien entre elles).

---

## 3. Souche multi-valuée et saturation : l'école Ciardo/Miner

- **MDD (Multi-valued DD)** — Miner & Ciardo, 1999, *Efficient Reachability Set Generation and Storage Using Decision Diagrams* ([DOI](https://doi.org/10.1007/3-540-48745-x_2)) : variables à domaines finis quelconques, MDD quasi-réduits, partage de sous-graphes pour les espaces d'états de réseaux de Petri.
- **Saturation** — développée ensuite par Ciardo, Lüttgen, Simnering, Miner (William & Mary) : explorer les nœuds du DD **en désordre**, en saturant chaque nœud par toutes les transitions locales avant de remonter, au lieu d'une BFS couche par couche. Gain exponentiel en pratique sur les espaces d'états.
- **EV⁺MDD / EVMDD** — arêtes valuées additives sur MDD, pour bornes de coûts/récompenses.
- **Synthèse de référence** : Ciardo, *Petri Net Analysis Using Decision Diagrams* (QEST 2009, [DOI](https://doi.org/10.1109/qest.2009.14)) — tutoriel de référence sur l'écosystème MDD + saturation.
- **Meddly** — bibliothèque open source extensible regroupant MDD, EV⁺MDD et variantes ; architecturalement proche de l'esprit « framework ».

**Leçon pour l'unification** : la saturation y est une **technique codée main** (l'ordre de saturation est lié au modèle, typiquement une hiérarchie de places). L'un défi de la reprise : faire de la saturation une *propriété déclarative* du contrôleur, pas un algorithme ad hoc — c'est exactement le pont vers le travail d'Hamez (§ 4).

---

## 4. La grande souche DDD/SDD/HSDD/ΣDD : une seule famille

**Correction du catalogue initial** : les « branches » DDD, SDD/HSDD et ΣDD ne sont pas trois souches indépendantes mais **une seule famille de recherche**, l'école de Paris sur les diagrammes de décision (LSV Cachan puis LIP6 : Couvreur, Thierry-Mieg, Kordon ; thèses menées en parallèle de Hamez et de Linard, LIP6/UPMC, 2009) et son extension genevoise (Buchs, Hostettler : ΣDD, MSDD). Relue dans la grille de Lisage, la filiation n'est pas trois positions dans le paramètre-space mais **une trajectoire** : chaque étape a nommé, sans le savoir, un constructeur du champ Σ. Cette famille a arpenté le champ avant qu'il ait un nom.

### 4.1 DDD : les données, et la répétition en germe

**DDD — Data Decision Diagrams**, Couvreur, Encrenaz, Paviot-Adet, Poitrenaud & Wacrenier, ATPN 2002 ([DOI](https://doi.org/10.1007/3-540-48068-4_8)).

- Variables à **domaines quelconques (potentiellement infinis)**, arêtes étiquetées par des données, feuilles 0/1.
- **Non canoniques** : l'ordre des variables peut varier le long des chemins ; c'est la contrepartie de la souplesse (permet l'in-place, l'itération en profondeur variable, les domaines dynamiques). Relue par Lisage : la répétition des variables, parquée comme « non-canonicité assumée », est le constructeur **list** en germe (« la répétition des DDD, domptée », proposition § 1.2).
- **Homomorphismes DDD** : les opérations sont des opérateurs inductifs (union, intersection, + sur variables, *itération transitive fixpoint*) que l'utilisateur compose ; l'implémentation **libDDD** en fait une bibliothèque déclarative. C'est historiquement la première généralisation « opérationnelle » des DD : on ne code plus le graphe, on code des fonctions sur le graphe.

**Lien direct avec la thèse de Linard (2009)** : la sémantique paramétrable, instanciée dans polyDD, généralise les homomorphismes DDD en séparant (i) la structure du graphe, (ii) la valuation des arêtes, (iii) la sémantique des opérateurs — le DDD devenant une instance parmi d'autres.

---

### 4.2 SDD (Set Decision Diagrams) : l'arc ensembliste (set/map)

- Étendent les DDD : la valeur d'une variable dans un chemin n'est pas une donnée atomique mais un **ensemble** (d'où la valuation ensembliste des arêtes), avec partage des ensembles entre variables.
- **Hiérarchie** : les ensembles portés par les arêtes peuvent eux-mêmes être des SDD sur d'autres variables → un SDD peut encoder des **régularités structurelles** (systèmes composés, tableaux symétriques, réseaux de Petri bien formés) exponentiellement plus compactes que tout DD plat.
- Implémentés dans **libDDD** (Couvreur/Thierry-Mieg), au cœur des outils **ITS/Stalina** puis **NeAT** (LIP6).

### 4.3 HSDD et la saturation automatique : le record imbriqué (Alexandre Hamez)

Le programme de recherche d'Hamez (thèse LIP6/UPMC : *Génération efficace de grands espaces d'états*, 2009) :

1. **HSDD + Automatic Saturation** — Hamez, Thierry-Mieg & Kordon, ATPN 2008 ([DOI](https://doi.org/10.1007/978-3-540-68746-7_16)) : définition formelle des SDD hiérarchiques, extension des homomorphismes à la hiérarchie, et surtout **saturation automatique** : l'itérateur fixpoint des DDD est remplacé par une stratégie qui détecte *dynamiquement* les nœuds à saturer, sans exiger de l'utilisateur une hiérarchie de variables ni un ordre de tir explicites (cf. la saturation « manuelle » de Ciardo, § 3).
2. **HSDD & Regular Models** — Thierry-Mieg, Poitrenaud, Hamez & Kordon, ATVA 2008 ([DOI](https://doi.org/10.1007/978-3-642-00768-2_1)) : utilisation de la hiérarchie pour encoder des **modèles réguliers** (blocs répétés, symétries) ; la structure du modèle est reflétée dans la structure du DD.
3. **Version journal** — Hamez, Thierry-Mieg & Kordon, *Building Efficient Model Checkers using Hierarchical Set Decision Diagrams and Automatic Saturation*, Fundamenta Informaticae 2009 ([DOI](https://doi.org/10.3233/fi-2009-137)) : la présentation de référence.
4. **Synthèse/contrôlabilité** — Zhang, Bérard, Kordon & Thierry-Mieg, IFAC 2010 ([DOI](https://doi.org/10.3182/20100830-3-de-4013.00047)) : au-delà de l'accessibilité, usage des HSDD pour la **synthèse de contrôleurs**.
5. **pnmc** — *A Symbolic Model Checker for Petri Nets*, 2016 ([DOI](https://doi.org/10.1007/978-3-662-53401-4_15)) : aboutissement industriel/outil de cette ligne (model checker symbolique open source, conception massivement parallèle).

À noter : Hamez a co-signé *Specification of Decision Diagram Operations* (2010) : le pont entre son programme et la sémantique paramétrable existait déjà, les deux thèses étant menées en parallèle dans le même laboratoire (LIP6/UPMC, 2009).

### 4.4 ΣDD/MSDD puis polyDD : Σ explicite

L'extension genevoise (Buchs, Hostettler) : **ΣDD** et **MSDD** (2009), algèbres de termes et multi-ensembles, précurseurs directs du « Σ = grammaire des variables » (proposition, § 1.2). **polyDD** (ACSD 2010) en est l'aboutissement : la sémantique paramétrable qui généralise toute la famille, DDD inclus.

**Leçon pour l'unification**, en deux points :

1. La saturation automatique démontre que **la stratégie d'exploration peut être calculée à partir de la structure du DD et des homomorphismes**, pas imposée par le modèle. Dans un cadre unifié, « saturation » devrait devenir un mode d'évaluation paramétrable (une sémantique du point fixe), dont la version de Ciardo (ordre statique de variables, § 3) et celle de Hamez (détection dynamique) sont deux instances.
2. La trajectoire de la famille se lit constructeur par constructeur : DDD la répétition (l'ordre libre), SDD l'arc-ensemble (**set**, la valuation d'arêtes), HSDD la hiérarchie (**record**, l'imbrication ; IPN folding la régularité du **list**), ΣDD l'algèbre explicite (Σ lui-même). Lisage ne fait que nommer le champ que cette famille a exploré pièce par pièce ; c'est la souche d'origine du cadre, et son test de complétude le plus exigeant.

---

## 5. Souches temporelles et numériques (mentionnées pour complétude)

- **CDD (Clock Decision Diagrams)**, **CRD** : horloges et contraintes temporelles (Larsen et al.).
- **DDD — Difference Decision Diagrams**, Møller & Andersen, ~2002 : contraintes de différence (DBM symboliques) pour les automates temporisés. ⚠️ Collision de nom avec les Data DD de Couvreur — un cadre unifié pourrait justement les recadrer comme valuations d'arêtes différentes.
- **NDD (Numerical DD)** — Wolper & Boigelot : automates acceptant les ensembles de Presburger, pour l'arithmétique réelle/entière.
- **IDD (Interval DD)** : domaines en intervalles.

---

## 6. L'autre généralisation : knowledge compilation (Darwiche et al.)

Une communauté parallèle a fait le même mouvement d'unification, mais **du côté logique** — à ne pas confondre avec les SDD ensemblistes de § 4.2 (même acronyme, structures différentes).

- **d-DNNF** — Darwiche, 2001 : décomposition conjonctive d'arbres.
- **Knowledge compilation map** — Darwiche & Marquis, 2002 (JAIR) : comparaison systématique des langages compilés par **classe de requêtes** supportées et **classe de transformations** — *le* paradigme concurrent du nôtre : classifier non pas par la structure du graphe mais par les opérations admissibles.
- **SDD sententiels (Sentential Decision Diagrams)** — Darwiche, AAAI 2011 : généralisation canonique des OBDD paramétrée par un **vtree** (arbre de décomposition disjonctif) ; les OBDD sont les SDD pour vtrees dégénérés. Mise en œuvre : package SDD, avec minimisation dynamique (Choi & Darwiche, AAAI 2013, [DOI](https://doi.org/10.1609/aaai.v27i1.8690)).
- **PSDD** — probabilistic SDD, Kimmig et al., 2014 : distributions sur SDD (comptage de modèles pondéré).
- **Z-SDD** — Choi, Xue & Darwiche, AAAI 2016 ([DOI](https://doi.org/10.1609/aaai.v30i1.10114)) : transplantation de l'idée zero-suppressed dans les SDD.
- **Canonicié** — *On the Role of Canonicity in Knowledge Compilation*, AAAI 2015 ([DOI](https://doi.org/10.1609/aaai.v29i1.9423)) : ce que coûte/apporte la canonicité.
- **Récent et pertinent** : *Canonical Decision Diagrams Modulo Theories*, 2024 ([DOI](https://doi.org/10.3233/faia241007)) — unification canonique de DD avec des théories de fond ; à lire de près comme concurrent direct de la reprise.

**Leçon pour l'unification** : le vtree de Darwiche joue le rôle du « contrôleur » : un paramètre structurel dont dépendent canonicité et complexité des opérations. Un cadre unifié devrait pouvoir exprimer *à la fois* la valuation d'arêtes (axe polyDD) et la décomposition logique (axe vtree) ; les deux se rencontrent dans la question de la canonicité conditionnelle.

---

## 7. Tentatives d'unification existantes — positionnement

| Approche | Ce qu'elle unifie | Ce qu'elle laisse de côté |
|---|---|---|
| **polyDD** (Linard et al., ACSD 2010, [DOI](https://doi.org/10.1109/acsd.2010.17)) + thèse de Linard 2009 | Sémantique paramétrable : ordres, domaines, valeurs, opérateurs via *Controller* | Hiérarchie native, saturation, parallélisme, canonicité conditionnelle |
| *Specification of Decision Diagram Operations* (2010) | Spécification déclarative des opérations | Passage à l'échelle, implémentation |
| Homomorphismes DDD (Couvreur et al. 2002) | Opérations sur domaines arbitraires | Structures booléennes classiques, canonicité |
| SDD de Darwiche (2011) + KC map (2002) | Langages de compilation vs requêtes | Tout le versant « gros graphes/partage mémoire » |
| Sylvan (2016) | BDD/MDD + multi-cœur | Domaines riches, hiérarchie, saturation |
| DD modulo théories (2024) | Canonicié + théories | Espace d'états, hiérarchie |
| [MEDDLY / paquet multi-domaine](https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12) (Miner et al. ; école Becker/Drechsler) | Tous les types MDD dans un paquet ; 11 types pseudo-booléens (BDD, FDD, OKFDD, MTBDD, EVBDD, BMD, *BMD, p*BMD, HDD, K*BMD, ZDD) | Unification par **framework**, sans contrôleur : chaque type recode ses réductions |
| [SLDD](https://www.ijcai.org/Proceedings/05/Papers/0857.pdf) (Wilson, IJCAI 2005) | Semi-anneaux de requête sur les arêtes de DD | Structure des atomes, attribution (le semi-anneau de **construction** n'est pas séparé) |

**Créneau clairement ouvert** : aucun cadre existant n'unifie simultanément (1) la valuation d'arêtes générique, (2) la hiérarchie HSDD, (3) la saturation comme stratégie *déclarative* (depuis Hamez, personne n'a généralisé la saturation automatique hors du contexte Petri/réseaux), (4) le parallélisme (Sylvan), et (5) les garanties de canonicité à la Darwiche.

---

## 8. Pistes immédiates pour la reprise

1. **Relire et reformuler** la sémantique paramétrable de la thèse de Linard (2009) en un langage algébrique unique (probablement : famille de foncteurs sur une catégorie de graphes à valuation — à creuser).
2. **Instancier le cadre** sur trois cas-tests étalons : BDD (canonique), DDD (non canonique, homomorphismes), HSDD (hiérarchique + saturation) — la saturation automatique d'Hamez comme instance d'un opérateur de point fixe paramétré.
3. **Intégrer la branche Darwiche** : prouver que le vtree est un cas particulier de contrôleur ; en tirer un théorème de canonicité conditionnelle.
4. **Prototype** : étendre polyDD (ou repartir de libDDD/Sylvan) avec contrôleurs de saturation et multi-cœur.
5. **Validation expérimentale** : Model Checking Contest comme banc d'essai (l'infrastructure 2018 conteneurisée existe déjà).

---

## 9. Le bestiaire 2026, instancié dans les paramètres de Lisage

La thèse de Linard (2009) cataloguait les familles d'alors. L'exercice ci-dessous le refait à novembre 2026, sources vérifiées par recherche bibliographique (liens dans la table 11), et surtout avec la grille des huit champs du contrôleur étendu (proposition, § 1.2) :

```
C = ( δ, Σ, décomposition + O, R, algèbre de défauts, D^déf, encodage, attribution )
attribution = canaux (hérité / synthétisé / mémoire) + semi-anneau de la requête + merge de construction
```

La **signature** d'une famille est sa position dans ce paramètre-space. Le BDD classique est l'origine du repère : δ booléen, Σ = Union(Bool)^n, O linéaire total, R-collapse + R-share, D^déf = ⊥, encodage absolu, hérité seul, merge ∨ idempotent. Chaque famille ci-dessous est caractérisée par **le champ qu'elle fait varier** (les autres restant à l'origine). Les attributions d'auteurs ne sont données que lorsqu'elles ont été vérifiées.

### 9.1 δ : le schéma d'atomes (ce que les clés peuvent être)

| Famille | Origine | Signature Lisage |
|---|---|---|
| BDD / ROBDD | Bryant, 1986 | L'origine du repère |
| MDD | Miner & Ciardo, ~1999 | δ énuméré ; la saturation y naît comme policy statique par niveaux |
| [LDD (list)](https://mcrl2.org/web/developer_manual/libraries/symbolic/symbolic_library.html) | bibliothèque symbolique mCRL2, [Sylvan](https://doi.org/10.1007/s10009-016-0433-2) | δ énuméré, arcs = listes d'arcs : la compilation *list* de Σ ; la fusion d'arcs adjacents ≈ R-collapse |
| DDD | Couvreur et al., 2002 | δ ouvert ; ordre libre le long des chemins, R-share seul (non canonique) ; les homomorphismes DDD = hérité seul, composition déclarative |
| DDD-différence | Møller & Andersen, ~2002 | δ = contraintes de différence (DBM) : la branche numérique du champ δ (cf. la collision de nom, § 5) |
| [CDD](https://link.springer.com/chapter/10.1007/3-540-48683-6_30) | école UPPAAL, 1998-99 | δ = différences d'horloges, ensembles non convexes |
| [CRD](https://link.springer.com/chapter/10.1007/0-306-47003-9_15), [CMD](https://www.researchgate.net/publication/221298169) | école temporisée | δ = matrices de contraintes : atomes totalement structurés |
| NDD | Wolper & Boigelot | δ = Presburger encodé par automates ; les feuilles sont des automates : le synthétisé est riche |
| IDD | cf. § 5 | δ = intervalles : la clé est un prédicat, pas une valeur |

### 9.2 D^déf et R : ce que l'arc absent dénote (la lignée la plus récente)

| Famille | Origine | Signature Lisage |
|---|---|---|
| ZDD | Minato, 1993 | D^déf = **absent** (≠ ⊥) : toute la différence BDD/ZDD tient dans l'algèbre de défauts |
| Z-SDD | Choi, Xue & Darwiche, 2016 | Idem, côté sententiel |
| CE-BDD | Bryant 1989 ; Brace, Rudell & Somer ~1990 | L'ancre porte un transformateur unaire (complément) |
| [TBDD, ESRBDD](https://www.cs.utexas.edu/~hunt/FMCAD/FMCAD17/preprints/s5p1.pdf) | FMCAD 2017 | La règle de réduction est désignée **par arête** (short / X / H0) : R devient un paramètre d'arc, exactement le champ R de C |
| [CESRBDD](https://link.springer.com/article/10.1007/s10009-021-00640-0) | Babar, Ciardo & Miner, STTT 2022 | ESR + complément : réduction désignée et complément coexistent par arête |
| [RexBDD](https://dl.acm.org/doi/10.1145/3649329.3656533) | Ciardo, Miner, Deng & Babar, DAC 2024 | + swap : les trois transformateurs unaires de polyDD (réduction, complément, swap) réunis en un seul paquet |

**Leçon** : la lignée Iowa State 2017-2024 (ESRBDD → CESRBDD → RexBDD) converge indépendamment vers « R désignée par le contrôleur », un des trois transformateurs de polyDD. Validation a posteriori du champ R de C ; Lisage l'absorbe sans extension, et va plus loin (R-share, R-collapse, défaut porteur comme paramètres globaux du contrôleur, pas seulement par arête).

### 9.3 Y, synthétisé, merge : la branche algébrique (semi-anneaux)

| Famille | Origine | Signature Lisage |
|---|---|---|
| MTBDD | Fujita et al., 1997 | Y = ℝ : synthétisé sur (ℝ, +, ×) |
| ADD | Bahar et al., 1993 | Y quelconque, apply pointwise : le merge **non idempotent** existe là dès 1993 |
| [SLDD](https://www.ijcai.org/Proceedings/05/Papers/0857.pdf) | Wilson, IJCAI 2005 | Arêtes étiquetées par le semi-anneau : le synthétisé-poids de la spec (§ 5), avant l'heure ; couvre CSP flous, probabilistes, bayésiens |
| EVBDD | Lai & Sastry, 1992 | Valuation affine additive : l'hérité porte (c₀, poids d'arc) |
| [FEVBDD](https://link.springer.com/article/10.1023/A:1008691605584) | FMSD 1999 | Poids × et + sur arêtes : le complément devient un poids multiplicatif |
| BMD, *BMD, p*BMD | Bryant & Chen, 1995 ; cf. le [paquet multi-domaine](https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12) | Moments : Y = fonctions, f = c₀ + c₁x : la décomposition de Davio est un merge algébrique |
| K*BMD | cf. le paquet multi-domaine | Kronecker × BMD : type de décomposition par variable |
| HDD | Clarke, Fujita & Zhao ([chapitre CMU](https://www.cs.cmu.edu/~emc/papers/Contributions%20to%20Edited%20Volumes/Multi-Terminal%20Binary%20Decision%20Diagrams%20and%20Hybrid%20Decision%20Diagrams.pdf)) | Hybride MTBDD × BMD |
| [AADD](https://ssanner.github.io/papers/anu/aadd.pdf) | Tafertshofer & Pedram, 1997 | Affine algébrique : quotient (constante, facteur) : l'ancre porte un couple |
| [EVMDD](https://www.researchgate.net/publication/4248060_Representations_of_Elementary_Functions_Using_Edge-Valued_MDDs) | Nagayama & Sasao, 2007 | Poids entiers sur MDD ; cf. aussi EV⁺MDD (§ 3) |
| FDD, KFDD, [OKFDD](https://link.springer.com/article/10.1023/A:1008635324476) | Becker, Drechsler, Sarabi et al., 1994-98 | **Merge = ⊕ de GF(2)** (Reed-Muller) : la fusion à la construction change de base, exactement l'axe A5 de la spec ; le dtl (Shannon / Davio± par variable) est une loi de nœud |
| [QMDD](https://arxiv.org/html/2401.01322v1) | 2006 | Y = matrices complexes, poids multiplicatifs sur arêtes |
| QuIDD | 2003, cf. la carte quantique | Y = ℂ : ADD restreint aux matrices |
| [LIMDD](https://arxiv.org/html/2108.00931v5) | 2022 | Poids = opérateurs locaux (Pauli) : arcs typés par un groupe, Y = états stabilisateurs |
| PSDD | Kimmig et al., 2014 | Distribution : synthétisé probabiliste sur SDD |

**Leçon** : presque tout ce zoo se lit comme (Y, semi-anneau du synthétisé, merge). C'est la branche que la spec § 5/§ 5.1 unifie (le pli exact ssi factorise par ce que la construction a préservé). Et le champ « merge de construction déclaré » reste une **case vide** : personne ne déclare son ⊕ de construction, tout le monde le code en dur.

### 9.4 O : l'ordre comme contrainte, et ses libérations

| Famille | Origine | Signature Lisage |
|---|---|---|
| OBDD | Bryant, 1986 | O linéaire total : l'origine |
| OKFDD (dtl) | Drechsler & Becker | O + loi de nœud par variable (Shannon, Davio+, Davio−) : le Order\<T\> étendu des lois de décomposition |
| SDD sententiels | Darwiche, 2011 | O = vtree (arbre de décomposition) : une contrainte, pas une séquence |
| dSDD, TSDD | filiation Darwiche | Variantes de vtree |
| free BDD | théorie ([Wegener, SIAM 2000](https://epubs.siam.org/doi/10.1137/1.9780898719789.bm)) | O = rien : canonicité perdue |
| IBDD, read-k-times | théorie | Répétition bornée des variables le long des chemins |
| [partitioned BDD](https://link.springer.com/article/10.1007/s002240010005) | Bollig, Löbbing & Wegener, 1999 | O = fonction de cas : partition de l'entrée |
| parity-rBP | Savický & Sieling | ⊕-nondéterminisme |

**Leçon** : O typé (proposition § 3.1) couvre OBDD et vtree. Free / IBDD / partitioned vivent dans le royaume non canonique (DDD-style) : le cadre peut les **exprimer** (O = relation) mais ne leur promet pas la minimalité (C1). La frontière nette est une contribution, pas un échec.

### 9.5 Σ, hiérarchie, encodage relatif : les familles structurelles

| Famille | Origine | Signature Lisage |
|---|---|---|
| SDD (ensembliste) | Couvreur et al. | Arcs = ensembles partagés : la valuation d'arêtes de polyDD |
| HSDD | Hamez et al., 2008-09 | Hiérarchie : l'arête porte un DD : Σ = record imbriqué |
| IPN, folding | Thierry-Mieg et al., 2009 | Régularité des modules : *Seq* réutilisé le long de la hiérarchie |
| ESDD | Devereux & Chechik, 2003 | Variable relative sur l'arc (shift) : l'encodage relatif de Lisage, avant l'heure |
| polyDD | Linard et al., 2010 | Valuation d'arêtes généralisée : l'aïeul direct de C |
| ΣDD, MSDD | groupe de Genève (Buchs, Hostettler) | Algèbre de termes / multi-ensembles : Σ comme algèbre libre, précurseur du « Σ = grammaire des variables » ; l'extension genevoise de la famille § 4 |
| MEDDLY | Miner et al. | Un paquet, tous les types MDD (y compris EV⁺MDD) : l'unification par framework, **sans contrôleur** |
| [paquet multi-domaine](https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12) | école Becker/Drechsler | 11 types (BDD, FDD, OKFDD, MTBDD, EVBDD, BMD, *BMD, p*BMD, HDD, K*BMD, ZDD) dans un paquet : l'unification côté synthèse formelle, toujours sans contrôleur |

**Leçon** : cette table coupe par champ, mais la généalogie compte autant que la grille (§ 4) : DDD → SDD → HSDD → IPN → ΣDD → polyDD est **une seule famille**, une trajectoire continue à travers le champ Σ. Les convergences du catalogue viennent d'équipes indépendantes ; les trajectoires, d'écoles continues. Lisage a besoin des deux lectures.

### 9.6 La policy : point fixe et flux

Saturation statique (Ciardo, ~2001-2004), saturation automatique (Hamez, 2008), saturation [on-the-fly multi-cœur](https://link.springer.com/chapter/10.1007/978-3-030-17465-1_4) (Sylvan, 2019), point fixe h* de libDDD ; et l'entrée la plus récente : [Q-Sylvan](https://arxiv.org/html/2508.00514v1) (2025), paquet quantique parallèle. Le fil est continu, du BDD aux stabilisateurs quantiques, dans des bibliothèques qui **réinventent chacune leurs réductions** : c'est le coût observable de l'absence de contrôleur unique.

---

## 10. Ce que le catalogue confirme

1. **Chaque case occupée est une instance de C.** Les huit champs sont validés rétroactivement, et les preuves les plus nettes sont les convergences récentes : la lignée ESRBDD/CESRBDD/RexBDD (2017-2024) redécouvre « R désignée » ; SLDD (2005) avait déjà le semi-anneau de requête ; LIMDD (2022) a des arcs typés par un groupe (valuation généralisée). Quand des équipes indépendantes réinventent les mêmes axes à quinze ans d'intervalle, c'est que les axes existent.
2. **Les cases vides sont les contributions.** (i) Le quotient par handle (contrôleur, ancre, forme) : personne. (ii) Le merge de construction déclaré (A5) : personne ne sépare construction et requête. (iii) L'attribution complète (hérité + synthétisé + mémoire) : polyDD seul, et sans les canaux. (iv) Σ comme grammaire des variables (record / union / list / set / map) : DDD parque la répétition comme anomalie, HSDD la capture en hiérarchie, personne n'en fait un **type**.
3. **Les cartes concurrentes.** Darwiche & Marquis (2002) et la [carte de compilation quantique](https://arxiv.org/html/2401.01322v1) (2024) classent par requêtes supportées ; Lisage classe par paramètres de contrôleur. La ligne « Compilation typée » de la spec (les opérations se définissent sur les types, le contrôleur traduit en transductions ordonnées) est le pont : une entrée du bestiaire = une signature, une signature = une classe de requêtes.
4. **Le bestiaire n'est pas infini.** Une cinquantaine de familles, huit axes, et la quasi-totalité des positions (δ, D^déf, R, Y, merge, O) déjà occupée **isolément**. L'espace restant est le produit croisé (attribution complète × grammaire × quotient par handle), qui est exactement le créneau identifié § 7.
5. **Une famille, une trajectoire** : la « branche DDD » du catalogue initial n'était pas trois souches mais un seul programme continu (§ 4) qui a découvert le champ Σ constructeur par constructeur. Leçon de méthode pour la suite : découper le bestiaire par champs, mais toujours vérifier les généalogies ; une école qui progresse à chaque papier sur le même axe est une preuve de l'existence de l'axe, au même titre qu'une convergence inter-équipes.

**Les familles qui résistent (réponse honnête).** Trois groupes, par gravité croissante :

1. *Hors C1, mais exprimables* : free BDD, IBDD, read-k-times, parity-rBP. O devient la relation triviale (ou bornée) : le cadre les code, mais le quotient par handle (contrôleur, ancre, forme) présuppose un futur déterminé ; sans contrainte d'ordre, la même sous-fonction revient avec des futurs différents, la table des handles perd sa clé, C1 ne s'applique pas. Le mémo (op, nœud) reste sain (L3 : homomorphisme en cet argument) ; c'est le partage qui devient heuristique. Frontière nette et assumée : le royaume non canonique (§ 9.4).
2. *Frontière structurelle : le coinductif.* Les cinq constructeurs de Σ sont tous bien fondés (μX.(V×X)+1, marqueur de fin requis). Les valeurs **infinies régulières** (streams, arbres rationnels) et les DD **cycliques** à sémantique de point fixe (circuits booléens bien formés, équations récursives) exigeraient un nœud de cycle et un quotient par bisimulation, hors C1. (Nuance du 10 novembre : le pool Lisage est cyclique en stockage, boucles et croisements A₀↔A₁, mais sa sémantique est le dépliage borné par l'ancre, un objet fini par n : la case vide concerne le coinductif non borné, pas le pli ; voir spec § 2.4.5.) C'est la seule case vide *structurelle* du bestiaire : candidat question ouverte n° 10 (la spec porte déjà la question 9 sur le continu).
3. *Frontière du royaume : pas des DD.* d-DNNF, AIG, circuits : pas de test de variable, la décomposition n'y est pas un ordre. Lisage ne les absorbe pas nativement, et ne doit pas : le pont est la compilation typée (conversion à coût prédictible), pas une signature. De même les atomes indécidables (hybrides, régions semi-algébriques) : δ doit être une clé finie décidable ; les encodages réels (CDD, CRD) sont couverts, le continu brut ne l'est pas, par construction.
4. *Frontière de taille, pas de couverture* : LIMDD compresse les états stabilisateurs ; l'état quantique arbitraire reste exponentiel pour **toute** famille de DD (carte quantique 2024), rien à voir avec le contrôleur. Et un cas non tranché, pas non couvert : le partage imbriqué irrégulier, hors conjecture d'unfold ; la tour de contrôleurs (proposition, § tour) en est la parade déclarée, à vérifier au jalon 5.

---

## 11. Références vérifiées

- Couvreur, Encrenaz, Paviot-Adet, Poitrenaud, Wacrenier — *Data Decision Diagrams for Petri Net Analysis*, ATPN 2002. https://doi.org/10.1007/3-540-48068-4_8
- Miner & Ciardo — *Efficient Reachability Set Generation and Storage Using Decision Diagrams*, ATPN 1999. https://doi.org/10.1007/3-540-48745-x_2
- Ciardo — *Petri Net Analysis Using Decision Diagrams*, QEST 2009. https://doi.org/10.1109/qest.2009.14
- Hamez, Thierry-Mieg, Kordon — *Hierarchical Set Decision Diagrams and Automatic Saturation*, ATPN 2008. https://doi.org/10.1007/978-3-540-68746-7_16
- Hamez, Thierry-Mieg, Kordon — *Building Efficient Model Checkers using HSDD and Automatic Saturation*, Fundamenta Informaticae 2009. https://doi.org/10.3233/fi-2009-137
- Thierry-Mieg, Poitrenaud, Hamez, Kordon — *Hierarchical Set Decision Diagrams and Regular Models*, ATVA 2008. https://doi.org/10.1007/978-3-642-00768-2_1
- Zhang, Bérard, Kordon, Thierry-Mieg — *Automated Controllability and Synthesis with HSDD*, IFAC 2010. https://doi.org/10.3182/20100830-3-de-4013.00047
- Hamez et al. — *A Symbolic Model Checker for Petri Nets: pnmc*, 2016. https://doi.org/10.1007/978-3-662-53401-4_15
- Linard, Paviot-Adet, Charron, Buchs — *polyDD: Towards a Framework Generalizing Decision Diagrams*, ACSD 2010. https://doi.org/10.1109/acsd.2010.17
- Linard, A. — *Sémantique paramétrable des diagrammes de décision : une démarche vers l'unification*, thèse, Université Pierre et Marie Curie, 2009 (2009PA066655, theses.fr).
- Linard, Hamez et al. — *Specification of Decision Diagram Operations*, 2010.
- van Dijk & Pol — *Sylvan: multi-core framework for decision diagrams*, 2016. https://doi.org/10.1007/s10009-016-0433-2
- Choi & Darwiche — *Dynamic Minimization of Sentential Decision Diagrams*, AAAI 2013. https://doi.org/10.1609/aaai.v27i1.8690
- Choi, Xue & Darwiche — *Zero-Suppressed Sentential Decision Diagrams*, AAAI 2016. https://doi.org/10.1609/aaai.v30i1.10114
- Darwiche & Marquis — *A Knowledge Compilation Map*, JAIR 2002.
- Darwiche — *On the Role of Canonicity in Knowledge Compilation*, AAAI 2015. https://doi.org/10.1609/aaai.v29i1.9423
- *Canonical Decision Diagrams Modulo Theories*, 2024. https://doi.org/10.3233/faia241007
- Babar & Ciardo — *Tagged BDDs: Combining Reduction Rules from Different Decision Diagram Types*, FMCAD 2017. https://www.cs.utexas.edu/~hunt/FMCAD/FMCAD17/preprints/s5p1.pdf
- Babar, Ciardo & Miner — *CESRBDDs: BDDs with Complemented Edges and Edge-Specified Reductions*, STTT 24, 2022. https://link.springer.com/article/10.1007/s10009-021-00640-0
- Ciardo, Miner, Deng & Babar — *RexBDDs: Reduction-on-Edge Complement-and-Swap Binary Decision Diagrams*, DAC 2024. https://dl.acm.org/doi/10.1145/3649329.3656533
- Wilson — *Decision Diagrams for the Computation of Semiring Valuations*, IJCAI 2005. https://www.ijcai.org/Proceedings/05/Papers/0857.pdf
- Lai, Pedram et al. — *Factored Edge-Valued Binary Decision Diagrams*, FMSD 1999. https://link.springer.com/article/10.1023/A:1008691605584
- Drechsler & Becker — *Efficient Representation and Manipulation of Switching Functions Based on OKFDDs*. https://www.researchgate.net/publication/4225957
- *Implementation of a Multiple-Domain Decision Diagram Package* (BDD, FDD, OKFDD, MTBDD, EVBDD, BMD, *BMD, p*BMD, HDD, K*BMD, ZDD). https://link.springer.com/chapter/10.1007/978-0-387-35190-2_12
- *K*BMDs: A New Data Structure for Verification*. https://www.researchgate.net/publication/3627375_KBMDs_a_new_data_structure_for_verification
- Clarke, Fujita & Zhao — *Multi-Terminal Binary Decision Diagrams and Hybrid Decision Diagrams* (chapitre CMU). https://www.cs.cmu.edu/~emc/papers/Contributions%20to%20Edited%20Volumes/Multi-Terminal%20Binary%20Decision%20Diagrams%20and%20Hybrid%20Decision%20Diagrams.pdf
- Tafertshofer & Pedram — *Affine Algebraic Decision Diagrams* (survey AADD, ANU). https://ssanner.github.io/papers/anu/aadd.pdf
- Nagayama & Sasao — *Representations of Elementary Functions Using Edge-Valued MDDs*, 2007. https://www.researchgate.net/publication/4248060_Representations_of_Elementary_Functions_Using_Edge-Valued_MDDs
- Vos et al. — *LIMDD: A Decision Diagram for Simulation of Quantum Computing Including Stabilizer States*, 2022. https://arxiv.org/abs/2108.00931
- *A Knowledge Compilation Map for Quantum Information*, 2024 (taxonomie MTBDD/ADD/QuIDD/SLDD×/QMDD/LIMDD). https://arxiv.org/abs/2401.01322
- *Q-Sylvan: A Parallel Decision Diagram Package for Quantum Computing*, 2025. https://arxiv.org/abs/2508.00514
- mCRL2 — *List Decision Diagrams* (bibliothèque symbolique). https://mcrl2.org/web/developer_manual/libraries/symbolic/symbolic_library.html
- van Dijk et al. — *Multi-core On-The-Fly Saturation*, 2019. https://link.springer.com/chapter/10.1007/978-3-030-17465-1_4
- Wegener — *Branching Programs and Binary Decision Diagrams*, SIAM 2000 (free BDD, IBDD). https://epubs.siam.org/doi/10.1137/1.9780898719789.bm
- Bollig, Löbbing & Wegener — *An Improved Hierarchy Result for Partitioned BDDs*. https://link.springer.com/article/10.1007/s002240010005
- Bengtsson, Larsen et al. — *Efficient Timed Reachability Analysis Using Clock Difference Diagrams*. https://link.springer.com/chapter/10.1007/3-540-48683-6_30
- Wang — *Symbolic Verification of Complex Real-time Systems with Clock-Restriction Diagram*. https://link.springer.com/chapter/10.1007/0-306-47003-9_15
- *Fully Symbolic Timed Model Checking Using Constraint Matrix Diagrams*. https://www.researchgate.net/publication/221298169