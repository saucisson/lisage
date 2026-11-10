# Proposition : Lisage — le cadre attribué des diagrammes de décision

*Synthèse s'appuyant sur l'état de l'art (canvas dédié) et sur les notes de conception exploratoires du projet, analysées dans les deux canvases « Analyse critique ». Niveau d'ambition assumé : un cadre unificateur, pas une bibliothèque de plus.*

---

## 0. Le propos en une phrase

> **Un diagramme de décision est l'implémentation d'une énumération paresseuse d'un ensemble de valeurs structurées ; les opérations sont des transductions attribuées sur cette énumération ; l'ordre n'est ni de la donnée ni de l'implémentation, mais le *contrat* qui lie les deux — et le compilateur est l'endroit où ce contrat se négocie.**

Trois conséquences définissent le cadre :

1. **Attribution** : la pièce manquante des homomorphismes DDD est un canal d'information bidirectionnel (hérité/synthétisé) + une mémoire de niveau. C'est la fondation opérationnelle, pas une extension.
2. **Typage** : un DD est typé `DD⟨X, Y⟩` (domaine de décision, codomaine) ; l'ensemble, la relation, la fonction et la partition sont des instances du même objet.
3. **L'ordre est une ressource compilée** : canonicité relative explicite ; reordering dégradé au rang de conversion à coût prédictible ; le plan de représentation se raisonne sur des chaînes d'opérations, pas sur des diagrammes isolés.

La formule qui positionne Lisage par rapport au programme de généralisation/unification :

> **Lisage intégré = Contrôleur polyDD (structures, paramétrables) × Attribution (opérations, unifiées) × Ordre typé (contrat).**
>
> Le cadre unifie les **opérations** complètement (c'est l'aboutissement de la thèse de Linard (2009) et de *Specification of Decision Diagram Operations* (Linard, Hamez et al., 2010)) ; côté **structures**, il reste général dans la mesure exacte où le niveau 2 *est* le contrôleur paramétrable. L'algèbre `seq/set/map/tag` n'en est que l'**instance de référence** — le terrain de validation complet, jamais un cadre fermé (démonstration § 1.2).

---

## 1. L'objet central : le diagramme attribué

### 1.1 Le foncteur de types (instance de référence)

Partir de l'algèbre de types polynomiale :

```
T ::= Atom(δ)            // δ = schéma d'atomes (borné / pointé / ouvert / structuré)
    | Seq(T) | Set(T) | Map(T,T) | Tag(l → T)
    | T → T              // flèche résolue par le niveau 1
```

`Seq/Set/Map/Tag` sont les *foncteurs* standard ; c'est la signature structurelle de Genève (ΣDD, MSDD) assumée comme algèbre de base — tout le reste (atomes, cardinalité, défauts) est paramétré par un **schéma**. Attention : cette algèbre est **l'instance de référence du contrôleur**, pas le cadre lui-même — tout ce qui suit vaut pour n'importe quelle instance du contrôleur étendu (§ 1.2), et `seq/set/map/tag` sert de terrain de validation complet.

### 1.2 Le contrôleur étendu : l'objet général

Le niveau 2 de Lisage **est** le contrôleur de polyDD (Linard 2009), étendu de trois paramètres :

```
Contrôleur étendu C = ( schéma d'atomes δ       // borné / pointé / ouvert / structuré (DBM, Presburger…)
                     , algèbre de types Σ       // constructeurs de domaines = grammaire des variables
                     //   (record, union, list, set, map ; référence : seq/set/map/tag)
                     , décomposition + ordre O  // contraintes, pas séquences
                     , règles de réduction R   // quotients
                     , algèbre de défauts       // déduite du type, surchargeable
                     , diagramme de défaut D^déf // ce que l'arc absent dénote ; désigné par contrôleur (D-abs, spec § 2.4)
                     , encodage des variables   // absolue au nœud / relative sur l'arc (shift) ; précédence par contrôleur
                     , interface d'attribution // canaux autorisés (hérité/synthétisé/mémoire)
                                              // + semi-anneau PAR TRANSDUCTION : il vit dans la
                                              //   signature de l'opération (une même transduction,
                                              //   des semi-anneaux) ; le contrôleur fixe les
                                              //   compatibilités
                                              // + merge de construction : le ⊕ des écrits,
                                              //   déclaré par l'instance (A5 de la spec) :
                                              //   Bool idempotent aujourd'hui, et le hash-consing
                                              //   EST ce choix ; ADD/pondéré en jalon
                     )
```

Toute catégorie de DD de l'état de l'art devient une instance de `C` :

| Catégorie | Paramètres saillants de l'instance |
|---|---|
| BDD (Bryant) | Σ booléen, δ = faux, réduction classique, attribution héritée seule |
| ZDD (Minato) | même Σ, **δ = absent** — l'algèbre de défauts suffit à distinguer les deux |
| MTBDD / ADD | codomaine `Y` riche (le typage `DD⟨X,Y⟩` suffit) |
| EVBDD | arêtes affines = valuation dans le contrôleur |
| MDD (Miner/Ciardo) | atomes énumérés, ordre statique par niveaux, saturation = policy statique |
| DDD (Couvreur) | atomes ouverts, non-canonicité assumée, homomorphismes = transductions à hérité seul ; sa répétition de variables = le dépliage des constructeurs list/set/map (ci-dessous) |
| SDD/HSDD (Hamez) | ensembles en valuation, hiérarchie = décomposition récursive, saturation = policy auto-détectée |
| ΣDD / MSDD (Genève) | algèbre de termes / multi-ensembles — prédécesseurs directs de l'instance de référence |
| ESDD (Devereux/Chechik 2003) | variable relative sur l'arc (shift), partage de structure par inclusion, canonicité par conditions suffisantes ; précurseur direct, pré-saturation, jamais adopté |
| CESRBDD/RexBDD (Ciardo/Miner) | transformateurs unaires d'arête : réduction désignée, complément, swap |
| SDD sententiels (Darwiche) | vtree = contrainte de décomposition ; canonicité relative = la nôtre |

Le bestiaire complet (une cinquantaine de familles vérifiées, instanciées champ par champ dans la grille ci-dessus, novembre 2026) est dans le canvas « État de l'art », § 9 : chaque famille y est replacée dans le paramètre-space de `C`, et les lignes ci-dessus en sont les pôles. Les convergences récentes (lignée ESRBDD/CESRBDD/RexBDD 2017-2024, SLDD 2005, LIMDD 2022) y sont documentées source en main.

**Σ est la grammaire des variables (la répétition des DDD, domptée).** Un DD n'a pas de séquence de variables : ses niveaux sont le dépliage d'une expression de Σ. Cinq constructeurs fermés : **record** (produit de champs nommés, ordre des champs fixé par le contrôleur : l'intercalage de l'exemple fil rouge est un choix de record), **union** (le discriminant, cas Tag), **list** (répétition d'une variable + valeur de fin), **set** (répétition dans l'ordre canonique des éléments + marqueur de fin), **map** (répétition de clés + marqueur de fin, la valeur portée par l'arc : la valuation d'arêtes de polyDD promue constructeur, contribution n° 8). La répétition de variables des DDD, jusqu'ici parquée comme « non-canonicité assumée », devient un constructeur typé : sa canonicité est une propriété de la compilation choisie, pas une anomalie du schéma. Le marqueur de fin est une **clé requise** (l'injection « 1 » de μX.(V×X)+1), jamais un défaut absorbable ; il coopère avec D^déf sans s'y confondre : le marqueur clôt les énumérations, D-abs fournit la sparsité des balayages. Conséquence immédiate : un même constructeur a plusieurs compilations. `Set(T)` borné se compile en balayage de présence (un niveau par élément, δ = absent, sans marqueur, creux à la ZDD) ou en énumération DDD (membres seulement + marqueur) ; même constructeur, deux compilations, l'oracle tranche au profil des données : le premier arbitrage mesurable du planner.

Et la conséquence architecturale : **les opérations sont définies sur les types, indépendantes de l'ordre** (insérer, membre, union, lookup, fold : la sémantique du constructeur), puis **traduites par le contrôleur en transductions ordonnées** sur sa (décomposition, O, encodage). Nouvelle obligation de preuve : la compilation est totale et correcte pour toute instance, et préserve la sémantique à travers les conversions. C'est la couche sémantique qui manquait au contrat d'ordre : l'opération est écrite une fois, l'ordre négocié ensuite, le planner convertit sans réécrire.

**Exemple fil rouge** : l'instance C_philo (n philosophes en anneau) est déroulée de bout en bout, du contrôleur au comptage, dans le canvas « Exemple : n philosophes » : l'espace atteignable, en ≈ (1+√3)^n, tient en 7 nœuds indépendants de n, le paramètre n vivant sur le shift de l'arc d'entrée (chiffres vérifiés par simulation exhaustive).

**Test de complétude du cadre** : les SDD sententiels doivent s'installer comme instance sans extension *ad hoc* — c'est la branche manquante de l'état de l'art (la compilation de connaissances), et le contrôleur étendu lui tend naturellement la place. Deux bonus structurels du paramétrage :

1. **Attributs sur semi-anneau** : le canal synthétisé vit dans un semi-anneau **porté par la signature de la transduction** (le contrôleur fixe les compatibilités) : booléen pour `exists/forall`, `(ℕ, +, ×)` pour le comptage, probabiliste pour l'évaluation. Comptage, model counting pondéré et PSDD entrent ainsi dans la même unification que la vérification : *une même transduction, des semi-anneaux différents*. Et la déclaration jumelle, invisible tant qu'on ne pose pas la question : le **merge de construction** (le ⊕ des écrits). Le hash-consing idempotent **est** le choix Bool ; une lecture pondérée exige une construction levée (arcs porteurs de poids, ⊕ pointwise à la ADD), où l'additivité est restaurée par la distributivité au prix de l'idempotence. Deux algèbres : l'une écrit, l'autre lit, et un pli n'est exact que s'il factorise par ce que la construction a préservé (A5, L3 de la spec).
2. **Schémas d'atomes structurés** : δ peut couvrir des atomes non bornés *structurés* (DBM pour le temps, Presburger pour les entiers) — la branche temporelle/numérique de l'état de l'art (CDD, difference DD, NDD) s'intègre comme simple choix de δ. Et le continu entre par la même porte : une clé est toujours une **description finie du langage Σ** (intervalle, contrainte, polyèdre ; le singleton {c} en est la région dégénérée, licite dès que l'égalité de ses points est décidable : rationnels, algébriques). Le domaine peut être continu, l'espace des clés ne l'est jamais (au plus dénombrable : une clé est un mot de Σ, pas un point du domaine). L'interdit ne porte pas sur le point isolé mais sur deux débordements : une clé *par point* du domaine (les fils d'un nœud sont en nombre fini, une partition ponctuelle n'est jamais exhaustive, tout le reste part dans D^déf) et le point à égalité indécidable (réel calculable arbitraire : pas de forme normale, donc pas de hash-consing). Le singleton licite reste dégénéré : couverture nulle, annihilé par le pli en semi-anneau de mesures, sauf semi-anneau à atomes (Dirac, comptage). Concrètement, δ est un contrat d'**algèbre de clés** (forme normale, égalité, couverture pour R-collapse et D^déf, intersection pour le produit, énumération des régions d'un univers borné), porté par le contrôleur, jamais par le pool : les « types de nœuds » (discret, intervalle, contrainte, continu) sont des instances de δ côté algebra/, le nœud bas-niveau reste unique et non typé.

**Un non-mécanisme à noter : la tour de contrôleurs.** On peut en théorie empiler plusieurs niveaux de contrôleurs (bas-niveau vers haut-niveau) avec des transformations inter-niveaux, définissant une partie du partage à la granularité des contrôleurs. Verdict : intéressant comme *motif d'instance*, pas comme mécanisme ; presque tout y existe déjà. Un ordre structuré en arbre est déjà un paramètre (le vtree des SDD en est un cas) ; un contrôleur local par sous-ordre est une instance du contrôleur avec une ancre locale (le triple (contrôleur, ancre, forme) s'applique tel quel) ; les transformations inter-niveaux = des transductions ; le partage par niveau = le hash-consing des handles (forme, ancre locale). Ce que la tour achète et que le plat+shift ne peut pas : le partage *imbriqué irrégulier* (répétition non uniforme ; le régulier est déjà couvert par l'unfold, spec § 2.4.4). La canonicité est modulaire : C1 par niveau, composé si les niveaux s'interface par handles canoniques (le format des SDD normalisés). La tour est la voie d'installation des SDD sententiels (vtree = ordre structuré, partage par élément, saturation par niveau = la « saturation auto-détectée » de la lignée HSDD), à vérifier au jalon 5. Le pool de bas niveau reste unique, plat et non typé : la tour vit dans la couche algèbre. Critère de décision (oracle) : répétition régulière → plat+shift suffit ; répétition imbriquée irrégulière → tour.

### 1.3 Le diagramme comme coalgèbre

Un `DD⟨X, Y⟩` est un graphe de décision sur la décomposition de `X` dont les feuilles sont dans `Y`. Le point de vue fécond : c'est une **coalgebra finie** sur le foncteur polynomial induit par la décomposition. Trois dividendes immédiats :

- **la canonicité = minimisation de coalgèbre** (le théorème à écrire : classification des constructeurs par statut canonique — voir § 2) ;
- les réductions (collapse, default-suppress, sharing) deviennent des *quotients* de la coalgèbre, unifiées sous une seule notion au lieu d'un bestiaire de règles ;
- `Map`/`Set` non bornés héritent proprement du statut DDD : non-canonicité assumée, égalité syntaxique, homomorphismes.

### 1.4 Les trois flux d'information (le cœur de la proposition)

Une opération Lisage est une **transduction attribuée** : à chaque application sur un nœud,

```
⟨hérité_h, mémoire_niveau⟩ ↓ nœud n ↓ ⟨synthétisé_s, effets⟩
```

- **Hérité** (descendant) : le paramètre de l'homomorphisme DDD — existe déjà ;
- **Synthétisé** (remontant) : le canal manquant — `fold`, `exists`, `count` en sont les cas purs ; il vit dans un **semi-anneau porté par la signature de la transduction** (booléen, `(ℕ,+,×)`, probabiliste…), le contrôleur fixant les compatibilités — c'est ce qui fait entrer comptage et évaluation pondérée dans la même unification que la quantification ;
- **Mémoire de niveau** : un store par (opération × niveau × nœud), persistant entre les visites — c'est ce qui fait de la saturation un opérateur déclaratif et qui répare le cas des événements non locaux.

**Table d'implémentation des opérations** :

| Opération du niveau 1 | Flux requis | Réalisation Lisage |
|---|---|---|
| `map f` | hérité | transduction descendante (≈ homomorphisme DDD) |
| `filter p` | hérité + synthétisé (le verdict remonte) | transduction descendante à garde remontante |
| `reduce/fold/exists/count` | synthétisé | accumulation remontante, terminal = agrégat |
| `lookup k` (Map) | hérité (la clé) + mémoire (comparaisons) + synthétisé (la valeur) | le cas d'école complet — impossible dans le modèle actuel |
| `saturate step` | les deux + mémoire de niveau | point fixe par niveau, stratégie paramétrée (§ 5) |
| `groupBy/join` | synthétisé + hérité | transduction bidirectionnelle |

C'est la réponse directe au diagnostic de 2010 (*Specification of Decision Diagram Operations* : *les homomorphismes manquent d'un stockage inter-niveaux*) : ils manquaient de deux canaux et d'une mémoire ; les voici, et les « opérations non homomorphiques » (lookup, insert, join, groupBy) deviennent homomorphiques *pour la sémantique attribuée* (elles ne l'étaient que pour la sémantique mono-directionnelle). Et la loi de mémoire qui précise «homomorphique» : une mémo (opération, nœud) est saine ssi l'opération est homomorphe en cet argument ; un pli se maintient en cours de saturation ssi ⊕ est idempotent (les ∃-summaries oui, les cardinalités ploient au point fixe) ; et le semi-anneau de construction (le merge des écrits : le hash-consing est le choix Bool) est une déclaration distincte de celle de la requête : un pli n'est exact que s'il factorise par ce que la construction a préservé (A5, L3 de la spec).

---

## 2. Canonicité : un théorème de classification, pas un slogan

**Énoncé cible.** Pour chaque constructeur et chaque schéma d'atomes :

| Domaine | Statut | Fondement |
|---|---|---|
| Atomes bornés + Seq/Set/Map bornés | **canonique** | minimisation (automate/coalgebra minimal) ; `Set` = fonction caractéristique, `Map` = coalgebra fonctionnelle triée par clés canoniques |
| Atomes pointés (défaut δ, encodage borné) | canonique + **algèbre de défauts** | la suppression de défaut généralise BDD (δ = faux) et ZDD (δ = absent) ; δ est déduit du type, surchargeable par opération |
| Domaines ouverts (Int/String non bornés, Map infini) | **non canonique assumé** | style DDD : égalité par identité syntaxique, opérations attribuées |

Deux corollaires travaillés dès le début du projet (pas « plus tard ») :

1. `Canon(T, D, O, R, δ)` — la forme canonique est relative au quintuple (type, décomposition, ordre, règles, défauts) ; *l'égalité sémantique entre deux plans différents = égalité des décodages*, jamais identité structurelle.
2. Le **fingerprint sémantique** (hash du quintuple + hash de la coalgebra réduite) sert de clé de cache inter-représentations — deux plans physiques d'un même objet partagent leurs résultats d'opérations.

---

## 3. Les ordres : typés, composables, négociés

### 3.1 `Order<T>` comme valeur de première classe

```
Order<T>  ::=  atom(δ, π)              // permutation par type et contexte
            | seq(O, tail)             // position × reste
            | set(O)                   // ordre canonique des éléments
            | map(O_key, O_val)
            | tag(l → O)               // discriminant avant payload
combinateurs : project / interleave / refine / coarsen
conversions  : DD⟨T, Y⟩ @ O₁  →convert→  DD⟨T, Y⟩ @ O₂   // nœud de premier niveau, coût prédictible
```

- Les **permutations π par type** sont intégrées *et contraintes* : π vit dans `Order<T>` et l'**alignement devient un typage** — deux opérandes ne se composent que sous un ordre commun, que le compilateur doit *introduire explicitement* (convert, ou ordre canonique du module, ou spécialisation de l'opération pour la paire (O₁,O₂)).
- Le reordering dynamique n'est pas supprimé mais **déclassé** : fallback mesuré du planner, jamais mécanisme par défaut. Le coût d'une conversion est prédit avant d'être payé (brique Sølvsten & van de Pol : prédiction des besoins mémoire des opérations).

### 3.2 Ce que les notes de conception laissaient ouvert et qu'il faut trancher

- Le **Semantic DD n'est jamais une passerelle obligatoire**. Il est materialisé à la demande : pour les valeurs qu'on sait vouloir sous plusieurs ordres (hot values), sinon on compile directement du niveau 2 vers le plan physique. Raison : la forme sémantique peut être exponentiellement plus grande que le meilleur plan physique.
- **Le planner est bidimensionnel** : statique sur les chaînes d'opérations connues (ce qui couvre le pattern `map → filter → exists`), dynamique sur l'inconnu (fonctions d'ordre supérieur runtime). Le langage pur est un atout : tout est réécrivable/mémorisable/déplaçable.

---

## 4. Le compilateur : planification de représentation

Pipeline (un seul IR, trois classes d'annotations — tranché comme dans l'analyse critique) :

```
Niveau 1 : langage pur typé (HOF, seq/set/map/tag)
              │  normalisation + fusion (le classique)
Niveau 2 : IR attribué  —  le contrôleur étendu (§ 1.2), instancié :
           opérations en transductions typées,
           types décomposés, ordres sémantiques (contraintes, pas séquences)
              │
        PLAN DE REPRÉSENTATION
        (oracle de rentabilité : concret vs DD@O₁ vs DD@O₂ …)
              │
Niveau 3 : IR physique — même langage + nœuds convert/materialize
           + ordre physique + encoding + placement
              │
        cœur : pool Lisage (formes, handles) ; Sylvan/CUDD : témoins
```

**L'oracle de rentabilité est la brique de recherche n° 1** — celle que toutes les approches existantes repoussent. Premier modèle, volontairement grossier mais *prédictif* :

```
profitable(dd, op-chain) ≈  taille_encodage(domain) × densité_attendue
                          <  Σ coût_concret(ops)  −  coût_construction(dd)
                          +  valeur_des_requêtes_aval (quantification = jackpot)
```

alimenté par : profil de la distribution des données, prédiction de nœuds (cuts maximaux), et un retour d'expérience appris (les données MCC 2011–2018 conteneurisées sont exactement le corpus d'entraînement rêvé). L'oracle peut être appris plus tard ; il doit exister en version analytique dès le prototype.

---

## 5. Saturation déclarative

```
saturate : (op : transduction attribuée, D : DD⟨S, Bool⟩) → DD⟨S, Bool⟩
```

- **Instances** : stratégie statique par niveaux (Ciardo) et stratégie auto-détectée (Hamez) = deux *policies* du même opérateur, sélectionnées par le plan.
- **Événements non locaux** : dans Lisage ils s'écrivent nativement — la mémoire de niveau transporte l'état intermédiaire entre les niveaux qu'un événement traverse ; plus besoin de réifier des variables intermédiaires à la Zhao/Ciardo. C'est le théorème d'expressivité à prouver : *toute transition multi-niveaux est une transduction Lisage à mémoire de niveau, avec une explosion au plus polynomiale en la largeur de l'événement*.
- **Portée mémoire** : la mémoire de niveau vit dans le nœud (table d'attributs co-localisée), ce qui fait de la saturation une des premières opérations *naturellement consciente du cache* du domaine.

---

## 6. Mémoire : le store bi-plan (la partie imaginative)

Le conflit fondamental — hash-consing = adressage par **contenu**, localité = adressage par **usage** — se résout en ne choisissant pas :

- **Plan du contenu** : le pool des nœuds, hash-consé, IDs en ordre de création, *jamais déplacé*. C'est là que vivent la canonicité, l'identité, le fingerprint.
- **Plans d'ombre** : par phase d'opérations dominante, une réplique compactée ordonnée par *affinité d'accès observée* (profil de la phase précédente), construite en période de GC, jetable, sans identité sémantique. Le mark-sweep-shift de Microsoft en est l'ancêtre ; la différence est qu'ici le placement est *piloté par les transductions attribuées elles-mêmes* (la mémoire de niveau connaît les voisins d'accès).
- Les attributs (synthétisés persistants, caches de transduction) sont **co-localisés avec les nœuds dans les deux plans** — le même mécanisme sert la sémantique (§ 1.3) et la localité (§ 7 du document sur la localité) : une pierre, deux oiseaux, zéro structure dupliquée.

Contre-intuitif mais défendable : on *accepte* de payer une réplique par phase. Un DD tient rarement dans les caches de toute façon ; une réplique bien ordonnée qui économise 30 % de défauts de cache sur la phase dominante rembourse sa construction.

---

## 7. Le langage (niveau 1) et la promesse utilisateur

Rien de nouveau sous le soleil — c'est voulu. L'utilisateur écrit :

```
type State = { pc : Int, stack : Seq Int, vars : Map String Int }

step : State → Set State

let reachable = fix(step, init)                 // sat
let good = exists (s -> s.pc = 42) reachable    // synthétisé
let norms = map normalize reachable             // hérité
```

- **Deux portes** vers le monde DD, comme discuté : opportuniste (le runtime convertit une valeur intermédiaire) et symbolique (quantifications). En Lisage, les deux portes sont le *même* mécanisme : l'oracle décide que `reachable` vaut une représentation DD ; `exists` et `map` deviennent des transductions.
- Les fermetures (HOF) ne descendent dans le DD que si elles se *réifient* en transduction attribuée (compilable) ; sinon `materialize`. Décision explicite du planner, pas d'échec silencieux.

---

## 8. Implémentation : les couches

```
lisage/
├── lang/         niveau 1 : types, interprète de référence (syntaxe concrète : décision de surface, roadmap § 10)
├── algebra/      niveau 2 : contrôleur étendu — schémas, Order<T>,
│                 transductions attribuées (semi-anneaux),
│                 réductions (quotients), fingerprints, canonicité,
│                 bibliothèque d'instances (BDD, ZDD, MTBDD, MDD, DDD,
│                 HSDD, SDD sententiels…) et instance de référence seq/set/map/tag
├── plan/         oracle de rentabilité, plan de représentation, conversions
├── core/         niveau 3 : pool multi-contrôleurs (formes, handles), IDs,
│                 GC, bi-plan, placement, co-localisation
├── backend/      sylvan | cudd : témoins (oracle BDD classique, baselines),
│                 mode dégradé
└── bench/        différentiel, property-based, MCC, non-local events
```

Règles de couches (critères de netteté, pas des slogans) :

- `algebra/` ne connaît pas `core/` : il ne voit que l'interface « stocker/récupérer un nœud par clé », qu'implémente `core/` ; les témoins (`backend/`) vivent côté `bench/` et ne remontent jamais au-dessus ;
- la frontière `core/` ↔ `algebra/` est **vérifiable**, pas un slogan : tout ce qui lit `Val(ℓ)`, `δ(ℓ)`, `Y`, un verdict d'ancre ou un cardinal vit dans `algebra/` (instancié par le contrôleur) ; tout ce qui compare clés et pointeurs vit dans `core/`, qui **n'inspecte jamais ⊕** : il route et mémoïse, les merges sont des callbacks opaques. Les réductions se répartissent de la même façon : R-share est syntaxique (côté pool) ; R-collapse, R-default et D-abs lisent le type (côté contrôleur) : le «+» et les réductions vivent au même étage ;
- `core/` est le chemin natif dès le jalon 1 : variables abstraites, formes relatives, défauts par contrôleur et canonicité par handle ne sont stockables dans aucun paquet existant. Sylvan/CUDD ne sont plus des backends mais des **témoins** : oracle différentiel quand un contrôleur est configuré en BDD classique (reproduction de CUDD à l'identique), lignes de base de bench ; le mode *dégradé* (transductions linéarisées en opérations binaires, sans mémoire de niveau ni bi-plan) survit comme route de comparaison. Le degré Lisage reste progressif : BDD → DDD → Lisage ;
- `bench/` est un citoyen de première classe dès le premier commit : differential testing (interprète vs DD), property-based (idempotence `reduce∘reduce = reduce`, commutation des quotients), et le triptyque A/B/C falsifiable (naïf / DD ordre fixe / Lisage+planner).

Séquencement (raccourci assumé des notes de conception, amendé) :

1. **Jalon 1** (8-10 semaines) : `core/` minimal : arena, `NodeId`, hash-consing à clé (contrôleur, ancre, forme), réductions spec § 2.3 ; `Atom borné + Set + Map` ; transductions hérité+synthétisé ; `map/filter/exists/fold` ; tests différentiels contre l'interprète de référence, Sylvan/CUDD en témoins sur l'instance BDD classique. → *répond déjà à « l'attribut remplace-t-il les contorsions actuelles ? »*
2. **Jalon 2** : `Seq/Tag` ; mémoire de niveau ; `saturate` avec les deux policies ; événements non locaux. → *le théorème d'expressivité.*
3. **Jalon 3** : Order typé complet, conversions à coût prédictible, oracle de rentabilité analytique. → *le plan de représentation.*
4. **Jalon 4** : pool bi-plan, co-localisation des attributs, métriques cache ; activation des formes relatives (shift) et du défaut généralisé (D-abs), prévues par le schéma core dès le jalon 1, mesure sur n philosophes contre encodage SDD hiérarchique. → *la localité et le partage.*
5. **Jalon 5** : langage complet, HOF, corpus MCC, oracle appris ; installation des SDD sententiels (test de complétude, contribution n° 6 ; la tour de contrôleurs comme motif d'instance, § 1.2).

---

## 9. Contributions attendues (et donc falsifiables)

| # | Contribution | Falsifiée si… |
|---|---|---|
| 1 | Sémantique attribuée unifiant homomorphismes, folds et saturation | un événement non local oblige une réification inévitable |
| 2 | Théorème de classification de canonicité par constructeur/schéma | un cas borné exige une représentation non canonique (ou inversement) |
| 3 | Ordre = contrat typé ; conversions à coût prédictible | le planner ne bat jamais « ordre fixe bien choisi » sur MCC |
| 4 | Oracle de rentabilité | il ne prédit pas mieux qu'un seuil heuristique |
| 5 | Pool bi-plan | les plans d'ombre coûtent plus qu'ils ne rapportent sur les workloads réels |
| 6 | Contrôleur étendu : toute catégorie de DD classique s'exprime comme instance (complétude testée sur les SDD sententiels de Darwiche) | une catégorie du paysage exige une extension *ad hoc* du cadre |
| 7 | Attribution sur semi-anneaux : comptage, model counting pondéré et évaluation probabiliste = une même transduction | un semi-anneau utile exige une sémantique d'opération distincte |
| 8 | Transformateurs unaires d'arêtes (shift, défaut, complément, swap) : une seule valuation structurelle, instance du contrôleur ; partage de stockage et de travail par invariance de translation (spec § 2.4) | sur N réplicas à événements invariants, le coût de saturation croît en O(N) au lieu de O(1) + bord ; ou le partage ne paraît pas sur n philosophes |

Une seule de ces lignes qui tient fait déjà un article ; les 1, 3 et 6 sont celles qui prolongent directement la thèse de Linard (2009) et *Specification of Decision Diagram Operations* (2010) — la 6 en est le critère de complitude. La 8 prolonge la valuation d'arêtes de polyDD du payload algébrique vers la structure.

---

## 10. Ce que la proposition ne retient pas des notes de conception

Pour marquer les choix :

- **Pas** de Semantic DD comme pivot systématique (risque d'explosion) ;
- **Pas** de génération de code à la compilation du programme utilisateur avant le jalon 5 (trop de couches à déboguer simultanément) ;
- **Pas** de planner entièrement automatique avant d'avoir un oracle analytique honnête ;
- **Inversion tranchée** : une bibliothèque DD bas niveau *spécifique* dès le jalon 1, pas au jalon 4. « Sylvan d'abord » supposait des nœuds classiques ; les formes relatives, le défaut désigné par contrôleur et la canonicité par handle ont rendu nos nœuds incompatibles avec tout paquet existant, et partir d'un moteur existant viderait le design de sa substance. Ce qui reste **pas** : une bibliothèque DD généraliste pour le monde ; `core/` est la couche stockage de nos formes, rien d'autre (pas de réordonnancement dynamique notamment : l'ordre est un contrat du contrôleur, les conversions s'en chargent). Sylvan/CUDD deviennent **témoins** : oracles différentiels sur l'instance BDD classique et lignes de base de bench. Risque assumé : le stockage sur le chemin critique dès le premier commit, couvert par l'interprète de référence (vérité) et le property-based testing (C1, quotients) dès le premier commit.

---

## 11. Le nom

« **Lisage** » — un seul nom pour la théorie, le cadre et l'outil. Le lisage est l'art de préparer un tissu Jacquard : la grille se lit sur deux axes (la chaîne descend, la trame remonte — exactement les attributs hérités et synthétisés), et le liseur y encode la figure sur des cartons perforés, l'ancêtre direct des cartes qui ont fondé l'informatique (Jacquard 1804 → Babbage → Hollerith). Étymologie : *lire* — l'oracle lit la structure, le code est un glossaire lisible. Le nom dit donc littéralement « encoder des décisions en une forme machine exécutable ». Rétroacronyme assumé : **L**angage, **I**nstances, **S**aturation, **A**ttributs, **G**reffe, **É**xécution — les six piliers du cadre. Collisions vérifiées et nulles : crate `lisage` libre sur crates.io, aucun logiciel homonyme sur GitHub (seule la filière textile), requête web non ambiguë. Le précédent « ADA » est abandonné : collision majeure (le langage Ada, la société AdaCore, les crates `ada` et `ada-url` occupées, le token ADA de Cardano qui noie les moteurs, et AADD déjà pris dans la littérature DD par les Affine Algebraic Decision Diagrams).