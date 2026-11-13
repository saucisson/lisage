# Feuille de route : jalons et issues

Base de départ : le dépôt GitHub **saucisson/project-template**, adapté à Rust. Le dépôt projet (suggestion de nom : `lisage`) accueillera le code **et** les documents produits pendant la conversation (importés dans `docs/`, issue M0-3). Les jalons reprennent et affinent le séquencement de la proposition (§ 8) ; l'outillage de visualisation (**Liseuse**, décrit dans le canvas dédié) est intégré à chaque jalon plutôt qu'ajouté à la fin : c'est l'instrument de vérification des énoncés (C1, minimalité par handle) et de production des figures.

## 1. Conventions de dépôt

**Adaptation du template vers Rust** (ce qui change, tout le reste est conservé) :

| Élément du template | Adaptation Rust |
|---|---|
| `ruff` + `mypy` (pre-commit, CI) | `rustfmt` + `clippy -D warnings` (+ `cargo-deny` pour les licences/avertissements) |
| `.python-version` | `rust-toolchain.toml` (channel stable épinglé) |
| `pyproject.toml` | `Cargo.toml` workspace + `deny.toml` |
| pre-commit | conservé : `gitleaks`, `rustfmt`, `clippy`, fin de fichier, marqueurs de conflit |
| release-please | conservé tel quel (versionnement par tags) |
| CI GitHub Actions + `.gitlab-ci.yml` | mêmes étapes : fmt → clippy → test (`cargo-nextest`) → coverage (`llvm-cov`) → audit (`cargo-audit`) |
| `AGENTS.md`, `.vibe/`, `tools/notify` | conservés (sessions Vibe, notifications) |

**Workspace cargo, une crate par couche** (proposition § 8) :

```
lisage/
├── crates/
│   ├── lisage-lang/      niveau 1 : types, interprète de référence (syntaxe concrète : décision de surface, § 10)
│   ├── lisage-algebra/   niveau 2 : contrôleur étendu, transductions, réductions
│   ├── lisage-plan/      oracle de rentabilité, conversions, plan de représentation
│   ├── lisage-core/      niveau 3 : pool, handles, GC, bi-plan, placement
│   ├── lisage-backend/   témoins : sylvan | cudd, mode dégradé
│   ├── lisage-bench/     différentiel, property-based, MCC, événements non locaux
│   └── lisage-liseuse/   le visualiseur (trace, index, rendu SVG/HTML)
├── docs/                 les documents de la conversation (voir M0-3)
└── bench-results/        sorties de bench, figures exportées
```

**Labels** : `area/core`, `area/algebra`, `area/lang`, `area/plan`, `area/bench`, `area/liseuse`, `area/docs`, `area/infra` ; `type/feat`, `type/test`, `type/doc`, `type/spike` ; `jalon/M0` … `jalon/M5` ; et un label par énoncé falsifiable : `C1`…`C8`, `T1`…`T5`, `A5`, `L3`, `A6`. Chaque issue cite ses énoncés dans le corps ; chaque PR ferme par `Closes #n` (convention du template).

**Miroir des documents** : les canvases restent la source de vérité pendant la rédaction ; `docs/` en est le miroir versionné (import initial en M0-3, resynchronisation à chaque jalon). Cible : `docs/proposition.md`, `docs/specification.md`, `docs/etat-de-lart.md`, `docs/exemple-philosophes.md`, `docs/analyses-critiques/{dd-ir-localite,sigmadd-structurels}.md`, `docs/liseuse.md`, `docs/roadmap.md` (ce document).

**Modèle d'exécution : agent, revue humaine, discussion juste-à-temps.** Le code est écrit par un agent (Vibe Code), la conception s'élucide dans la conversation, l'humain révise. Trois conséquences sur le plan. (1) **Les estimations quittent les semaines-homme** : la production de code, de tests et de documentation s'effondre ; l'horloge réelle devient triple : disponibilité de la revue humaine (le chemin critique), temps machine des benches (PBT longs, corpus MCC), et cadence d'élucidation des points ouverts dans la discussion. Le risque scientifique (C1, unfold, T5) ne se comprime pas : la falsification prend le temps qu'elle prend. L'unité de plan est donc le **cycle** (une session de travail de l'agent close par une revue humaine), et les chiffres de § 8 restent indicatifs jusqu'à la calibration du premier jalon. (2) **Escalade juste-à-temps** : l'agent ne bloque jamais sur un point non tranché (§ 10 ci-dessous, questions ouvertes de la spec) ; il l'escalade dans la discussion, la réponse devient un ADR ou une ligne de spec, et le travail reprend. (3) **Discipline d'ordre par jalon : interface → tests → implémentation.** Phase 1, l'interface : traits, types, bounds, politique d'erreurs, ADR ; une frontière compilée et documentée est le contrat idéal pour un agent (le test de frontières de M0-2 l'ancre dès le départ). Phase 2, les tests : l'interprète de référence d'abord (l'oracle, volontairement naïf et sans DD), puis les propriétés par énoncé (PBT rouge) et le harnais différentiel. Phase 3, l'implémentation : rendre vert, énoncé par énoncé. La discipline vaut **par jalon**, pas pour le projet entier : figer l'interface de M3 avant les leçons de M1 serait un retour à la cascade ; l'ordre s'applique à l'unité de travail, juste-à-temps.

## 2. M0 : mise en place (un cycle)

| Issue | Titre | Contenu | Critère d'acceptation |
|---|---|---|---|
| M0-1 | Dépôt depuis project-template | Import du template, purge du Python (ruff/mypy/.python-version), réglage Rust (toolchain, pre-commit, release-please) | CI verte sur les étapes fmt/clippy/nextest/coverage/audit, avec un test factice |
| M0-2 | Squelette du workspace | Les 7 crates, vides mais aux frontières déclarées (dépendances unidirectionnelles dans `Cargo.toml`, commentées du critère de couches de la proposition § 8) | `cargo nextest` et `clippy -D warnings` passent ; une dépendance interdite est rejetée par un test de frontières |
| M0-3 | Import des documents | Les documents de la conversation dans `docs/`, liens relatifs vérifiés, un README par document (statut : travail en cours) | `docs/` complet, aucune ancre cassée |
| M0-4 | Charte de test | Triptyque A/B/C décrit (naïf / DD ordre fixe / Lisage+planner), interprète de référence précisé comme source de vérité, `proptest` installé, convention de nommage `test_c<n>_*` par énoncé | Un premier test différentiel factice passe dans les trois voies |
| M0-5 | Socle de bench | `criterion` branché ; harness MCC conteneurisé à vide (le corpus arrive en M5, l'outillage non) | Un micro-bench tourne en CI et refuse les régressions de temps |

## 3. M1 : le cœur vérifiable (jalon 1 de la proposition, 4 à 6 cycles)

**Résumé** : `core/` minimal, instance de référence `Atom borné + Set + Map`, transductions hérité + synthétisé, `lookup` comme opération-test, `map/filter/exists/fold`, différentiel contre l'interprète de référence, témoins Sylvan/CUDD sur l'instance BDD classique. Liseuse v0 : trace + vue diagramme statique. Ordre d'exécution (discipline du modèle d'exécution, § 1) : **interface** (M1-0), **tests** (M1-7, l'oracle, puis les PBT rouges par énoncé), **implémentation** (M1-1 à M1-6 rendus verts), fermeture (M1-8 à M1-10).

| Issue | Titre | Contenu | Énoncés |
|---|---|---|---|
| M1-0 | Interface du cœur (phase 1 : interface) | Les traits d'interface : pool, algèbre de clés, contrôleur étendu, `Observer` ; `NodeId`/`Handle` ; bounds `Send`/`Sync` (§ 10) ; politique d'erreurs ; ADR 0001+ ; test de frontières étendu aux signatures ; spike FFI CUDD/Sylvan en parallèle (§ 10, avant toute dépendance du différentiel) | compile, `cargo doc`, frontières vertes | contrat |
| M1-7 | Interprète de référence (phase 2 : tests) | La vérité, en pur `lang/` sans DD ; différentiel A/B/C branché dès le premier commit ; les PBT par énoncé écrits **rouges** avant l'implémentation | falsifiable |
| M1-1 | Arena + handles | `arena`, `NodeId`, hash-consing à clé (contrôleur, ancre, forme) ; la table des handles se clé sur la forme réduite pour cette lecture | C1 |
| M1-2 | Réductions § 2.3 | R-share, R-collapse, R-default ; confluence testée par PBT (`reduce∘reduce = reduce`, commutation des quotients) | C1 |
| M1-3 | Schémas de référence | `Atom(D)` borné, `Set(T)`, `Map(K,V)` bornés ; compilation balayage d'univers (un niveau de présence par clé, δ = absent, sans marqueur) | Σ |
| M1-4 | Transductions hérité/synthétisé | Les deux canaux, l'agrégation `φ_aggr`, les fingerprints de mémo (op, nœud) | L3 |
| M1-5 | `lookup` | L'opération-test : synthétisé = fragment de DD (pas un scalaire), clé portée vers le bas, comparaison en contexte | spec § 5 |
| M1-6 | Pli et semi-anneaux | `map/filter/exists/fold` ; comptage = pli sur (ℕ, +, ×) ; merge de construction déclaré (hash-consing = choix Bool) | A5, C7 |
| M1-8 | Témoins BDD | Configuration du contrôleur en BDD classique ; différentiel contre CUDD/Sylvan (reproduction à l'identique), lignes de base de bench | C6 |
| M1-9 | Port d'instrumentation | Trait `Observer` (no-op par défaut) dans `core/` ; événements typés ; sérialiseur de trace JSONL versionné ; Liseuse v0 : rendu SVG statique du diagramme, niveaux = clés | Liseuse |
| M1-10 | Démo philosophes | n = 3, 4, 5 en `map/filter/exists` + saturation via `fold` ; figure exportée depuis Liseuse ; les 7 formes attendues visibles et vérifiées | C8 |

**Critères de sortie du jalon** : différentiel nul contre l'interprète sur l'instance de référence ; C1 tenu (unicité des handles, vérifiée par PBT) ; la vue diagramme de Liseuse montre les 7 formes des philosophes et leurs multiplicités.

## 4. M2 : expressivité (jalon 2)

**Résumé** : `Seq`/`Tag`, mémoire de niveau, `saturate` avec les deux policies, événements non locaux. Liseuse v1 : la trace pas à pas.

| Issue | Titre | Contenu | Énoncés |
|---|---|---|---|
| M2-1 | `Seq` et ses deux compilations | Répétition + marqueur de fin (clé requise) ; compilation par positions et par énumération d'éléments ; T5 durci pour l'énumération (borné par la longueur, pas par l'univers) | T5 |
| M2-2 | `Tag` | L'union typée : niveau à un arc utile, discriminant | C2 |
| M2-3 | Mémoire de niveau | État stocké entre deux visites d'un même niveau ; politique de stockage déclarée par le contrôleur | T3 |
| M2-4 | `saturate`, deux policies | Statique par niveaux (à la Ciardo) et auto-détectée (à la Hamez), comme deux instances d'une même sémantique du point fixe | T3 |
| M2-5 | Événements non locaux | Compilation des transitions de largeur w en transductions à mémoire de niveau, surcoût poly(w) | T3, C1 |
| M2-6 | Liseuse v1 : pas à pas | Scrubber sur la trace ; canaux colorés (hérité, synthétisé, mémoire) ; `MemoHit`/`MemoMiss` ; rounds de saturation ; différenciel avant/après avec fantômes | Liseuse |

## 5. M3 : l'ordre et le plan (jalon 3)

**Résumé** : `Order<T>` complet, conversions à coût prédictible, oracle de rentabilité analytique. Liseuse : la vue ordre et royaumes.

| Issue | Titre | Contenu | Énoncés |
|---|---|---|---|
| M3-1 | `Order<T>` valeur de première classe | Ordres typés, π par type contraintes dans `Order<T>`, alignement comme typage (deux opérandes ne se composent que sous un ordre commun) | C3 |
| M3-2 | Conversions | Réordonner = convertir, transductions à coût prédictible ; contrat typé étendu des ordres aux encodages (royaumes de contrôleurs) | C3 |
| M3-3 | Oracle de rentabilité analytique | Premier modèle prédictif : taille du domaine, densité attendue, coût d'encodage, nombre de requêtes aval ; garde-fou par différentiel | C4 |
| M3-4 | Liseuse : vue ordre | O comme piste linéaire ou arbre (vtree), fiche contrôleur (les huit champs), royaumes côte à côte, conversions fléchées avec coût | Liseuse |

## 6. M4 : localité et partage (jalon 4)

**Résumé** : pool bi-plan, co-localisation des attributs, métriques cache ; activation des formes relatives (shift) et du défaut généralisé (D-abs) ; mesure des philosophes contre l'encodage SDD hiérarchique. Liseuse : les vues mémoire et quotients.

| Issue | Titre | Contenu | Énoncés |
|---|---|---|---|
| M4-1 | Pool bi-plan | Plans principal et d'ombre, GC, compaction profilée ; métriques cache dans `bench/` | C5 |
| M4-2 | Formes relatives | Activation du shift (encodage relatif, ancre) prévu par le schéma core dès M1 ; plus les deux extensions de la spec 2.4.5 : ancres composées (décalages nommés par fragment) et transformations de clés désignées par le contrôleur (groupe d'action + section canonique des orbites ; instance canonique : normalisation min = 0, argument n sur l'arc entrant ; applications : compteurs non bornés, couleurs, régions convexes ; l'instance canonique a désormais son modèle-test : la fenêtre glissante, canvas exemple-fenetre-glissante-lisage : la base de séquence = l'argument sur l'arc, la glisse = le déplacement d'ancre, l'ISN = la section canonique, Wireshark en précédent industriel) | C8 |
| M4-3 | D-abs / D-trans | Défaut généralisé : absorption sur tout sous-domaine, D^déf désigné par contrôleur | A6 |
| M4-4 | Benchmark philosophes | Mesure contre SDD/HSDD : 7 nœuds attendus, O(1) + bord, ratio gaucher 1,2887 constant, pic vivant, passes, firings ; falsification explicite si la mesure dit autre chose | C8 |
| M4-5 | Liseuse : vues mémoire et quotient | Pools, GC, badges de multiplicité (replicas), classes du quotient (formes × ancres), mode comparaison naïf/DD/Lisage | Liseuse |

## 7. M5 : échelle et complétude (jalon 5)

**Résumé** : langage complet, fonctions d'ordre supérieur, corpus MCC, oracle appris, installation des SDD sententiels, instance ADD/MTBDD. Liseuse : export figures articles.

| Issue | Titre | Contenu | Énoncés |
|---|---|---|---|
| M5-1 | Langage complet + HOF | Niveau 1 complet ; le planner admet un aval runtime (reordering dégradé en fallback) | C3 |
| M5-2 | Corpus MCC | Modèles MCC 2011-2018 conteneurisés ; pile d'import des formats du corpus (parsing PNML, conversion vers `lang/` : un parseur de *formats externes*, pas du langage Lisage) ; triptyque A/B/C complet ; publication des chiffres | C3, C4 |
| M5-3 | SDD sententiels | Test de complétude : installation sans extension ad hoc (vtree = ordre structuré, tour de contrôleurs comme motif d'instance) | C6 |
| M5-4 | ADD/MTBDD | Merge non idempotent, table indexée par les poids ; différentiel de compatibilité en garde-fou (question ouverte n° 8 de la spec) | C7 |
| M5-5 | Liseuse : export figures | Mode publication : SVG/PGF propre à la mise en page article, thème serif, légendes auto depuis la légende des canaux | Liseuse |

## 8. Synthèse

| Jalon | Cycles de revue (indicatif) | Énoncés visés | Liseuse |
|---|---|---|---|
| M0 | 1 | aucun (socle) | aucun |
| M1 | 4 à 6 | C1, L3, A5, C7 (départ), C6 (départ) | v0 : trace + diagramme statique |
| M2 | 2 à 3 | T3, T5, C2 | v1 : trace pas à pas |
| M3 | 1 à 2 | C3, C4 | vue ordre/royaumes |
| M4 | 2 à 3 | C5, C8, A6 | vues mémoire/quotient |
| M5 | 3 à 5 | C3, C4, C6, C7 (complétude) | export figures |

Un **cycle** = une session de travail de l'agent close par une revue humaine. La compression par rapport aux semaines-homme d'origine vient du mode d'exécution (§ 1), mais elle ne s'achète ni sur les horloges réelles (disponibilité de la revue, temps machine des benches, cadence d'élucidation dans la discussion), ni sur le risque scientifique (C1, unfold, T5). Chiffres à recalibrer après M1 sur les cycles réellement consommés.

Le fil directeur : **chaque jalon termine par une figure ou une mesure publiée depuis Liseuse**. L'outil n'est pas un luxe pédagogique ; c'est le moyen de voir C1 (deux formes identiques sous ancres différentes ne fusionnent pas), le travail de saturation (rounds, firings) et le partage (multiplicités, O(1) + bord), qui sont les trois choses que le texte affirme et que le lecteur d'un article voudra voir.

## 9. Backlog (candidats d'extension, hors M0-M5)

- **δ continu** (question ouverte 9 de la spec) : algèbres de clés-régions, intervalles puis DBM ; instance-test : DDD-différence / CDD. Le contrat de clés (forme normale, couverture, intersection) se déclare côté contrôleur ; le pool ne change pas. Le pli devient intégrale : à ne lancer qu'après l'instance ADD/MTBDD (M5-4), les deux questions touchant au même champ attribution.
- **Coinductif** (candidat question ouverte 10, cf. état de l'art § 10) : nœud de cycle + quotient par bisimulation, hors C1. À ne rouvrir que si un cas d'usage le demande.
- **DSL textuel + outillage éditeur** (si la surface est textuelle, décision § 10) : workstream propre, à lancer après M5-1 : spécification de la syntaxe concrète (données : déclarations record/union/list/set/map ; opérations : transductions, plis), parseur (`lalrpop` ou `chumsky`), serveur LSP (`tower-lsp`), linter (diagnostics = typage et admissibilité, spec § 3, déjà spécifiés), complétion (résolution de noms + types), coloration et extension VS Code (grammaire tree-sitter, TextMate en v0). Outiller une syntaxe avant sa stabilisation, c'est payer la dette deux fois ; rien dans M0-M5 n'exige la surface textuelle (les jalons se pilotent par l'API). Le parseur PNML de M5-2 est indépendant et n'attend pas celui-ci.
- **L'atelier didactique (l'éditeur graphique des chemins)** (demande du 10 novembre 2026) : partir d'un modèle et de **chemins de son espace d'états** (un petit modèle entier, ou des traces tirées d'un grand), affichés nus, puis **activer/désactiver chaque réduction une à une** (hash-consing, merge, collapse de l'absent, forme relative/ancre, shift, pli) et **jouer les paramètres du contrôleur** (ordre, encodage absolu/relatif, royaume, ancre composée) : le diagramme et son compte de nœuds se recalculent sous l'œil, l'espace d'états se rétracte en direct. Le but est didactique : rendre manipulable la thèse « le diagramme est une fonction du contrôleur », chaque réduction payée visible une à une (les 7 contre 4n − 4 des philosophes, le 4 contre l'infini de la fenêtre, le gaucher 1,2887 à l'œil nu). C'est la navette à échelle humaine : l'humain tient le rôle du LLM (hypothèses d'ordre, d'ancrage), le moteur dispose et mesure ; c'est aussi le socle des démonstrations (table Rennes, Fête de la Science, MOOC, marque-page du tissage, anneau 3). Dépendances : aucune pièce nouvelle (l'API du cœur, le port `Observer` de Liseuse, un front web autonome) ; contrairement à Liseuse (lecture seule, elle rejoue des traces), l'atelier **écrit** le contrôleur et redemande : il se lance donc après M3-4 (l'ordre et la fiche contrôleur existent), les deux exemples fil rouge servant de bancs étalons.

### Pistes inter-projets (séance d'ouverture, 10 novembre 2026)

Huit directions qui croisent Lisage, petrichor et layered-data, classées par proximité avec le plan courant. Chacune branche au moins deux projets et réutilise une pièce déjà prévue ; aucune ne touche au pool.

- **Le cartomancien** : l'oracle lit le modèle. Le LLM (la lecture sémantique de petrichor) parcourt noms et structure du réseau et propose une configuration de contrôleur (ordre, encodage, compilations Set/Map, royaume) ; l'oracle analytique mesure, le LLM devine, le différentiel corrige (M3-3, C3, C4). Falsifiable : le LLM ne prédit pas mieux que le seuil heuristique, exactement la clause C4. Attaque de front le problème historique de l'ordonnancement des variables.
- **La navette** : le LLM propose, le diagramme dispose. Le LLM émet des hypothèses (symétries, familles de réplicas, invariants), le moteur les vérifie par évaluation, et les symétries prouvées activent le shift (C8, M4-2). La correction ne quitte jamais le moteur : le LLM est générateur d'hypothèses à confiance nulle. Boucle : deviner, vérifier, exploiter, mesurer dans Liseuse.
- **Le métier à contrôleurs** : chaque couche de layered-data porte son contrat de compilation vers le pool ; le contrôleur se déduit du formalisme. La tour de contrôleurs (motif d'instance de la proposition) devient un théorème des couches, et l'installation des SDD sententiels (M5-3) en est le test exécutable : écrire la couche SDD, dériver le contrôleur, zéro extension *ad hoc*.
- **L'envers du tissu** : les contre-exemples racontés. Le point fixe contient le contre-exemple ; extraire un chemin tiré minimal (re-explicitation d'un seul chemin, le prix de A6 payé sur une seule branche), puis le LLM le raconte dans le vocabulaire du modélisateur (noms, intentions). Le diagnostic assisté complète les analyses statiques de petrichor par le récit dynamique.
- **Le velours** : la veine δ continu racontée en produit : réseaux temporels (clés DBM, clôture convexe connue), pli en intégrale (question ouverte 9), stochastique (semi-anneaux de densités). Cible QEST+FORMATS, strictement après M5-4, conformément au backlog.
- **La broderie quantique** : au-delà du merge non idempotent de M5-4, des semi-anneaux d'amplitudes complexes ; l'état de l'art a les témoins (LIMDD, Q-Sylvan). Piste longue, à ne lancer que si un cas d'usage arrive.
- **Le faussaire** : un LLM qui forge des modèles plausibles et méchants (fausses symétries, réplicas presque parfaits, noms menteurs) pour falsifier l'oracle à échelle (C4) et éprouver la lecture sémantique de petrichor contre des noms qui mentent. Source de familles de benches au-delà de MCC ; tournoi oracle contre faussaire comme métrique.
- **La remontée** : de l'espace d'états vers le modèle. La synthèse de réseaux (théorie des régions) assistée par LLM : navette appliquée à l'envers, le réseau synthétisé s'écrit en couches layered-data et se vérifie par différentiel contre l'espace d'origine.

### Panorama élargi (horizons, 10 novembre 2026)

Hors du trio, par anneaux. Rien ne touche au sprint Rennes : ce sont des horizons, pas du plan.

**Anneau 1, d'autres étoffes sur le même métier** (domaines d'application) :
- **Process mining** : journaux d'événements → réseaux → espaces d'états, conformance checking sémantique (petrichor lit les noms d'activités) ; le CFP de Petri Nets liste explicitement process discovery et conformance checking ; passerelle industrielle immédiate.
- **Biologie à règles** (Kappa, réseaux métaboliques) : l'explosion combinatoire des règles est l'explosion d'états dans sa forme pure ; les familles de réplicas de la navette sont les symétries d'agents ; semi-anneaux stochastiques ; communauté CMSB.
- **EDA** : le berceau historique des DD (équivalence formelle, vérification de propriétés) ; rouvert comme domaine d'application, pas comme venue (l'arbitrage § 11 tient).
- **Planification et jeux symboliques** : MDP/POMDP à facteurs (lignée SPUDD : ADD sur facteurs), résolution symbolique de jeux ; le semi-anneau probabiliste de M5 est la porte.

**Anneau 2, d'autres métiers qui tissent pareil** (ponts interdisciplinaires) :
- **Circuits probabilistes** : PSDD et model counting pondéré sont déjà dans le plan (M5-3) ; l'horizon est la communauté des tractable probabilistic models (UAI, NeurIPS) : un moteur, des semi-anneaux.
- **Bases de données factorisées** : la factorisation d'Olteanu (évaluation de requêtes sur décompositions) rime structurellement avec vtree et contrôleur (d-tree ≈ ordre structuré) ; le planner de M3 est un planificateur de factorisation ; communauté PODS/SIGMOD. La rime la plus profonde et la moins vue.
- **E-graphes et equality saturation** : l'egg de la communauté PLDI (en Rust) est le cousin structurel du pool saturé à hash-consing ; la saturation de réécriture et la saturation de Ciardo sont deux saturations ; terrain commun à creuser.
- **Grammaires attribuées et transducteurs d'arbres** : la maison théorique du cadre (hérité/synthétisé = attribute grammars cycliques et partagées) ; et sa conséquence la plus académique : **le cœur certifié**, C1/L3/A5 mécanisés en Lean ou vérifiés en Creusot/Kani, une bibliothèque DD certifiée n'existe pas encore.

**Anneau 3, la fabrique et la ville** (industrie et société) :
- **L'auditeur symbolique** : la traçabilité event-sourced de Liseuse + le récit du LLM = rapports de vérification explicables pour la certification (DO-178C, IEC 61508) ; le contre-exemple raconté devient pièce d'audit.
- **Jumeaux numériques et MBSE** : les formalismes en couches comme formalisation vivante de SysML ; simulation et analyse de sûreté des contrôleurs industriels sur le même moteur.
- **Le tissage des espaces d'états** (précisé le 10 novembre 2026 ; remplace l'entrée « musée Jacquard »). Un tissu réel, pas une métaphore : le carton perforé est une structure de décision, et la chaîne/trame sont déjà la grille de lecture du cadre (hérité/synthétisé). Protocole : **chaîne** = variables dans l'ordre O (l'intercalage record), **trame** = états, **point** = valuation (satin ou son inversé selon 1/0), **état inatteignable** = absence dans la trame, **forme partagée** = motif répété, **gaucher** = faille du motif (l'asymétrie 1,2887 visible à l'œil nu). Trois objets des philosophes, à 10 trames/cm : n = 3, 152 états : un marque-page de 15 cm ; n = 4, 23 168 états : une bannière de 23 m ; n = 5, 536 754 176 états : **537 km d'étoffe explicite**, plus que Paris-Lyon à vol d'oiseau, alors que la même information tient en 7 motifs, une serviette. La paire « 537 km contre une serviette » rend C8 physique : l'explosion d'états est une étoffe interminable, le partage est un jeu de cartons. Réalisabilité : les métiers jacquard numériques des écoles d'art textile se pilotent par simple fichier image ; le pipeline est Lisage calcule, Liseuse exporte le carton (un renderer bitmap de plus après l'export SVG/PGF de M5-5), le métier tisse (à 1 fil par variable le tissu est un ruban ; pour une étoffe large, chaque variable devient une bande). Services en ligne relevés le 10 novembre 2026 : trois familles. (1) Grand public tissé depuis image (jacquard réel, pas impression) : WOVNS et Jacquard Works (tissu au mètre, MOQ 50 m : la bannière n = 4 de 23 m rentre dedans), FiberArt, WovenPhotos, PictureWeave ; Prodigi expose la couverture jacquard en API print-on-demand, donc le pipeline Liseuse → carton → objet tissé peut être automatisé de bout en bout. Limite : ces services tissent des pixels (luminosité → point), pas des armures. (2) Labs de commissionnement : TextielLab Tilburg (métiers jacquard, appels à projets ouverts, collaboration à distance documentée ; le contrôle de l'armure, donc la valuation 1/0 fil par fil et la faille du gaucher, vit à ce niveau), ateliers indépendants (portraits jacquard sur commande). (3) Outillage libre : AdaCAD (CAD jacquard open source, Unstable Design Lab), export TC2 documenté ; intégrable à Liseuse comme une cible de rendu de plus. Partenaires à sonder : une école de tissage lyonnaise, le Musée des Tissus, Le 19M ; l'objet peut voyager : marque-page sur la table de démonstration à l'exposition d'outils, Fête de la Science, MOOC en habillage. Post-sprint par construction : sa seule dépendance d'ingénierie est un renderer.

**Anneau 4, le miroir** (la méthode elle-même) :
- **Le protocole saucisson comme objet de recherche** : développement piloté par agents (dev + relecteur autonomes, humain à la frontière du jour) est une méthodologie neuve dont ce projet est l'expérience : mesurer, publier (communautés AI4SE, ICSE-SEIP), et la construire de Lisage par Lisage, réflexivement.

## 10. Audit de complétude : points à élucider pendant le développement (2026-11-10)

Verdict de l'audit : M0 et M1 sont spécifiés jusqu'au niveau de l'issue (énoncés, critères de sortie, différentiel), les questions ouvertes 1-9 de la spec sont toutes affectées (Q1 → M3, Q2 → M2 à mesurer, Q3 → M1 admissibilité, Q4 → M1-3 vérification, Q5 → M2-1, Q6 → M4-2, Q7 → M4-3, Q8 → M5-4, Q9 → backlog). Restent sept décisions d'**ingénierie** non tranchées, absentes des documents ; aucune ne bloque M0, chacune est affectée au jalon où la mesurer est possible.

| Point | Question ouverte | Jalon | Décision proposée |
|---|---|---|---|
| Concurrence de l'interface | bounds `Send`/`Sync` du pool et du computed-cache : les figer dans les signatures dès M1-0 (phase d'interface), l'exécution restant mono-thread | M1-0 | figer maintenant (le rétrofit d'une interface mono-thread coûte plus cher que la décision) ; parallélisme effectif repoussé au-delà de M5, Sylvan multi-cœur servant de témoin |
| Clé absente dans `lookup` | spec Q3 : l'`Option` du résultat comme algèbre de défauts, greffon constant sur continuations | M1-5 | rendre le cas explicite dans l'issue (arc absent collapsable, à intégrer à l'admissibilité § 3.3) |
| FFI CUDD/Sylvan | liaisons C/C++ (compilation, conteneurs CI, plateformes) : risque technique n° 1 de M1-8. Pourquoi des liaisons : l'interprète de référence ne voit que la sémantique, jamais la forme ; la « reproduction à l'identique » (C6) se mesure en nombre de nœuds contre une bibliothèque indépendante (un mini-BDD maison partagerait les malentendus du même auteur), et Sylvan ajoute un second témoin indépendant portant saturation et LDD. Le hors-processus ne supprime pas le risque : il faut toujours compiler les bibliothèques ; l'in-process achète le PBT différentiel (fonction aléatoire → deux constructions → comptes comparés sur place). Les baselines externes conditionnent les mesures publiées (M4-4, M5-2) | M1 | spike dédié dès la première semaine (la vraie question est le build en CI) ; séquencement possible : CUDD seul en M1, Sylvan à M2 quand la saturation (M2-4, à la Hamez) l'exige |
| Politique d'erreurs | invariants internes (`core/`, `algebra/`) = asserts ; erreurs du niveau `lang/` = `Result` | M0-4 puis M1-0 | deux lignes dans la charte de test, reprises comme types dans l'interface du cœur |
| Persistance | format de dump versionné des diagrammes ; les handles sont-ils stables hors session ? | M4 | à spécifier avec le pool bi-plan (M4-1), le format de trace JSONL de Liseuse n'étant pas un format de persistance |
| Arithmétique du semi-anneau probabiliste | rationnels exacts vs `f64` : A5 (pli exact sur construction levée) exige la décision | M5-4 | exact par défaut (rationnels), `f64` en option de bench ; le garde-fou est déjà écrit |
| ADR | figer les choix structurels de M1 (arena, `NodeId`, encodage des formes, Arc vs borrow) | M1-0 | convention `docs/adr/` dès le premier PR de la phase d'interface, importée dans M0-3 avec les documents |
| Surface du langage | le squelette de `lisage-lang/` annonçait un « parser » qu'aucune issue M0-M5 ne livre ; la syntaxe concrète n'est spécifiée nulle part, et l'outillage éditeur (LSP, linter, complétion, coloration VS Code) est absent de tous les jalons | M5-1 | trancher : API hébergée Rust (lint et complétion gratuits via rust-analyzer) ou DSL textuel (outillage en backlog) ; ni l'un ni l'autre ne bloque M0-M5, pilotables par l'API |

Les deux candidats gelés du backlog (δ continu, coinductif) ne bloquent rien par construction : le premier attend l'instance ADD/MTBDD (M5-4), le second un cas d'usage. Les conjectures (C1 minimalité, unfold) ne sont pas des manques : falsifiables par design, elles sont ce que M1 est construit pour tester.

## 11. Plan de publication (stratégie relevée le 10 novembre 2026)

Chaque jalon se termine par une figure ou une mesure publiée depuis Liseuse : le plan de publication épouse donc les jalons, un article par groupe de contributions falsifiées avec succès. Règle : ne soumettre que ce que le différentiel a déjà validé ; la date limite ne justifie jamais de soumettre un énoncé non testé.

| Article | Matériau (jalon) | Contributions | Venue cible | Timing réaliste |
|---|---|---|---|---|
| **PR : le cadre appliqué aux réseaux de Petri (sprint Rennes)** | M0-M1 + le slice saturation de M2 : transductions attribuées, C1, saturation statique par niveaux, philosophes en 7 formes, différentiel CUDD, figures Liseuse v0 ; la chaîne des trois projets (layered-data : formalismes en couches ; petrichor : l'outil réseaux de Petri ; Lisage : le moteur) | C1, T3 (policy statique), C8 (compacité philosophes, sans la part O(1) + bord qui attend M4) | **Petri Nets 2027** (Rennes : résumé 15 janvier, article 22 janvier 2027) ; repli : tool paper (10 p. + exposition d'outils) | janvier 2027 |
| **P0 : la grille de lecture** | sans code : la grande famille (§ 4 de l'état de l'art) + le bestiaire, toute famille = (contrôleur, ancre, forme) | la lecture unifiante, avant falsification | **SYNASC** (track BDD, soumission ~juin, niche et barrière d'entrée naturelles) ; **JFLA 2028** (octobre 2027, en français, pour Σ et le langage) ; version longue en survey après P1 | SYNASC 2027 |
| **P1 : le cœur** | M1-M2 : transductions attribuées (hérité/synthétisé/mémoire), C1, semi-anneaux (A5, L3), `lookup`, différentiel CUDD/Sylvan, figures Liseuse | C1, C2, C7 | **TACAS** (l'audience de la lignée DDD/SDD, fondations + outil) ; repli **ATVA** (~avril) ; **CAV** si l'angle classification de canonicité domine | TACAS 2028 (soumission ~fin octobre 2027) |
| **P2 : localité et saturation** | M4 : shift, D-abs, O(1) + bord, philosophes (7 nœuds, gaucher 1,2887), contre SDD/HSDD | C5, C8 | **Petri Nets 2028** (théorie 20 p., outils 10 p. + exposition : Liseuse y entre comme tool paper) ; **ACSD 2028** (la maison de polyDD 2010) | soumission ~janvier 2028 |
| **P3 : complétude et échelle** | M5 : SDD sententiels sans extension *ad hoc*, oracle sur le corpus MCC, merge ADD/MTBDD | C3, C4, C6, C7 | **CAV** (l'article drapeau de la thèse de complétude) ; les chiffres MCC ont aussi leur public à Petri Nets (le concours y vit) | CAV 2029 (janvier 2029) |
| **Journal** | extension de P1 + P3 | toutes | **STTT** (la maison de CESRBDD) ou **FMSD** (la maison de la génération précédente) | 2029 |
| **Veine δ continu** | backlog, après M5-4 | question ouverte 9 | **QEST+FORMATS** (conjoints, veine temps et stochastique) ; **HSCC** si le fragment hybride s'ouvre | seulement si le backlog s'ouvre |

Dates relevées le 10 novembre 2026, à revérifier à chaque soumission : TACAS 2027 a fermé le 29 octobre 2026 ; CAV 2027 ferme le 20 janvier 2027 (trop tôt pour du code : viable seulement pour une position sans implémentation) ; JFLA 2027 a fermé le 16 octobre 2026 (Dinard) ; Petri Nets 2027 (Rennes, Inria) : résumés 15 janvier, articles 22 janvier 2027 : **la cible du sprint Rennes, ci-dessous** ; HSCC 2027 a fermé le 5 novembre 2026. Révision du 10 novembre 2026 : le « trop tôt pour du code » était calibré sur un rythme humain ; sur la preuve des dépôts récents (`layered-data` complet en quelques jours, `petrichor` scaffoldé le 7 novembre et déjà 65 commits), la première fenêtre avec du code devient Petri Nets 2027, TACAS 2028 restant la cible de l'article de fondations et P0 (SYNASC 2027, ~juin) installant la grille de lecture entre les deux.

### Le sprint Rennes (décision du 10 novembre 2026)

Trois projets engagés : **layered-data** (les formalismes en couches : le réseau de Petri dérivé du socle des hypergraphes, les philosophes comme instance ; déjà livré), **petrichor** (l'outil réseaux de Petri : modèles, arcs, tirages ; jeune, actif), **Lisage** (le moteur d'espaces d'états par DD attribués ; à construire : M0-M1 + le slice saturation de M2). L'article régulier (20 p. LNCS) : les transductions attribuées appliquées aux réseaux de Petri, saturation statique par niveaux, la compacité des philosophes (7 formes), le partage C1 visible dans Liseuse, différentiel CUDD. **Anti-collision des claims** : PR porte l'application (T3 statique, compacité philosophes) ; P1 garde les fondations (C1, C2, C7, `lookup`) ; P2 garde la localité complète (shift, D-abs, O(1) + bord, gaucher), qui attend M4.

Séquence (cycles d'agent, revue humaine au fil de l'eau) : semaines 1-2 : M0 + M1-0 (interface) + interprète de référence + PBT rouges + spike CUDD ; semaines 3-5 : cœur M1 (arena, handles, réductions, transductions hérité/synthétisé, `lookup`, pli), différentiel CUDD vert sur l'instance BDD, C1/L3/A5 tenus ; semaines 6-7 : saturation statique + philosophes n = 3 à 5 via l'intercalage record ; semaine 8 : Liseuse v0, gel des mesures, résumé (15 janvier) ; semaines 9-10 : rédaction depuis les documents, revue, soumission (22 janvier).

Périmètre découpé pour tenir : CUDD seul témoin (Sylvan attend M2-M4) ; philosophes seulement, pas de MCC, pas de `Seq`, pas d'événements non locaux au-delà de l'intercalage, pas de SDD sententiels, pas d'ADD, pas d'oracle (M3-M5 intacts). **Garde-fou (notre règle de soumission) : si le 15 janvier le différentiel n'est pas vert, l'article régulier se retire au profit du tool paper** (10 p. + démonstration à l'exposition d'outils), qui n'exige que la démo philosophes vivante et un accès libre pour les relecteurs. Le calendrier plie ; les énoncés ne plient pas.

Communautés, pour arbitrer : TACAS/CAV/ATVA = vérification orientée outils, là où vit la lignée DDD/SDD ; Petri Nets/ACSD = la maison saturation et la lignée polyDD/HSDD ; SYNASC = la niche BDD ; STTT/FMSD = les journaux qui ont publié CESRBDD et la génération précédente ; JFLA/GDR GPL = le public francophone langages et génie logiciel. DAC/ICCAD/DATE : écartés (audience EDA, style différent), à rouvrir seulement si l'angle compilation de connaissances (PSDD) domine un jour l'agenda.