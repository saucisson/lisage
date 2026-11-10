# Spécification : le calcul des transductions attribuées

*Formalisation du cœur de Lisage sur l'instance de référence (jalon 1 : atomes bornés, `Set`, `Map`). `lookup` est l'opération-test. Chaque section se termine par ce qui est démontrable, démontré, ou laissé ouvert. La section 2.4 est une extension discutée (défauts généralisés, formes relatives), hors du périmètre du jalon 1.*

---

## 1. Périmètre

Instance de référence du contrôleur étendu :

- **Schéma d'atomes** δ : *borné* (domaines finis, énumérables, avec défaut éventuel).
- **Algèbre de types** : `Atom(D)`, `Set(T)`, `Map(K, V)` — clés et valeurs bornées. (`Seq`/`Tag` : jalon 2.)
- **Canonicité** : canonique (minimisation), avec suppression de défaut (représentation creuse).
- **Attribution** : les trois canaux, synthétisé sur semi-anneau.

**Principe posé (grammaire des variables)** : un DD n'a pas de séquence de variables ; ses niveaux déplient une expression de Σ (record, union, list, set, map). La répétition de variables des DDD est le dépliage de list/set/map : répétition d'une variable + marqueur de fin, une **clé requise** (jamais un défaut absorbable). Les opérations du niveau 1 sont définies sur les types, **indépendantes de l'ordre**, puis traduites par le contrôleur en transductions ordonnées. Le jalon 1 n'explicite qu'une compilation : le balayage d'univers (Set/Map bornés : un niveau de présence par clé, δ = absent, sans marqueur) ; la compilation énumérative (marqueur de fin) arrive avec `Seq` (jalon 2) et les schémas ouverts.

Un modèle d'exemple unique sert tout le document :

```
K = {a, b}      (ordre canonique a ≺ b)      V = {0, 1}
S = { {a↦1, b↦0}, {a↦0, b↦0} }              (un ensemble de maps)
```

---

## 2. Décompositions et diagrammes

### 2.1 Décomposition

Une **décomposition** `Δ` est un arbre (ou DAG) de *niveaux*. Chaque niveau `ℓ` porte :

- `Val(ℓ)` : domaine fini des arcs (valeurs de décision) ;
- `Δ(ℓ, v)` : sous-décomposition successeur de l'arc `v` ;
- `δ(ℓ) ∈ Val(ℓ) ∪ {⊥}` : **valeur par défaut** (arc supprimable), paramétrée par l'algèbre de défauts du contrôleur.

Pour `Map(K, V)` borné (ordre canonique `k₁ ≺ … ≺ kₙ`) :

```
k_i :  présence ∈ {absent, présent}   δ(k_i) = absent
         └─ si présent : val(k_i) ∈ V   (niveau atomique, δ = ⊥)
```

Le niveau *présence* est exactement la valuation ensembliste des SDD ; `δ = absent` est la convention creuse (généralisation directe du ZDD).

### 2.2 Diagrammes sémantiques

```
D ::= ε(y)                    terminal de payload y ∈ Y
    | ν(ℓ, {v₁ ↦ D₁, …, vₘ ↦ Dₘ})     nœud de niveau ℓ
```

Les arcs absents d'un nœud signifient `δ(ℓ)` (suppression de défaut). **Sémantique ensembliste** (l'interprétation A/B, ensemble ou fonction, tranchée ici) :

```
⟦ε(y)⟧        = {( [], y )}                          un chemin vide à payload y
⟦ν(ℓ, A)⟧    = ⋃_{v ↦ D ∈ A} { ( [v·w], y ) | (w, y) ∈ ⟦D⟧ }
              ∪  { ( [δ(ℓ)·w], y ) | (w,y) ∈ ⟦D_δ⟧ }   si le défaut est « porteur »
```

Le cas « défaut porteur » (un arc absent continue le chemin avec `δ(ℓ)`) caractérise la présence : une clé absente reste absente dans toute la suite. Un DD dénote donc un **ensemble de chemins à payload** : l'interprétation-fonction (`X → Y`) est le cas où chaque préfixe d'affectation admet au plus un payload ; l'interprétation-ensemble (`Y = Bool`) est le cas fonctionnel trivial. *Les deux lectures sont deux conditions de bien-typage, pas deux objets.*

### 2.3 Réductions = quotients

Sur `Δ` fixé, `D` dénote une fonction partiellement définie (préfixes → payloads). Deux diagrammes sont **équivalents** (`≡`) s'ils dénotent la même fonction. Les règles de réduction sont des simplifications préservant `≡` :

```
(R-collapse)    ν(ℓ, {v ↦ D | v ∈ Val(ℓ)})  →  D          (tous les arcs mènent au même D)
(R-default)     ν(ℓ, A ⊎ {δ(ℓ) ↦ D})  →  ν(ℓ, A)          si D ≡ D_δ (« porteur du défaut »)
(R-share)       sous-DD identiques ⇒ même nœud (interning, structurel)
```

`Canon(D)` = forme minimale modulo ces règles — la coalgèbre minimale du § 1.3 de la proposition. Sur l'exemple :

```
S canonique :
        k_a?
        └─ présent
           └─  [R-collapse sur val(a) : 0 et 1 mènent au même sous-DD]
              k_b?
              └─ présent
                 └─ val(b)=0 ↦ ε(⊤)
```

**Observation décisive pour la suite** : après `R-collapse`, le niveau `val(a)` n'apparaît plus — la valeur de `a` est devenue *implicite* (tout valeur convient). La représentation a perdu une information qu'elle n'avait pas besoin de garder *en tant qu'ensemble*… mais que certaines opérations devront retrouver (§ 6).

### 2.4 Extension : défauts généralisés et formes relatives (jalons 4–5)

Deux généralisations du système de 2.3, discutées après sa rédaction. Elles prolongent la méta-théorie sans la modifier : le jalon 1 implante 2.3 ; l'extension est pour les jalons 4–5, derrière l'oracle.

#### 2.4.1 Défaut généralisé (D-abs)

(R-default) n'absorbe que l'arc étiqueté `δ(ℓ)`. Généralisation : le contrôleur désigne un **diagramme de défaut** `D^déf` (une forme canonique désignée *par contrôleur*, jamais par niveau, jamais un sous-DD libre : sinon la sémantique d'un nœud ne serait pas récupérable depuis (forme, contrôleur)). Toute clé dont la cible ≡ `D^déf` est alors absorbable, sur **n'importe quel sous-domaine** :

```
(D-abs)    ν(ℓ, A ⊎ {v ↦ D^déf | v ∈ R})  →  ν(ℓ, A)     pour tout R ⊆ Val(ℓ)
(D-trans)  ν(ℓ, A) avec A = ∅ ou toutes cibles ≡ D  →  D   nœud transparent, écarts composés
(R-share)  formes égales ⇒ même nœud                       inchangé, structurel
```

(R-collapse) et (R-default) deviennent des cas particuliers ; l'instance se choisit dans l'algèbre de défauts :

| Instance | `D^déf` désigné | Effet |
|---|---|---|
| BDD (don't care) | la continuation-saut δ | absorption classique ; D-trans = collapse |
| ZDD (présence) | le diagramme « vide » de présence | zero-suppression |
| MTBDD | le terminal neutre | absorption des arcs nuls |

Cas limite : « le reste ↦ D₂ ≠ D^déf » (MDD à domaine large) n'est pas compressible par D-abs seul ; il l'est si le contrôleur désigne D₂ comme défaut pour cette application. La désignation est un paramètre du contrôleur, pas de la forme.

#### 2.4.2 Formes relatives : la variable quitte le nœud

Seconde extension : l'identité de variable peut quitter le nœud pour l'arc prédécesseur.

```
Forme relative :  ν( K )          K : clés finies, v ↦ (écart_v, sous-forme_v)
handle = (forme, ancre)           ancre = variable absolue de la racine
ℓ(n) = ℓ(parent) + écart          l'écart s'accumule dans le canal hérité (φ_disp)
```

- **Encodage polymorphe** : chaque contrôleur décide si ses nœuds portent la variable (absolu), la reçoivent de l'arc (relatif), ou mélangent les deux avec une **précédence** fixée par contrôleur ; un nœud absolu atteint par un arc relatif resynchronise l'accumulation (point de resynchronisation, utile aux événements à empan non uniforme).
- **Typage par inclusion** : le placement d'une forme s sur la variable x est valide ssi `clés(s) ⊆ Val(x)` ; le complément est couvert par l'arc absent. Le partage entre variables à domaines différents est sain sous **inclusion**, pas sous égalité.
- **Réductions inverses** : ré-explicitier une variable supprimée (quantification, substitution nommée) = T-EXPAND (§ 6.2), disponible car les opérations parcourent de haut en bas, accumulent, et sont définies sur un contrôleur qui connaît l'ordre.

#### 2.4.3 Identité : le triple (contrôleur, ancre, forme)

Le pool de bas niveau est un **stockage canonique sans aucune autre connaissance** : plusieurs contrôleurs coexistent ; deux formes au même niveau peuvent résulter de deux réductions différentes portées par deux contrôleurs distincts ; la lecture d'une forme est fixée par celui qui traverse.

> **La canonicité migère du nœud vers le handle.** Contre-exemple : s = {0 ↦ D, 1 ↦ D} avec D ≢ `D^déf` est collapsable sur une variable booléenne (sa forme canonique y est D seul) et irréductible sur une variable à trois valeurs (l'absent y couvre 2 ↦ `D^déf`). L'invariant classique « tout nœud stocké est réduit » disparaît ; il devient : tout handle est canonique pour sa lecture (contrôleur, ancre). Conséquences : la réduction s'applique à la création du handle (l'ancre y est connue) ; la table des handles se clé sur (forme réduite pour cette lecture, ancre) ; les conversions entre royaumes de contrôleurs = transductions à coût prédictible (le contrat typé s'étend des ordres aux encodages).

> **(C1, à prouver)** Confluence du système {D-abs, D-trans, R-share}, puis minimalité par handle. Le format de T5 s'étend : la canonicité est relative au contrat (contrôleur, ordre, ancre), jamais une propriété de la structure nue.

- **Caches** : les clés du computed-table portent l'empreinte du contrôleur (au minimum son algèbre de défauts). En régime homogène, les opérations équivariantes se clé sur la différence d'ancres ; en régime hétérogène, les ancres reviennent dans les clés.
- **Saturation** : elle itère sur des *usages* (paires forme × ancre), pas sur des formes ; l'index niveau → formes utilisées vit dans le plan d'ombre. Le stockage est partagé, pas le parcours (sauf ci-dessous).

#### 2.4.4 Ce que l'extension ouvre

- **Partage de travail (falsifiable)** : sur N composantes répliquées à événements invariants par translation, le computed-cache déduplique le travail entre réplicas, pas seulement le stockage. Énoncé : *coût en O(1) pour le cœur des réplicas, plus un terme de bord* (la couture d'un anneau brise l'invariance). Benchmark : n philosophes, ligne de base : l'encodage SDD hiérarchique.
- **À plat ce que la hiérarchie fait en SDD (conjecture)** : le shift à plat est l'unfold d'une hiérarchie *régulière* ; si la décomposition est uniforme, le plat égale le vtree avec moins de machines ; si elle est irrégulière, il perd. C'est le test de complétude (SDD sententiels sans extension ad hoc) qui ressort : l'instance relative est peut-être la brique qui manquait pour installer les SDD dans le monde plat.
- **Lignée** : ESDD ([Devereux & Chechik 2003](https://www.oldcitypublishing.com/journals/mvlsc-home/mvlsc-issue-contents/mvlsc-volume-9-number-1-2003/mvlsc-9-1-p-59-86/)) : multi-valués, partage de structure, canonicité par conditions suffisantes sur les transformateurs unaires d'arêtes ; pré-saturation, jamais adopté. CESRBDD ([Babar/Ciardo/Miner, STTT 2022](https://link.springer.com/article/10.1007/s10009-021-00640-0)) puis RexBDD ([Ciardo/Miner/Deng/Babar, DAC 2024](https://par.nsf.gov/biblio/10527627-rexbdds-reduction-edge-complement-swap-binary-decision-diagrams)) : la réduction sur l'arête, complément et swap. L'apport de Lisage : l'unification (tout transformateur unaire d'arête = une valuation structurelle, instance du contrôleur) et l'intégration (typage par inclusion, canonicité par handle, T-EXPAND, oracle).

#### 2.4.5 Deux décalages de plus : ancres composées, valeurs relatives (10 novembre 2026)

Une seule idée, deux actions de groupe : **la forme est relative, le contexte porte l'absolu**. L'ancre positionnelle (2.4.2, 2.4.3) en est le cas où le groupe agit sur les indices ; deux extensions complètent la symétrie.

- **Ancres composées (décalages nommés)** : quand le réseau factorise en plusieurs fragments distincts (périodes différentes, entrelacés dans l'ordre), l'ancre devient un **enregistrement nommé** de décalages, un par fragment ({machine : +i, convoyeur : +j}). Les formes se factorisent par fragment (des petites formes composées, au lieu du mot énuméré sur le ppcm des périodes) ; la dégradation reste en O(Σ kᵢ) points irréguliers, jamais en O(∏) (falsifiable : une paire d'irrégularités indépendantes ne doit pas coûter le produit) ; les coutures se comptent par frontière de fragment. Canonicité : ordre lexicographique des noms, zéro = début de fragment ; l'ancre reste dans le handle (C1 inchangé), le pool reste aveugle. Le § 9.6 du document philosophes (k gauchers : O(k) handles, formes partagées) en est le cas particulier à un fragment ; la tour de contrôleurs reste réservée à l'imbriqué irrégulier ; la décomposition en fragments est une décision de l'oracle.
- **Transformations de clés portées par le contrôleur** : rien de spécifique au décalage ne vit sur l'arc, et rien n'y vit du tout qui ne soit un **terme de la grammaire des clés**. Le contrôleur désigne, comme pour tout le reste, un **groupe G de transformations** agissant sur le domaine des clés ; les formes sont normalisées modulo G par une **section canonique des orbites** (choisie par le contrôleur) ; la **fonction** vit dans le contrôleur, **l'arc entrant porte l'argument** : pour un décalage, la valeur effective de n (le minimum des clés vraies dans la section min = 0) ; le terme de l'arc est f(n), l'arc reste (terme, handle), le pool reste aveugle, et le hash-consing se clé sur la forme réduite comme dès M1-1. Le précédent est déjà dans le document philosophes (§ 4) : le shift y vit sur l'arc, +1 par cellule. L'argument participe au hash de la forme du **parent** (comme donnée du terme), jamais interprété par le pool. Le décalage +n en est l'instance (G = (ℤ,+), section : l'arc le plus petit étiqueté 0, le terme de reconstruction sur l'arc entrant) ; l'arête complémentée en est une autre (G = Z₂, section : la convention du terminal) ; le swap de RexBDD une troisième (action sur les fils, même rang : l'attribut vit sur l'arête). Contrat d'algèbre ajouté (il prolonge celui des clés-régions) : (i) l'action est **décidable et inversible** (un groupe : la lecture absolue et le produit exigent t⁻¹, renormalisation t_B∘t_A⁻¹, O(1) termes par paire) ; (ii) les transformations sont des **bijections** (D^déf invariant : le complément commute, « partout ailleurs » ne change pas) ; (iii) la section canonique est **confluente** (forme normale, sinon pas de hash-consing) ; (iv) l'admissibilité (3.3) exige que l'opération commute à l'action ou porte ses paramètres relatifs, et le mémo (L3) se clé sans le terme pour les opérations invariantes ; (v) la famille f est **close par composition** avec une loi calculable sur les arguments : f(n₁)∘f(n₂) = f(n₁ ⊕ n₂), un plongement homomorphe de (A, ⊕) dans G (l'accumulation de la lecture absolue se fait sur les arguments, O(1) par pas) ; (vi) l'argument est stocké **modulo le noyau de f** : f(n) = f(n') ⇒ même écriture canonique de n et n', sinon deux orthographes d'une même transformation divisent des parents sémantiquement identiques et C1 est violé relativement à l'algèbre des clés. Lecture : clé effective = f(n)(clé stockée) ; l'invariant de Q6 s'étend : lecture absolue = composition des arguments (n₁ ⊕ n₂ ⊕ …). Applications : compteurs non bornés ((ℤ,+)), classes de couleurs (permutations), régions convexes (translation de (ℝᵈ,+) : le premier fragment δ continu bon marché) ; la clôture DBM n'est **pas** de cette forme (opérateur de fermeture, pas une section d'orbite) et garde son propre traitement. Instance vivante vérifiée le 10 novembre : la fenêtre glissante (canvas exemple-fenetre-glissante-lisage) : la base de séquence = l'argument sur l'arc d'entrée, l'ancre composée {base, horizon} désigne la course réglée, et Wireshark fait la section canonique min = 0 par défaut (numéros de séquence relatifs) : le précédent industriel de la clause (vi). Cousins vérifiés (état de l'art § 11) : arêtes complémentées (Z₂, côté terminal), EVBDD/FEVBDD/AADD (additif, côté terminal), DDD (contraintes de différence, canonicité par clôture) ; la case (groupe désigné par le contrôleur, côté clés, section locale) n'est pas au bestiaire : à vérifier comme revendication, pas à présumer.
- **Composition** : les deux coexistent, l'ancre (position) dans le handle, le terme de transformation (valeur) dans la grammaire des clés ; les philosophes n'en ont pas besoin (clés symboliques), les compteurs et les horloges si. Comme toujours : côté algebra/, zéro changement du pool.
- **L'incrément des variables : une fonction du contrôleur, paramétrée par l'arc, comme pour les valeurs** (précision du 10 novembre). Rien ne fige le pas : incrémenter une variable (le décalage d'ancre d'un niveau, +1 par cellule ; l'effet d'un événement sur un compteur, une horloge, la base de séquence), c'est appliquer **une transformation désignée par le contrôleur dont le paramètre vit sur l'arc**, exactement le mécanisme des transformations de clés. L'écriture relative (x ↦ x ⊕ δ) en est le cas général, l'écriture absolue (x ↦ v) le cas dégénéré (une transformation constante) ; les deux sont des termes de la même grammaire, et les clauses (v)-(vi) couvrent l'incrément sans retouche : composition des incrémentations = accumulation des arguments (O(1) par pas, l'invariant de Q6), stockage modulo le noyau. Instance : la glisse de la fenêtre glissante avance la base de f(δ), δ porté par l'événement ; la trace vérifiée n'exhibe que δ = 1, mais rien dans le montage n'exige le pas constant : un acquit de r cellules d'un coup est le même terme f(r). Unification : l'ancre (position), la transformation de clés (lecture) et l'incrément de variable (effet) sont **trois emplois d'un seul mécanisme** (la fonction dans le contrôleur, l'argument sur l'arc, le pool aveugle) ; le pool n'apprend rien de nouveau.
- **L'arc de racine : la valeur d'un DD est un arc**. La normalisation des formes prive la racine de son arc entrant ; l'argument de la racine vit donc sur un **arc d'entrée virtuel**, et le type « DD autonome » est le couple **(terme f(n), handle)**, pas le nœud nu. Conséquences : (i) les opérations prennent et rendent des arcs (les transductions signent arc → arc), la descente démarre avec l'argument d'entrée, et l'invariant de lecture absolue commence là ; (ii) le mémo se clé sur la forme seule pour les opérations invariantes (l'argument est déjà normalisé par construction), et sur (forme, argument) sinon ; (iii) deux DD partageant tous leurs nœuds mais différant d'entrée sont deux DD distincts : R₅ et R₅₀ partagent les 7 nœuds, l'entrée porte le paramètre ; (iv) Liseuse rend l'arc d'entrée et son terme, c'est l'identité visuelle du diagramme. Le rang est standard dans les familles à attribut d'arête : le pointeur de racine de CUDD porte son bit de complément, la racine d'un EVBDD porte son offset ; la valeur d'un DD à attributs est une arête, jamais un nœud nu. Rien de tout cela ne matérialise un nœud : le couple (terme, handle) est porté par l'appelant (algebra/, plan/), le pool reste inchangé.
- **Le DD reste un DAG, le pool ne l'est pas**. Le pool est cyclique en stockage, et l'a toujours été : boucle (t,f) sur A₀, arcs croisés A₀↔A₁ (le pli des philosophes, l'automate minimal du langage des cellules légales) ; le « DAG » du niveau 2.1 est la sémantique, pas le pool. À n fixé, le dépliage parcourt n cellules puis prend l'arc d'ancre vers les terminaux : acyclique par construction. Le pool est donc une **présentation finie d'une famille infinie de DAG** (un par n), pas un objet infini : la conjecture d'unfold (2.4.4) en acte. C'est aussi pourquoi ce cycle n'occupe pas la case « cyclique » du bestiaire (état de l'art, question ouverte 10 : les DD cycliques à sémantique de point fixe décrivent un objet infini et exigent un nœud de cycle + un quotient par bisimulation) : ici le point fixe n'est **jamais pris**, l'ancre borne le dépliage, C1 tient sans bisimulation. Conséquence de terminaison : elle ne vient plus de l'acyclicité du pool mais du **bon ordre sur l'argument accumulé** (shift +1 par cellule, l'ancre est la borne) ; la descente est à profondeur bornée sur un graphe cyclique, le mémo se clé sur (forme, argument). Et l'arc d'entrée est le point où n entre dans le cycle : sans lui, la boucle de A₀ serait illisible (rotation infinie ou point fixe). Terminologie à tenir : « DD » désigne le déplié (DAG), « pool » le plié (graphe cyclique) ; l'entrée est un arc dans le pool, la valeur une racine du DAG déplié. Vérifié sur les philosophes (simulation, 10 novembre) : la construction ascendante niveau par niveau depuis le terminal converge vers exactement les 7 nœuds (les 4 formes au premier niveau, zéro création ensuite, S au niveau n), parce que l'ancre y est une table de verdicts et l'argument vit sur l'arc ; le même montage en absolu, verdicts matérialisés en nœuds, crée 4 nœuds par niveau indéfiniment (4n − 4 vivants, la ligne du prototype plat) : la convergence est un effet de la convention d'ancre, pas du terrain. Et l'exécution **ne déplie jamais** : la descente interprète le pool cyclique sur des **paires (forme, argument)**, le mémo garantit qu'une paire est payée une fois, et le coût d'une opération est le **nombre de handles visités, jamais le nombre de chemins** (l'explosion combinatoire vit dans les chemins : 2ⁿ pour les philosophes ; le DD canonique déplié n'a que Θ(n) handles ; le pool, O(1) formes). Deux régimes (clause iv) : opérations **invariantes** par l'argument (verdicts, gardes) → mémo sur la forme seule ; opérations non invariantes (plis, comptes) → pas de table sur l'argument mais un **accumulateur** : la composition de la clause (v) garde O(1) état par niveau, |R₅| = 536 754 176 se compte en O(n) (et en O(log n) par exponentiation du pli, le pool étant un cycle à transition constante), jamais en O(|R_n|). Le mémo est **transitoire** (par appel), le pool **persistant** : c'est lui qui reste à O(1) quand l'argument vit sa vie (la base de la fenêtre parcourt ℕ, le pool reste 4). Pire cas avoué : si le DD canonique déplié est exponentiel (modèle sans régularité), le mémo l'est aussi ; C1 est la minimalité, pas la magie, et la promesse est de **ne jamais payer plus que le canonique**, le stockage de la famille restant partagé.

---

## 3. Le calcul des transductions attribuées

### 3.1 Définition

Une **transduction attribuée** `τ` (pour un contrôleur fixé) est donnée par :

- un ensemble d'**attributs hérités** `H`, avec un élément initial `h₀` ;
- un ensemble d'**attributs synthétisés** `S`, muni d'une structure d'agrégation ;
- une **mémoire de niveau** `M : (id(nœud), ℓ) ⇀ valeur`, avec tampons ;
- trois fonctions partielles du contrôleur :

```
φ_disp :  (ℓ, h, v)  ↦  h'            dispatch : attribut hérité par arc
φ_aggr :  (ℓ, h, {v ↦ s})  ↦  s       synthèse : attribut remontant du nœud
φ_out  :  (ℓ, h, {v ↦ (D', s)}, M)  ↦  (nœud résultat, effet mémoire)
```

et deux règles d'exécution mutuellement récursives :

```
(T-TERM)   τ(ε(y), h) = ( ε(ψ_y(y, h)) ,  σ_y(y, h) )

(T-NODE)   n = ν(ℓ, A)
           hᵢ = φ_disp(ℓ, h, vᵢ)              pour chaque arc vᵢ ↦ Dᵢ ∈ A
           (Dᵢ', sᵢ) = τ(Dᵢ, hᵢ)             récursion (descendante)
           s  = φ_aggr(ℓ, h, {vᵢ ↦ sᵢ})       synthèse (remontante)
           (n', μ) = φ_out(ℓ, h, {vᵢ ↦ (Dᵢ', sᵢ)}, M)   restructuration
           ─────────────────────────────────────────
           τ(n, h) = (n', s)                 [avec M ← M ⊕ μ]
```

`φ_out` est la seule règle qui *écrit* le diagramme : elle peut filtrer des arcs, réétiqueter, **changer de niveau** (l'ordre n'est pas fixé par la structure — c'est ici que l'intrication ordre/opérations s'incarne : une transduction peut réordonner sa sortie), sauter un niveau (ré-émettre un collapse) ou **expandre un niveau implicite** (§ 6.2).

### 3.2 Typage

```
τ : DD⟨Δ_in, Y_in⟩ × H  →  DD⟨Δ_out, Y_out⟩ × S
```

`Δ_out` est déterminé par `τ` et *peut différer de* `Δ_in` (transductions changeant d'ordre ou de codomaine). La correction sémantique est une **obligation de spécification** attachée à `τ` :

> **(Spec-τ)** Pour chaque `τ`, fournir `f_τ : ⟦·⟧ × H → ⟦·⟧` telle que
> `⟦π₁(τ(D, h))⟧ = f_τ(⟦D⟧, h)` — la transduction implémente sa fonction sémantique.

La preuve de `(Spec-τ)` se fait par induction sur les règles `T-TERM`/`T-NODE`, cas par cas. C'est le *format de tous les théorèmes de correction du cadre* : chaque instance d'opération apporte sa fonction sémantique et son induction.

### 3.3 Admissibilité (compatibilité avec la canonicité)

Une transduction est **admissible** si :

- **(A1-défaut)** `φ_disp` traite les arcs absents via `δ(ℓ)` : aucune information implicite n'est ignorée sans être reconstruite ;
- **(A2-collapse)** si tous les arcs d'un nœud d'entrée produisent le même `(D', s)`, alors `φ_out` ré-émet un nœud collapsed (le résultat n'est pas plus explicite que nécessaire) ;
- **(A3-canon)** `φ_out` n'émet que des nœuds internés (le résultat est canonique par construction, pas par post-traitement).

> **Lemme L1 (mémoïsation).** Sous (A1)–(A3), `τ(n, h)` est une fonction pure de `(id(n), h, version(M))`. *Preuve* : les règles ne lisent que le sous-diagramme de `n` et la mémoire ; la version de `M` falsifie les entrées antérieures. **Conséquence** : computed table à clé `(id(n), h)` — la généralisation du `(opérande, opérande)` de Bryant et du `(h, D)` des homomorphismes DDD.

> **Lemme L2 (commutation).** Si `D ≡ D'` et `τ` admissible, alors `τ(D, h) ≡ τ(D', h)`. *Preuve* : induction sur la dérivation de `≡` ; cas critiques : collapse (par (A2)) et défaut (par (A1)). **Conséquence** : les opérations peuvent travailler sur n'importe quelle forme physiquement équivalente — le plan de représentation est libre.

---

## 4. Première famille : transductions héritées seules

`map`, `filter`, réétiquetages : `S` trivial, mémoire inutilisée.

**Exemple — `filter p` sur `Set(Map(K,V))`** avec `p(m) = (m(a) = 1 ∨ ¬présent(a))` :

```
φ_disp(k_a?, h, présent)  = h_cherche_valeur_a          (descendre avec intention)
φ_disp(val(a), h, v)      = v = 1 ? garde : tue         (le verdict est résolu ici)
φ_out : réécrit les arcs selon les verdicts remontés
```

Note : le verdict est résolu *au niveau de la valeur*, mais appliqué *au niveau de la présence* — sans canal remontant, l'implémentation DDD classique doit soit énumérer les chemins, soit réifier le verdict dans une variable. Ici `φ_aggr` transporte le verdict vers le haut en une ligne. *(Spec-τ)* : `f_filter = image réciproque de p` — induction directe.

---

## 5. Deuxième famille : synthétisés seuls (semi-anneaux)

`count`, `exists`, `fold` : `H` trivial, le terminal produit la base, `φ_aggr` la structure du semi-anneau.

```
count  :  S = (ℕ, +, ×)        σ_y(⊤, h) = 1        φ_aggr = Σ des arcs (× par les sous-totaux)
exists :  S = (Bool, ∨, ∧)     σ_y(⊤, h) = vrai    φ_aggr = ∨ des arcs
```

**Comptage sur l'exemple** : les chemins *explicités* de `S` canonique sont au nombre de 1 — et pourtant `|S| = 2`. La règle :

> **(A4-expansion-comptage)** pour un niveau collapsed `ν(ℓ, {v ↦ D | v ∈ Val(ℓ)})`, le poids de l'arc implicite est `|Val(ℓ)| × poids(D)` — la cardinalité du niveau entre dans l'agrégation.

`count(S) = |{0,1}| × 1 (k_a présent) × 1 × 1 = 2`. ✔ **Le collapse force le semi-anneau à connaître les cardinaux des niveaux** — c'est la première apparition du principe d'explicitation (§ 6.2), côté agrégation : *l'information implicite doit être comptée, pas ignorée*. `(Spec-τ)` : `f_count = cardinalité de l'ensemble dénoté` — la preuve exige (A4).

### 5.1 Où vit le «+» : trois rôles, deux algèbres

Le symbole ⊕ apparaît à trois endroits qui ne vivent pas au même niveau :

| Rôle | Site | Déclaration |
|---|---|---|
| (a) fusion à la construction | le ⊕ de `saturate` (§ 7) : combiner deux diagrammes en écriture | **implicite aujourd'hui** : à faire de l'instance (§ 7 amendé) |
| (b) agrégation du pli | `φ_aggr` : combiner les fils d'un nœud à la remontée | semi-anneau de la transduction (ce § 5) |
| (c) dénotation ponctuelle | ce que le diagramme dénote quand `Y ≠ Bool` | le typage `DD⟨X,Y⟩` (MTBDD/ADD, table des instances de la proposition) |

> **(A5-deux-algèbres)** Le semi-anneau de **construction** (le merge des écrits) et le semi-anneau de **requête** (paramètre de la transduction) sont deux déclarations distinctes, toutes deux de l'instance. Aujourd'hui la construction est `Bool` idempotent, et **le hash-consing est ce choix** : l'union idempotente détruit la multiplicité à l'écriture. Un pli est **exact ssi sa lecture factorise par ce que la construction a préservé** : `count` cardinal est exact sur diagramme canonique déterministe (interprétation-fonction, § 2.2 : un chemin par affectation) ; tout pli pondéré (probabilités, multiplicités de chemins) est irrécupérable sur une construction Bool et exige une **construction levée** (arcs porteurs de poids, fusion ponctuelle ⊕ à la ADD), où l'additivité est restaurée par la distributivité au prix de l'idempotence (plus de collapse des doublons).

> **(L3-mémo)** La mémo (opération, nœud) est saine ⟺ l'opération est homomorphe en cet argument. Conséquences : les opérations sémantiquement additives (union, image par relation, garde = restriction) se mémoïsent par nœud ; **un pli est terminal, jamais incrémental, sauf si ⊕ est idempotent** : les ∃-summaries (`exists`, verdicts) se maintiennent en cours de saturation, les cardinalités ploient au point fixe. Les axiomes de semi-anneau sont exactement le permis de `f(d₁ ⊕ d₂) = f(d₁) ⊕ f(d₂)` : `count` est homomorphe pour la structure (un pli mémoïsable, cf. (A4)) et non additif pour la dénotation (∪ idempotent) ; la correction demande de nommer les deux algèbres, pas de trancher entre elles.

### 5.2 Chaque réduction a un dual d'évaluation

> **(A6-dualité)** Chaque réduction (la syntaxe oublie) a un dual d'évaluation (la lecture re-explicite, et paie). **Le coût d'un pli = le coût de re-expliciter ce que les réductions ont absorbé :**

| Réduction (oubli syntaxique) | Dual d'évaluation (re-explicitation payée) |
|---|---|
| R-share | mémoïsation du pli par (forme, hérité) : le partage paie le pli une fois |
| R-collapse | (A4) : le cardinal `|Val(ℓ)|` entre dans l'agrégation |
| R-default / défaut porteur | le pli ajoute le poids des continuations δ implicites (l'arc absent continue le chemin : sa contribution se compte) |
| D-abs (course de défauts absorbée en un arc) | la récurrence du pli sur l'hérité (la longueur re-dépliée) : matrices, exponentiation |

Prédiction falsifiable : la régularité de l'absorbé achète l'exponentiation rapide (le pli des philosophes : matrice [[2,1],[2,0]], O(log n) au lieu de O(n)) ; l'imbriqué irrégulier paie l'explicitation au dépliage complet, Θ(n) ou pire. Contre-épreuve mesurée : le philosophe gaucher (canvas exemple, § 9.4), même matrice enrichie, ratio |R'_n| / |R_n| = 1,2887 constant : la classe irrégulière décale la constante, pas l'exposant. C'est le raccord au comptage par handles : **le pli des philosophes est l'explicitation de D-abs** (canvas exemple, § 7).

---

## 6. L'opération-test : `lookup`

### 6.1 Spécification

```
lookup(k) : DD⟨Δ_map, Bool⟩ → DD⟨Δ_map, Option(V)⟩
⟦lookup(k)(D)⟧ = { ([m], m(k)) | ([m], ⊤) ∈ ⟦D⟧ }
```

Le résultat est un DD *fonctionnel* : chaque map de l'ensemble reçoit en terminal la valeur de la clé `k`. Sur l'exemple : `m₁ ↦ Some(1)`, `m₂ ↦ Some(0)`.

### 6.2 La difficulté centrale : les niveaux implicites

Sur `S` **canonique**, le niveau `val(a)` a disparu (collapse) — or le résultat n'est *pas* constant en `a`. Une transduction naïvement récursive lit `ν(k_a?, …)` puis trouve `k_b?` sans jamais voir la valeur de `a`. **Conclusion structurelle** :

> **Principe d'explicitation.** Toute opération non constante sur un axe doit explicitier les niveaux implicites de cet axe ; c'est possible **ssi l'univers de l'axe est énumérable** (schéma borné). L'explicitation est une règle de `φ_out` :
>
> ```
> (T-EXPAND)  n' absent de l'axe cherché ⇒ n' = ν(val(a), { v ↦ D_v | v ∈ Val(val(a)) })
>              avec D_v = copie du sous-diagramme (l'axe redevient explicite)
> ```

C'est **le théorème de correspondance schéma-transduction** :

> **T5 (correspondance).** Schéma borné ⟺ toute opération du niveau 1 se compile en transduction (avec expansion) ⟺ canonicité utilisable. Schéma ouvert ⟹ l'expansion est impossible ⟹ la décomposition doit énumérer les éléments présents (style DDD) ⟹ non-canonicité assumée. *La classification de canonicité du § 2 de la proposition n'est pas une propriété de représentation : elle est forcée par le calcul des opérations.*

### 6.3 La définition de `lookup(a)`

Deux modes hérités : `cherche` (au-dessus de l'axe), `extrait` (dans l'axe). Attribut synthétisé : **un fragment de DD** sur l'axe — les attributs synthétisés sont de *second ordre* (voir § 8).

```
φ_disp(k_a?, cherche, présent)  = extrait
φ_disp(k_b?, cherche, ·)        = cherche        (les autres clés : transit)
φ_disp(val(a), extrait, v)      = stop           le mode devient terminal

(T-TERM en mode stop)     σ_y(y, stop)      = ε(Some(v))     fragmentDD = payload
(T-TERM en mode extrait)  σ_y(y, extrait)   = —              (inaccessible : val(a) précède)
(T-TERM en mode cherche)  σ_y(y, cherche)    = ∅              pas encore trouvé

φ_aggr(ℓ, cherche, {vᵢ ↦ sᵢ}) = union des fragments (fusion canonique des DD fragments)
φ_aggr(val(a), extrait, {v ↦ s}) = ν(val(a), {v ↦ s})      re-racine le fragment

φ_out(k_a?, cherche, {présent ↦ (D', s)}, M) :
      D''  = greffe : les terminaux de D' reçoivent le payload du fragment s
             (le fragment est « appliqué » sur chaque continuation)
      n'   = ν(k_a?, { présent ↦ D'' })
```

La **greffe** (`φ_out` du niveau cherché) est l'opération qui n'existait dans aucun formalisme antérieur : le fragment synthétisé — récolté en bas de l'axe — est *distribué sur les terminaux du reste du diagramme*. C'est exactement « porter l'information inter-niveaux » : ni hérité pur (le fragment ne descend pas), ni synthétisé pur (il ne remonte pas assez), mais **les deux, coordonnés par le nœud de l'axe**.

### 6.4 Dérivation sur l'exemple

```
Entrée (canonique) :  k_a? ─ présent ↘ [val(a) implicite]  k_b? ─ présent ↘ val(b)=0 ↘ ε(⊤)

1. τ(k_a?-node, cherche) : dispatch présent ↦ extrait ; autre défaut (absent ↦ ∅)
2. sous l'arc présent : niveau suivant = k_b?  ⟹ niveau val(a) IMPLICITE
   (T-EXPAND)  :  ν(val(a), { 0 ↦ Q, 1 ↦ Q })   avec Q = copie du k_b?-sous-DD
3. τ(Q, cherche) sur chaque branche : transit k_b?, count-like :
   σ remonte le fragment ∅ (pas de a dans la branche)… non — la branche EST sous val(a) :
   en mode « extrait », la base est posée par (T-TERM en mode stop) — or on n'atteint pas
   encore val(a). D'où la correction : le mode extrait s'applique au NIVEAU val(a) lui-même ;
   ses arcs posent σ = ε(Some(v)) ; le greffon est donc ν(val(a), {0 ↦ G, 1 ↦ G'}) où
   G, G' transportent Some(0), Some(1).
4. φ_aggr(k_a?) remonte le greffon ; φ_out greffe :
   k_a? ─ présent ↘ ν(val(a), { 0 ↦ [k_b?… ε(Some(0))], 1 ↦ [k_b?… ε(Some(1))] })
```

Résultat : fonction `m₁ ↦ Some(1)`, `m₂ ↦ Some(0)`. ✔ Le résultat n'est pas collapsable en `val(a)` (les deux arcs mènent à des sous-DD *différents*) : **l'opération a explicité l'axe** — exactement ce que la sémantique ordonnait, et `R-collapse` ne s'applique plus. La dérivation et la réduction sont *d'accord* : c'est le signe que le calcul est correct.

---

## 7. Troisième famille : `saturate` (mémoire de niveau)

```
saturate(τ, D)  =  νX. D ⊕ τ(X, h₀)        X = l'ensemble-courant ; ⊕ = merge de construction,
                                           déclaré par l'instance (A5) : union canonique
                                           idempotente (Bool) aujourd'hui ; en lecture pondérée,
                                           ⊕ pointwise à la ADD : additivité par distributivité,
                                           perte de l'idempotence
```

`τ` est ici une transduction d'étape (l'image d'un pas du système). La **mémoire de niveau** `M` vit entre les itérations (clé : `(id(nœud), ℓ)`) : c'est elle qui porte l'état intermédiaire des **événements non locaux**.

**Exemple — transfert** `t : m(a) -= 1; m(b) += 1` (touche `k_a` et `k_b`, niveaux non adjacents) :

```
φ_disp(k_a?, h_t, présent)     = h_odo(valeur-a)      descend lire la valeur de a
φ_disp(val(a), h_odo(v), v)     = v ≥ 1 ? h_transfert(v−1) : stop
     — le reste de la descente vers k_b? se fait en mode h_transfert :
φ_disp(k_b?, h_transfert(w), ·) = h_pose(w+1)         écrit b
     — et la mémoire de niveau enregistre, au nœud k_a? émetteur :
M[id(n), k_a?] ← « décrément déjà produit par ce nœud à l'itération courante »
```

La mémoire de niveau joue le rôle de **jeton between levels** : l'événement est non local, mais chaque *segment* (lire `a`, écrire `b`) est local ; `M` et les attributs hérités cousent les segments. C'est la formalisation exacte de « les homomorphismes manquent d'un stockage inter-niveaux » — et la **preuve constructive que la réification de variables intermédiaires (Zhao/Ciardo) est remplaçable**.

> **T2 (terminaison, instance bornée).** Sur schéma borné, l'ensemble des fonctions `Δ → Y` est fini, donc le nombre de formes canoniques est fini ; `saturate` construit une chaîne croissante pour l'ordre d'inclusion des dénotations ⟹ terminaison, et le nombre d'itérations est au plus le nombre de formes canoniques intermédiaires (en pratique : très inférieur, cf. saturation de Ciardo).
>
> **T3 (expressivité).** Toute transition non locale de largeur `w` (touchant `w` niveaux) se compile en transduction à mémoire de niveau avec un surcoût polynomial en `w`. *À prouver* — c'est le jalon 2, l'énoncé est maintenant précis.

---

## 8. Attributs de second ordre : la décision de conception principale

La formalisation de `lookup` a *forcé* une décision que la proposition initiale ne prévoyait pas explicitement : l'attribut synthétisé de `lookup` est un **fragment de DD**, pas une valeur scalaire.

> **Décision D1.** L'algèbre des attributs synthétisés est *fermée par construction de fragments* : `S` peut contenir des `DD⟨Δ', Y'⟩` (attributs de second ordre). L'agrégation `φ_aggr` devient une union canonique de fragments ; la greffe `φ_out` en est la codage dual.

Conséquences (à assumer, pas à cacher) :

- **Coût** : les fragments sont hash-consés dans le même pool que le diagramme principal → le coût mémoire des attributs est borné par celui des nœuds existants + les fragments d'axe ; les computed tables de fragments se versionnent comme L1.
- **Gain** : `fold` ne se distingue plus de `lookup` que par le semi-anneau du synthétisé (`(ℕ,+,×)` vs fragments) — la table d'opérations du § 1.4 de la proposition se ramène à *une* règle `T-NODE` et trois choix d'algèbre. C'est l'unification des opérations, réalisée.

---

## 9. Table des énoncés

| # | Énoncé | Statut |
|---|---|---|
| Spec-τ | Chaque transduction porte sa fonction sémantique + induction | Format de preuve, à remplir par opération |
| L1 | Mémoïsation : pureté en `(id, h, version M)` | Démontré (structurel) |
| L2 | Commutation avec les quotients (admissibilité A1–A3) | Démontré (structurel) |
| A4 | Cardinaux dans l'agrégation (count sur collapse) | Démontré sur l'exemple ; à intégrer à (A1)–(A3) comme condition |
| **A5** | Deux algèbres : le merge de construction (le hash-consing **est** le choix Bool) et le semi-anneau de requête (paramètre de la transduction) sont deux déclarations ; pli exact ssi sa lecture factorise par ce que la construction a préservé | Énoncé à prouver (format (Spec-τ)) ; ouvre l'instance ADD/pondérée |
| **L3** | Mémo (opération, nœud) saine ⟺ homomorphisme en cet argument ; pli incrémental ⟺ ⊕ idempotent (∃-summaries oui, cardinalités non) | Lemme structurel à démontrer (extension de L1/L2) |
| **A6** | Dualité réduction↔explicitation : le coût d'un pli = le coût de re-expliciter ce que les réductions ont absorbé | Prédiction falsifiable ; instance philosophes (matrice, O(log n)), contre-épreuve gaucher (ratio constant) |
| T-EXPAND / principe d'explicitation | Opération non constante sur un axe ⇒ explicitation de l'axe, possible ssi borné | Démontré sur `lookup` ; à généraliser |
| **T5** | Correspondance schéma-transduction : borné ⟺ transductions complètes + canonicité ; ouvert ⟹ énumératif + non canonique | **Cible principale** — la classification de canonicité dérivée du calcul des opérations |
| Compilation typée | Toute opération définie sur Σ (indépendante de l'ordre) se compile en transduction ordonnée pour toute instance (O, R, encodage) ; la sémantique est préservée par les conversions | À prouver ; s'étend de T5 (le schéma force la compilation disponible, le type en fixe la sémantique) |
| T2 | Terminaison de `saturate` sur schéma borné | Démontré (finitude) |
| T3 | Expressivité événements non locaux, surcoût poly(|largeur|) | Cible jalon 2, énoncé précisé |
| D-abs / D-trans | Défaut généralisé : absorption sur tout sous-domaine, `D^déf` désigné par contrôleur | Extension jalons 4–5 (§ 2.4) ; (R-collapse)/(R-default) = cas particuliers |
| C1 | Confluence {D-abs, D-trans, R-share}, puis minimalité par handle (contrôleur, ancre, forme) | À prouver ; étend le format T5 |
| Partage de travail | Coût O(1) pour le cœur de N réplicas à événements invariants (+ terme de bord) | Falsifiable ; benchmark n philosophes, ligne de base SDD hiérarchique |

**Ce que la formalisation a produit de neuf par elle-même** : (1) le principe d'explicitation et le théorème T5 — la canonicité n'est pas une propriété de la représentation mais une conséquence du calcul des opérations ; (2) la greffe comme primitive manquante ; (3) les attributs de second ordre (D1) et la réduction de la table des opérations à une règle + trois algèbres ; (4) les deux algèbres du «+» (A5, L3) : construction et requête sont des déclarations distinctes, et le hash-consing idempotent est un choix de semi-anneau, pas une évidence ; (5) la dualité réduction↔explicitation (A6) : chaque quotient paie son explicitation, et le pli des philosophes est celui de D-abs. Ces cinq points sont *sortis de la spécification* elle-même : c'est le signe que le formalisme a de la force.

---

## 10. Questions ouvertes et décisions d'implémentation

1. **Expansion et ordre physique** : `T-EXPAND` insère des niveaux explicités — dans quel ordre physique ? L'expansion doit consulter `Order<T>` (le niveau réinséré prend sa place déclarée). C'est le point de jonction exact entre ce calcul et le plan de représentation.
2. **Versioning de la mémoire** : L1 exige `version(M)` ; granularité (par nœud ? par niveau ? par itération de saturation ?) — décision d'implémentation à mesurer (elle détermine le taux de réussite du computed table).
3. **Greffe sur défauts** : lorsque le fragment transporte `None` (clé absente), le greffon est constant sur les continuations → l'arc absent reste collapsable (le `Option` de `lookup` est *l'algèbre de défauts du résultat*). À intégrer à l'admissibilité.
4. **`Set` vs `Map`** : la décomposition présence + valeur couvre `Map` ; `Set(T)` = `Map(K, Unit)` — vérifier que les règles couvrent les deux sans duplication (attendu : oui, l'instance `Set` n'utilise que les niveaux présence).
5. **Généralisation `Seq`/`Tag`** (jalon 2) : `Tag` = union (niveau à un arc utile, déjà couvert) ; `Seq` = le constructeur *list* (répétition + marqueur de fin) : deux compilations, par positions ou par énumération d'éléments — c'est là que T5 devra se durcir (le schéma « énumération » est borné par la longueur, pas par l'univers). À la clé : la correction de la compilation des opérations typées (indépendantes de l'ordre) vers les transductions ordonnées, pour toute instance.
6. **Encodage des variables (absolu/relatif)** : précédence par contrôleur ; le nœud absolu resynchronise l'accumulation. Le hash-consing doit normaliser l'encodage : sérialisation canonique avec bit d'encodage, tie-break structurel par fingerprint (jamais par id de création). Invariants de bench : lecture absolue = accumulation relative ; conversions abs ↔ rel involutives modulo réduction.
7. **Désignation du défaut par contrôleur** : `D^déf` n'est pas une propriété du niveau ; deux contrôleurs coexistent au même niveau avec deux réductions différentes. Conversions entre royaumes = transductions à coût prédictible (le contrat typé s'étend des ordres aux encodages) ; l'oracle décide dans quel royaume calculer. Les clés du computed-cache portent l'empreinte du contrôleur ; l'index niveau → formes vit dans le plan d'ombre (la saturation itère sur les usages, pas sur les formes).
8. **Merge de construction non idempotent** : (A5) ouvre la lecture pondérée (arcs porteurs de poids, ⊕ pointwise à la ADD), mais elle exige une table unique indexée par les poids et perd le collapse des doublons : à instancier au jalon ADD/MTBDD, avec le différentiel de compatibilité en garde-fou (un pli pondéré n'est exact que sur une construction levée ; sur une construction Bool, seul le cardinal est exact).
9. **δ continu : régions partout, intégrale au pli** : le continu s'installe par l'algèbre des clés (une clé = une description finie du langage Σ, région ou singleton dégénéré licite dès que l'égalité de ses points est décidable ; le domaine est continu, l'espace des clés ne l'est jamais, au plus dénombrable, et l'interdit porte sur une clé par point du domaine, pas sur le point isolé). Deux points résistent. (i) C1 devient relatif à l'algèbre de régions : DBM convexes (clôture canonique) oui, unions non convexes sans forme normale connue, semi-algébrique décidable (Tarski) mais doublement exponentiel. (ii) Le pli sur une dimension continue est une **intégrale**, pas une somme : le semi-anneau porte des mesures (densités), et le pli exact (A5) exige l'intégration symbolique sur le fragment ; candidat : régions polyédriques + densités polynomiales ; les clés-ponctuelles n'y comptent que sous un semi-anneau à atomes (Dirac, comptage), sinon mesure nulle. Instance-test : DDD-différence ; l'ordre O s'entend sur le treillis des régions, et le balayage d'univers ne compile que si l'univers est borné à régions énumérables, sinon royaume libre DDD assumé.