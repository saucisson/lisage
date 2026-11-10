# Analyse critique — note de conception « ΣDD structurels : seq/set/map/tagged union »

*Analyse critique de la note de conception fournie (2026), succédant à la note « Diagrammes de décision, IR et localité mémoire ».*

---

## 1. Ce que cette note apporte réellement

Trois idées y sont solides et à conserver :

1. **Le trio d'interprétations** (valeur structurée / fonction sur dimensions / collection de chemins) et la conséquence opérationnelle : `map`-sur-chaque-chemin ≠ `reduce`/`exists`-sur-l'ensemble des chemins ≠ fermeture (`saturation`). Cette taxonomie en trois familles (transformation de chemins, agrégation entre chemins, fermeture) est la bonne grille de lecture pour toutes les opérations du niveau 2.
2. **La canonisation relative** : `Canon(T, O_s, R)` — canonique pour le triple (décomposition, ordre, règles de réduction), pas absolu. C'est exactement le correctif à apporter au « Semantic DD canonique mathématique » de la note précédente, et la note l'intègre bien ici.
3. **L'ordre comme ressource algorithmique** et le programme « compiler l'expression *sous* l'ordre dont auront besoin les opérations aval », plutôt que construire puis réordonner. C'est la thèse de l'intrication ordre/opérations (Linard 2009 ; *Specification of Decision Diagram Operations*, 2010), reformulée en problème de compilation : la bonne reformulation.

Le reste appelle des réserves sérieuses, en particulier sur la nouveauté revendiquée et sur un angle mort canonicité. Détail ci-dessous.

---

## 2. Le point de départ : une lignée interne à l'écosystème du projet

Vérification faite : les **ΣDD** sont de **Didier Buchs et Steve Hostettler** (2009), le groupe de Genève, avec lequel les auteurs de polyDD ont co-publié (AlPiNA, CosyVerif). Le même groupe a publié les **Multi-Set Decision Diagrams** (Levi & Hostettler, 2009). Autrement dit, la note propose d'étendre une lignée *interne à l'écosystème du projet* (Genève, LIP6) : ce n'est pas une architecture posée sur une littérature étrangère, c'est une continuation interne, et elle doit être évaluée comme telle (en particulier : pourquoi les ΣDD n'ont-ils pas déjà été étendus ainsi ? La réponse de 2009-2012 était polyDD, Linard et al., ACSD 2010).

---

## 3. Le « Structural DD » : moins nouveau qu'il n'y paraît, et c'est une bonne nouvelle

La proposition d'un DD sur `Seq/Set/Map/Tag` est présentée comme un saut conceptuel. En réalité, chaque couche a un antécédent direct et connu :

| Couche proposée | Antécédent connu | Ce qui s'y joue |
|---|---|---|
| `Seq` (partage de suffixes, langage de séquences) | DD sur séquences = **trie partagé = automate minimal** (BDD/ZDD vus comme automates) ; **Queue BDDs** de Godefroid & Long (LICS 1996) | Un DD sur des séquences à positions variables *est* un automate déterministe minimal ; le partage de suffixes est la déterminisation/minimisation |
| `Set` (canonisation + DD de présence) | **Multi-Set Decision Diagrams** (Levi & Hostettler 2009) ; forme canonique = fonction caractéristique DD sur variables des éléments | Canonique ssi ordre total sur les encodages d'éléments ; rien de plus exotique qu'une BDD de fonction caractéristique |
| `Map` (relation clé→valeur) | HSDD (arcs valués par des ensembles) + MTBDD fonctionnels ; `Map<K,V> ≈ DD<K, Option<V>>` (la note le dit elle-même) | La « contrainte de canonisation sur les clés » = ordre des clés = un variable ordering de plus |
| `Tag` | Constructeurs de termes des ΣDD eux-mêmes ; unions typées des MDD | C'est la couche la plus proche des ΣDD d'origine |

**Conséquence pour le projet** : le « Structural ΣDD » n'est pas une nouvelle primitive, c'est une *synthèse* — et c'est précisément le programme de polyDD (un framework généralisant les catégories de DD) instancié sur la signature structurelle de Genève. La note dit d'ailleurs elle-même, en conclusion, qu'il faudra « voir si ça retombe sur des structures connues » : oui, en grande partie, et le seul endroit où ça *ne* retombe pas sur du connu est l'endroit que la note n'explore pas : **les opérations et leur compilation** (§ 7). Le slogan correct serait : les constructeurs sont connus, l'ordre-est-une-compilation est le vrai sujet.

---

## 4. L'angle mort canonicité : la preuve demandée est fausse en général

La note propose de démontrer : « pour toute structure finie, sa représentation réduite est unique ». C'est **faux sans hypothèses fortes**, et l'histoire du domaine le montre :

- Pour `Seq` non borné, le point de fixation de la canonicité, c'est la **minimalité d'automate** — atteignable seulement pour les représentations régulières. Les **Queue BDDs** de Godefroid & Long en sont la démonstration par l'échec : pour les files non bornées, la représentation canonique n'existe pas, et il a fallu approximer (perte de canonicité, de O(1)-égalité, de l'union en temps raisonnable).
- Pour `Set<T>`/`Map<K,V>` sur des atomes **infinis** (le langage cible a `Int` et `String` non bornés !), la forme canonique d'un ensemble fini de valeurs existe (fonction caractéristique sur l'encodage), mais son *support* (les dimensions) dépend de l'encodage des atomes — et change à chaque insertion hors du domaine déjà vu. La canonicité n'est donc stable que **relativement à un schéma d'encodage des atomes fixé** — un paramètre de plus dans `Canon(T, O_s, R)`, que l'architecture doit expliciter (schéma d'atomes = quelles dimensions existent, encodage borné ou pointé).
- Le cas `Map` non borné en clés est le pire : c'est exactement le territoire des DDD de Couvreur, dont la **non-canonicité est un choix assumé** pour cette raison. La note devrait dire : `Seq`/`Set` bornés → canonique ; domaines ouverts → DDD-style non canonique + homomorphismes, avec égalité par encodage syntaxique, pas par fonction.

**Correction à apporter au programme** : remplacer « la forme réduite est unique » par « la forme réduite est unique *relativement à* (schéma d'atomes, décomposition, ordre sémantique, règles) » et classer les constructeurs par statut canonique. C'est un théorème de classification à écrire — c'est là, et pas dans les constructeurs, qu'est la contribution théorique.

---

## 5. Terminal non booléens : la réponse est correcte, et c'est un résultat déjà connu

La question des terminaux non booléens dans la hiérarchie reçoit dans la note une réponse juste (chaque sous-DD est une fonction ; le nœud parent *compose* ces sous-fonctions ; le partage `D₁ = D₂` en découle). Mais il faut nommer ce que la réponse redécouvre : c'est **la sémantique même des SDD/HSDD** — un arc d'un HSDD porte un ensemble représenté par un sous-DD, et l'égalité en O(1) par identité de nœud est précisément le partage hiérarchique de Hamez. De même, la dualité « DD = ensemble (interprétation A) vs DD = fonction (interprétation B) » est la dualité classique **langage/fonction** (BDD vs MTBDD), et le fix du « une seule fonction globale » — *le DD représente un ensemble de chemins, une union* — est exactement la sémantique des DDD (un DDD dénote un ensemble d'affectations, union par construction).

En revanche, deux éléments de la note sont réellement bons et à garder :

1. **L'algèbre de défauts par type** (`Bool → false`, `Set → ∅`, `Map → ∅`, `Option → None`, `Int → 0`) : c'est la généralisation correcte des conventions BDD (défaut = faux) et ZDD (défaut = absent), déductible du type et surchargeable par l'opération. À intégrer dans le contrôleur polyDD comme paramètre à part entière — c'est concret et neuf sous cette forme.
2. **Le terminal fonctionnel** (`DD : Bool → (Int→Int)`, terminaux = fermetures) et la prudence immédiate sur l'égalité extensionnelle : correct. Le partage se fera sur la représentation canonique de la *syntaxe* (IR canonique de la fermeture), jamais sur la sémantique. C'est une contrainte structurelle du modèle, bien vue.

---

## 6. Ordre sémantique / ordre physique : convergence, mais trois coûts cachés

L'architecture `SemanticDD canonique → (permutation π par type, ordre physique) → PhysicalDD`, avec recompilation plutôt que reordering destructeur, converge avec l'analyse précédente (note « DD, IR et localité mémoire »), dont elle corrige d'ailleurs le point canonicité. Trois coûts restent cachés :

1. **Le Semantic DD lui-même peut être exponentiellement plus gros que le meilleur Physical DD.** La canonicité « indépendante de l'ordre physique » reste relative à une *décomposition sémantique* — qui est un choix d'ordre déguisé. Pour certaines fonctions, aucune décomposition sémantique unique n'est compacte alors que plusieurs ordres physiques le sont. Conséquence pratique : la stratégie « recompiler depuis le Semantic DD » vaut reconstruction complète — sur les très grands espaces d'états, c'est précisément le coût qu'on voulait éviter. Le Semantic DD doit donc être *optionnel* : matérialisé pour les objets qu'on sait devoir réutiliser sous plusieurs ordres, jamais comme passerelle systématique.
2. **Le planner doit connaître le futur.** « Compiler `x` directement sous l'ordre dont auront besoin `map f` puis `exists p` » suppose une chaîne d'opérations connue à la compilation. Avec des fonctions d'ordre supérieur passées en arguments, l'aval est une valeur *runtime* : le planner statique se réduit à des heuristiques, et il faut quand même du reordering dynamique dans la panoplie — le problème de CUDD ne disparaît pas, il est repoussé au rang de fallback.
3. **La composition est repoussée, pas résolue.** La note identifie elle-même « la composition de deux Semantic DD de décompositions différentes » comme la prochaine difficulté. C'est *le* problème : avec des permutations π **par type et par contexte**, l'espace d'incompatibilité entre opérandes explose littéralement (π₁ ≠ π₂ sur un même type suffit à casser les opérations binaires compilées). L'architecture a besoin d'une politique d'alignement explicite — c'était déjà le constat de l'analyse précédente ; les π par type l'aggravent.

---

## 7. Opérations : la note s'arrête pile là où la proposition Lisage commence

Le point le plus important, soulevé en objection : « on a aussi besoin d'opérations **non homomorphiques** ». La note l'admet (§ 8 : `lookup`, `insert`, `join`, `groupBy` ne sont pas des homomorphismes ; § 13 : trois familles), mais ne propose **aucun mécanisme** — elle renvoie à un vague « interpréteur sur l'ordre ».

Or la taxonomie en trois familles qu'elle donne est exactement la grille qui manque à l'idée d'homomorphismes à attributs de la proposition Lisage, et elle s'y superpose terme à terme :

| Famille (note) | Sens de l'information | Mécanisme |
|---|---|---|
| Transformation de chemins (`map`) | **attributs hérités** (descendants) | homomorphisme DDD classique |
| Agrégation entre chemins (`reduce`, `fold`, `exists`) | **attributs synthétisés** (remontants) | canal absent des homomorphismes actuels — c'est le mécanisme manquant |
| Fermeture (`saturation`) | les deux + **mémoire de niveau** (point fixe par niveau) | saturation d'Hamez = instance ; événements non locaux = cas où la mémoire de niveau manque |

`lookup : Map<K,V> × K → Option<V>` est le cas d'école parfait : l'opération doit porter la clé *vers le bas* (hérité), comparer aux clés rencontrées (contexte inter-niveaux), et *remonter* la valeur trouvée (synthétisé). Aucun de ces trois canaux n'existe dans le modèle homomorphique actuel. La note produit donc, sans le voir, l'énoncé de besoins de la proposition : **le « Decision IR » doit être fondé sur une sémantique de grammaire attribuée sur le diagramme** — hérités + synthétisés + mémoire de niveau — dont les homomorphismes DDD, les folds et la saturation sont les trois instances. C'est la pièce à écrire en priorité, et elle est absente du plan de la note.

---

## 8. Estimation et plan d'implémentation : solide, mais deux risques majeurs manquent au tableau

La partie estimation est raisonnable et honnête (phases 6/10 → 9/10, backend Sylvan/CUDD d'abord, planner par étapes, benchmark A/B/C falsifiable — très bien). Le tableau des risques est incomplet :

1. **Manque le risque scientifique n° 1, identifié par l'analyse précédente** : le modèle de coût (quand la représentation symbolique est-elle rentable). La note l'effleure (« l'IA ne va pas inventer la fonction de coût ») sans le traiter comme le goulot du projet. Le benchmark A/B/C le mesure *a posteriori* ; il faudrait une brique *prédictive* (cf. Sølvsten & van de Pol 2023 sur la prédiction des besoins mémoire des opérations).
2. **Manque le risque « explosion du Semantic DD »** (§ 6.1 ci-dessus) : toute l'architecture repose sur la recompilation depuis la forme sémantique ; si celle-ci est monstre, l'architecture se dégrade en reconstruction systématique.
3. **Manquent les métriques mémoire/localité** de l'analyse précédente : le benchmark proposé (nœuds, mémoire, temps d'opérations) ne mesure ni défauts de cache, ni hit-rates des tables (souvent dominantes), ni coût du placement — alors même que la note fait de la disposition un « annotation » de niveau 3.
4. **Manque le test de saturation** : le raccourci proposé (modèle `State / Set<State> / step / reachable`) est le bon, mais il faut y inclure un modèle à **événements non locaux** — c'est le seul qui mette simultanément à l'épreuve le canal inter-niveaux et la saturation par homomorphismes.

Sur les chiffres eux-mêmes : les ordres de grandeur (prototype 1-2 mois, système de recherche 3-6 mois, framework 18-36 mois) sont plausibles *pour l'ingénierie* ; ils sous-estiment la recherche (canonicité classifiée par constructeur, sémantique attribuée, alignement des ordres) dont dépend l'intérêt du tout — et la note admet implicitement ce point en concluant sur la question falsificatrice. À garder : la discipline « architecture qui permet de remplacer le backend » et « ne pas déboguer sept couches à la fois ».

---

## 9. Synthèse — ce qu'on garde, ce qu'on corrige, ce qu'on ajoute

**À garder tel quel**
- Les trois familles d'opérations (chemins / entre chemins / fermeture) comme grille du niveau 2.
- `Canon(T, O_s, R)` : canonicité relative, jamais absolue.
- L'ordre comme ressource algorithmique ; conversions (`convert`, `materialize`) comme nœuds de premier niveau à coût explicite dans le Physical IR.
- L'algèbre de défauts par type.
- Le plan d'implémentation par étapes et le benchmark A/B/C falsifiable.

**À corriger**
- Le théorème de canonicité : unique *relativement à* (schéma d'atomes, décomposition, ordre, règles) ; classification `Seq`/`Set`/`Map` bornés = canoniques vs domaines ouverts = DDD non canonique. S'appuyer sur les antécédents : automates minimaux (Seq), MSDD (Set), Queue BDDs (échec de canonicité non bornée), HSDD (Map/valeurs).
- Le Semantic DD : optionnel et jamais passerelle systématique (risque d'explosion).
- Le planner : admettre qu'avec fonctions d'ordre supérieur, l'aval est runtime ; le reordering dynamique reste un fallback nécessaire.
- La composition de décompositions/permutations différentes : à traiter comme *le* problème central, pas comme annexe.

**À ajouter (absent de la note)**
- La sémantique à attributs (hérités/synthétisés/mémoire de niveau) comme fondation du niveau 2 — elle implémente exactement les trois familles d'opérations et répare le déficit des homomorphismes identifié par *Specification of Decision Diagram Operations* (2010). `lookup` = cas d'école ; événements non locaux = banc de validation.
- Le modèle de coût prédictif en brique n° 1.
- Les métriques mémoire (tables, cache, placement) dans le benchmark.

**Positionnement stratégique** : cette note fait converger la lignée Genève (ΣDD, MSDD : Buchs/Hostettler) et celle de polyDD (contrôleurs) vers le même objet : une bibliothèque en couches où la couche basse est un backend générique (Sylvan/CUDD ou polyDD étendu), la couche moyenne l'algèbre décisionnelle (contrôleurs + attributs), et la couche haute le langage. La couche moyenne est la contribution ; les deux autres sont de l'ingénierie connue. C'est exactement « de moins structurée à plus structurée » — et chaque couche ne doit exiger de la couche inférieure que ce qu'elle fournit déjà (c'est le critère de netteté des couches à respecter dans le code : `semantic/` ne connaît pas `backend/`).

---

## 10. Références vérifiées pour cette analyse

- Buchs, D., Hostettler, S. — *Sigma Decision Diagrams*, 2009 (origine des ΣDD, groupe de Genève).
- Levi, L., Hostettler, S. — *Multi-Set Decisions Diagrams*, 2009 (antécédent direct de la couche `Set`).
- Godefroid, P., Long, D. — *Symbolic Protocol Verification with Queue BDDs*, LICS 1996 / STTT 1999. https://doi.org/10.1023/a:1008771008310 (antécédent — et échec de canonicité — de la couche `Seq` non bornée).
- Couvreur, Encrenaz, Paviot-Adet, Poitrenaud, Wacrenier — *Data Decision Diagrams for Petri Net Analysis*, ATPN 2002. https://doi.org/10.1007/3-540-48068-4_8 (sémantique ensembliste des chemins ; non-canonicité assumée).
- Hamez, Thierry-Mieg, Kordon — *Hierarchical Set Decision Diagrams and Automatic Saturation*, ATPN 2008. https://doi.org/10.1007/978-3-540-68746-7_16 (terminaux non booléens = sous-DD partagés ; saturation par homomorphismes).
- Miner & Ciardo — *Efficient Reachability Set Generation and Storage Using Decision Diagrams*, ATPN 1999. https://doi.org/10.1007/3-540-48745-x_2
- Hermanns, Meyer-Kayser, Siegle, Plateau — *Multi Terminal BDDs to Represent and Analyse Continuous Time Markov Chains*, 1999 (l'interprétation-fonction multi-terminale).
- van Dijk & Pol — *Sylvan: multi-core framework for decision diagrams*, 2016. https://doi.org/10.1007/s10009-016-0433-2
- Sølvsten & van de Pol — *Predicting Memory Demands of BDD Operations Using Maximum Graph Cuts*, TACAS 2023 (brique du modèle de coût prédictif).