# Analyse critique de la note de conception « Diagrammes de décision, IR et localité mémoire »

*Note de méthode : le document analysé est une note de conception exploratoire du projet (2026) ; sa bibliographie y était signalée comme non vérifiée. Les références vérifiables ont donc été vérifiées, et l'analyse sépare ce qui tient du document de ce qui tient du fond.*

---

## 1. Ce qui tient — et tient bien

1. **La distinction localité de représentation / localité d'exécution** est juste et rarement faite explicitement. La plupart des travaux sur le placement se préoccupent de la première et constatent empiriquement la seconde.
2. **La séparation en quatre dimensions** (ordre sémantique, ordre physique des variables, disposition mémoire, ordre d'exécution) est une bonne grammaire d'analyse. Réserve importante toutefois : voir § 4.7 — c'est une bonne *discipline de spécification*, mais pas un théorème de factorisation.
3. **La prudence méthodologique** (backend existant, interprète de référence, tests différentiels, property-based testing, fuzzing) est la bonne stratégie pour un prototype. Rien à redire sur le fond.
4. **La conclusion finale** — « ce sont les objets utilisés ensemble par les opérations dominantes qui doivent guider le placement » — est la bonne question, correctement formulée.

Le cœur du problème n'est pas dans ces points. Il est dans trois angles morts : un problème de recherche repoussé sans être nommé, une canonicité « sémantique » mal posée, et une factorisation trop propre entre ordre et opérations — précisément ce que *Specification of Decision Diagram Operations* (Linard, Hamez et al., 2010) démontre être insoutenable.

---

## 2. Le projet de langage : le modèle de coût est le sujet, il est repoussé en annexe

### 2.1 « Exploitation transparente » : un problème de recherche dur, pas une fonctionnalité

La voie 1 (`map`/`filter` « détecte » qu'une valeur se prête à une représentation DD) suppose un **modèle de profitabilité** : savoir *a priori* quand la construction symbolique d'une valeur + l'opération spécialisée coûtent moins que l'exécution concrète. C'est exactement le problème ouvert de la thèse de Linard (2009) et de polyDD (Linard et al., 2010) (quand une catégorie de DD « dédiée à un problème donné » est-elle rentable ?), et la note le traite comme un détail d'implémentation future. Il devrait être l'objectif de recherche n° 1 du projet, avec un premier modèle de coût même grossier : taille du domaine, densité attendue, coût de l'encodage, nombre de requêtes symboliques ultérieures (une représentation symbolique n'est jamais rentable pour *une* opération, seulement pour une *séquence*).

### 2.2 L'angle mort majeur : la composition d'opérandes d'ordres différents

Le document dit (§ 3.2) que « l'encodage peut dépendre du type et du contexte », mais ne confronte jamais la conséquence : si deux valeurs symboliques construites dans des contextes différents (donc des ordres différents) se combinent, `apply` inter-ordres est catastrophique — l'opération sur deux DD d'ordres incompatables peut exponentier en taille intermédiaire, ou imposer une permutation coûteuse d'abord. Une fonction compilée vers DD est un objet *fermé* dont l'ordre de variables fait partie du contrat d'appel. Le « plan physique » doit donc inclure une **politique d'alignement des ordres** comme primitive de première classe (réordonner l'opérande le plus petit ? maintenir un ordre global canonique du module ? payer le permute ?), faute de quoi le langage compose des représentations incompatibles. C'est le seul vrai problème conceptuel que la note ne voit pas, et il est lié à l'intrication ordre/opérations (§ 4).

### 2.3 La voie 2 est saine

La voie « explicitement symbolique » (`exists`, `forall`, requêtes) est au contraire bien posée : la quantification est une *abstraction* sémantique naturelle, et le compilateur choisit librement l'encodage en dessous. C'est le niveau auquel les deux voies devraient d'ailleurs se rejoindre : le compilateur peut réutiliser la voie symbolique comme cible de spécialisation pour la voie transparente, le modèle de coût décidant de la conversion.

---

## 3. Semantic IR / Physical IR : la « canonicité mathématique » est mal posée

### 3.1 Il n'existe pas de forme canonique indépendante de l'encodage

Le § 2.1 présente le Semantic DD comme « forme canonique mathématique, même si non matérialisée ». Or dans toute la littérature (Bryant, Darwiche, les contrôleurs polyDD), la canonicité est *toujours relative à un triple* : (ordre des variables, règles de réduction, convention de défaut). Un ordre sémantique « conceptuel » est lui aussi un choix — simplement un choix différent. Le niveau sémantique ne peut pas être canonique ; il peut seulement être **défini à équivalence près** (la classe de toutes les représentations physiques de la même fonction). C'est d'ailleurs une définition plus utile : le Semantic DD est une classe d'équivalence, et le plan physique est une fonction de choix dans cette classe. Formulé ainsi, la distinction Semantic/Physical devient propre.

### 3.2 « Annotations plutôt que langage séparé » : le risque documenté

L'idée d'un IR unique avec annotations est défendable (c'est grosso modo la stratégie LLVM), mais l'expérience des compilateurs réels montre le mode de défaillance : les annotations mutent en nœuds de premier niveau dès qu'une passe doit raisonner dessus. La ligne de partage correcte est :

- les **conversions** entre représentations sont des opérations *sémantiques* (elles changent l'objet, pas seulement son coût) → nœuds de premier niveau du IR ;
- les **hints** (traversal plan, placement) sont des métadonnées *non observables* sémantiquement → annotations.

La note effleure cette distinction (« nœuds explicites indiquant des conversions ou des coûts ») mais la traite comme un détail. C'est le point d'architecture le plus important du document et il devrait être tranché avant tout prototype.

### 3.3 La liste des réductions mélange trois natures différentes

Le § 3.1 classe ensemble des mécanismes qui ne sont pas de même nature :

1. **Réductions canoniques préservant la fonction** : partage des sous-DD identiques, suppression des tests redondants. Ce sont les seules qui méritent le nom de *réduction* : elles ne changent ni la fonction représentée ni l'interprétation des diagrammes.
2. **Changements de convention canonique** : le zero-suppressed n'est *pas* une réduction du BDD — c'est une autre algèbre. La branche manquante n'a plus la même signification (jamais sélectionné vs. par défaut), donc changer de convention change l'interprétation de *tous* les diagrammes du système, pas seulement d'un sous-graphe. Idem pour le collapse/don't-care, qui suppose une logique à trois valeurs ou des fonctions partiellement définies. Ces choix relèvent du contrôleur (au sens polyDD), pas du niveau « réduction ».
3. **Compression physique non observable** : packing, compactage de champs.

En outre, le document range le **hash-consing** dans la catégorie 3, alors qu'il est le mécanisme même de la canonicité : la table d'unicité fait de l'identité de nœud une conséquence du contenu. Le hash-consing n'est pas une optimisation mémoire parmi d'autres ; c'est ce qui rend l'égalité structurelle décidable en O(1) et c'est précisément ce qui **contraint** les stratégies de placement (voir § 4.5).

### 3.4 « Construire directement dans l'ordre physique » : le plan statique ne peut pas exister

Le § 3 affirme qu'on peut éviter le reordering destructeur en construisant directement selon le plan physique. La difficulté éludée : le bon ordre dépend des *résultats intermédiaires* de la construction, pas seulement de la description sémantique initiale. C'est la raison d'être du sifting déclenché dynamiquement dans CUDD (seuils de nœuds). Le « plan physique » doit donc être un *processus réactif* (observable : compteurs, seuils, replanification), pas une décision initiale. La note le présuppose statique : contradiction directe avec l'intrication ordre/opérations (Linard 2009 ; *Specification of Decision Diagram Operations*, 2010).

---

## 4. Localité mémoire : réponse globalement juste, trois incomplétudes notables

### 4.1 Oubli du multi-cœur / NUMA

Sylvan figure dans les références, mais toute la note est mono-thread. Or la localité dominante d'un moteur multicœur est **l'affinité NUMA et la contention sur les tables** (unique table et computed table partagées), pas la disposition des nœuds. Point d'entrée existant exactement à la jonction des thèmes de la note : *Multi-core On-The-Fly Saturation* (van Dijk, Meijer & van de Pol, TACAS 2019, [DOI](https://doi.org/10.1007/978-3-030-17465-1_4)) — saturation, multi-cœur et ordre des variables y interagissent frontalement.

### 4.2 `apply` est un parcours du *produit* de deux DAG

La limite la plus fondamentale du placement par diagramme, non dite dans le document : une opération binaire visite des *paires* de nœuds, une dans chaque opérande. La localité d'`apply` dépend donc simultanément des deux dispositions, qui interfèrent — aucune disposition d'un seul diagramme ne peut capturer le motif d'accès du produit. Les deux conclusions correctes que le document ne tire pas : (i) le placement doit se raisonner *par opération* (voire par paire d'opérations fréquentes), pas par diagramme ; (ii) pour les opérations dominantes binaires, le vrai levier est la disposition *conjointe* (co-placer deux diagrammes souvent combinés), ce qui rejoint le mark-sweep-shift de la bibliothèque Microsoft mais le généralise.

### 4.3 Tables souvent dominantes

La note optimise les nœuds ; or dans un package réel, la computed table et la table d'unicité représentent fréquemment l'essentiel du trafic mémoire et des défauts de cache (la table d'unicité est systématiquement traversée à chaque création de nœud ; la computed table à chaque sous-problème). Un benchmark qui ne mesure que les défauts de cache *sur les nœuds* peut optimiser la composante minoritaire. (Appui : CacBDD — Lv, Su & Xu 2013, « A BDD Package with Dynamic Cache Management », [DOI](https://doi.org/10.1007/978-3-642-39799-8_15) — montre que la gestion du cache à elle seule est un sujet.)

### 4.4 La tension fondamentale non identifiée : adressage par contenu vs. adressage par usage

Le hash-consing assigne aux nœuds une identité *fondée sur le contenu* ; le placement par affinité d'accès veut les disposer selon l'*usage*. Ces deux principes produisent des affectations contradictoires. Conséquence pratique intéressante que le document pourrait exploiter : dans un hash-consing standard, les IDs sont attribués **en ordre de création**, donc la disposition par défaut capture l'affinité temporelle de la *construction*, pas celle des opérations ultérieures. D'où deux stratégies cohérentes avec ce constat : (a) recaler les IDs lors de la compaction selon le profil de la *phase suivante* (généralisation du mark-sweep-shift) ; (b) renoncer au placement fin et optimiser les tables — ce qui ramène au § 4.3. Le document oppose compaction/profilage sans voir que la question de fond est ce conflit d'adressage.

### 4.5 Bibliographie : partiellement invérifiable, deux erreurs identifiées

La note le reconnaît elle-même, mais voici l'état des vérifications :

| Référence citée | État |
|---|---|
| *Binary Decision Diagrams on Modern Hardware*, FMCAD 2023 | Confirmée : Samuel Pastva (Université Masaryk), le PDF `is.muni.cz` est cohérent |
| *TeDDy* → lien `github.com/ahamez/eddy` | **Erreur** : ce lien pointe vers EDDy (bibliothèque d'Alexandre Hamez). TeDDy est un autre projet : Mrena, Kvaššay & Zaitseva, « TeDDy: Templated decision diagram library », SoftwareX 2024 ([DOI](https://doi.org/10.1016/j.softx.2024.101715)). Les deux méritent d'être cités, séparément |
| *Design of a Pointerless BDD Package* (2001) | Introuvable en base bibliographique sous ce titre exact — à re-vérifier avant usage |
| Yang & O'Hallaron 1997 | DOI plausible mais non confirmé lors des vérifications |
| Chilimbi & Larus 1998, Sylvan, CUDD, Microsoft DecisionDiagrams | Solides |

Et deux absents récents directement dans la cible du projet : la manipulation de BDD **en mémoire externe** (Sølvsten, van de Pol et al., arXiv 2021, [DOI](https://doi.org/10.48550/arxiv.2104.12101)) et la **prédiction des besoins mémoire des opérations** sur BDD (Sølvsten & van de Pol, TACAS 2023) — ce dernier est exactement un morceau du modèle de coût manquant du § 2.1.

### 4.6 Benchmark : méthodologie à durcir

Le plan de benchmark (§ 7) est bon, mais deux pièges ne sont pas signalés :

1. **Confusion densité/hachage** : une compaction change l'assignation des IDs, donc les séquences de sondage des tables de hachage. Mesurer « compaction on/off » capture simultanément l'effet de densité et l'effet de distribution de hachage. Il faut contrôler (randomiser les IDs de façon reproductible, ou mesurer avec/sans re-hash).
2. **Manques dans les mesures** : hit-rate de la computed table, taille relative des tables vs. nombre de nœuds, ordre de construction contrôlé (même fonction, ordres de construction différents → dispositions différentes). Et des workloads réalistes existent déjà : les modèles du Model Checking Contest servent de banc éprouvé à toute cette lignée d'outils.

---

## 5. Le point qui manque à toute la note : l'intrication ordre/opérations

La remarque finale du document n'y est pas développée, et elle en est pourtant la clef de voûte critique : **l'ordre et les opérations sont intriqués, et les homomorphismes manquent d'un canal d'information inter-niveaux**.

### 5.1 Mise en forme du diagnostic

Dans le modèle DDD, un homomorphisme est un opérateur descendant qui se décompose niveau par niveau : à chaque variable, il choisit les successeurs. Le seul canal d'information est le *paramètre* de l'opérateur (les combinainateurs de Thierry-Mieg : `Collected`/`IfThenElse`/etc.), donc toute information contextuelle doit être gelée dans l'opérateur *avant* la descente. Il n'y a :

- **pas de canal remontant** (attributs synthétisés : une information calculée dans un sous-arbre n'est pas consultable par le niveau parent pendant l'opération) ;
- **pas de mémoire de niveau** (pas d'état stocké entre deux visites du même niveau, hormis le diagramme lui-même) ;
- et donc pas d'autre choix que de *réifier l'état dans les variables* (variables intermédiaires) quand un calcul dépend de la conjonction de plusieurs niveaux — avec le coût que cela suppose en taille de diagramme.

### 5.2 La littérature valide empiriquement ce diagnostic

C'est exactement le problème rencontré par la **saturation avec événements non locaux** : les transitions couvrant plusieurs niveaux cassent la validité/efficacité de l'itération par niveau. La lignée Ciardo/Zhao le contourne par de la *localisation d'événements* (event localization : réécrire les événements multi-niveaux en événements locaux via des niveaux intermédiaires) — c'est-à-dire précisément la réification d'état dénoncée ci-dessus, faite à la main. Les deux diagnostics (intrication ordre/opérations, absence de stockage inter-niveaux) sont les deux faces du même phénomène : **le niveau n'est pas une frontière d'encapsulation étanche**, et les formalismes actuels font comme si elle l'était.

### 5.3 Deux conséquences pour le projet

**Vers la sémantique.** La formalisation naturelle existe : les homomorphismes actuels correspondent à des *attributs hérités* (l'information ne descend que par paramétrage). Il manque les *attributs synthétisés* et une mémoire de niveau — autrement dit, passer des transducteurs descendants aux **grammaires attribuées** sur le diagramme (ou aux réécritures d'arbre à état). Un « contrôleur » polyDD généralisé devrait spécifier, par catégorie de DD, les deux flux (hérité/synthétisé) et la politique de stockage inter-niveaux. C'est, à ce jour, un vide réel dans la littérature, et l'extension naturelle de *Specification of Decision Diagram Operations* (2010).

**Vers la mémoire.** Il y a une convergence heureuse avec le sujet de localité : si l'information inter-niveaux doit être stockée quelque part, la question de *son* placement se pose en même temps que celui des nœuds. Stocker l'attribut *dans* le nœud (ou dans un tableau parallèle co-localisé) plutôt que dans une structure séparée résout le problème sémantique et le problème de localité ensemble : le `TraversalPlan`/`PlacementPlan` de la note gagnerait un quatrième concept, un `AttributeStore`, dont la disposition est partie intégrante du plan physique. Le benchmark du § 7 du document devrait donc ajouter une variante : opérations à attributs inter-niveaux, avec et sans co-localisation des attributs.

---

## 6. Synthèse — cinq corrections prioritaires

1. **Promouvoir le modèle de coût** (quand une représentation symbolique est-elle rentable ?) d'implémentation future à objectif de recherche central ; s'appuyer sur la prédiction de besoins mémoire (Sølvsten & van de Pol 2023) comme première brique.
2. **Ajouter la politique d'alignement des ordres** aux primitives du plan physique ; c'est la conséquence non traitée de « l'encodage dépend du contexte ».
3. **Trancher l'IR** : conversions = nœuds sémantiques de premier niveau ; hints = annotations non observables. Pas de statut intermédiaire.
4. **Reclassifier les mécanismes** : réduction canonique / changement de convention canonique / compression physique — trois catégories étanches, et le hash-consing côté fondations de la canonicité, pas côté optimisation.
5. **Fusionner les deux fils** (intrication ordre/opérations + canal inter-niveaux) : la saturation avec événements non locaux est le cas d'école ; un prototype d'homomorphismes à attributs sur ce cas validerait cette thèse et donnerait au projet son résultat de recherche distinctif.

---

## 7. Références ajoutées lors de la vérification

- Pastva, S. — *Binary Decision Diagrams on Modern Hardware*, FMCAD 2023 (confirmée).
- Mrena, M., Kvaššay, M., Zaitseva, E. — *TeDDy: Templated decision diagram library*, SoftwareX 2024. https://doi.org/10.1016/j.softx.2024.101715
- van Dijk, T., Meijer, J., van de Pol, J. — *Multi-core On-The-Fly Saturation*, TACAS 2019. https://doi.org/10.1007/978-3-030-17465-1_4
- Sølvsten, S. C., van de Pol, J., et al. — *Efficient Binary Decision Diagram Manipulation in External Memory*, arXiv 2021. https://doi.org/10.48550/arxiv.2104.12101
- Sølvsten, S. C., van de Pol, J. — *Predicting Memory Demands of BDD Operations Using Maximum Graph Cuts*, TACAS 2023.
- Lv, G., Su, K., Xu, Y. — *CacBDD: A BDD Package with Dynamic Cache Management*, 2013. https://doi.org/10.1007/978-3-642-39799-8_15
- Minato, S. — *Streaming BDD manipulation*, IEEE Trans. Computers 2002. https://doi.org/10.1109/tc-2002.1004587