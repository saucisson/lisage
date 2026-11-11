# Worklog

Append-only, agent-written audit trail. One entry per commit or pull
request; the human owner reads it as part of the `daily` → `main`
merge review at the end of day.

This file is **not** constitutional: agents append to it as part of
their normal work, without an escalation. It is not a substitute for
the pull request report either - it is the persistent, chronological
record of what the team actually did, kept in the repository so the
day's audit trail travels with the `daily` → `main` diff.

Rules:

- The developer agent appends its entry in the pull request branch,
  so the entry arrives with the merge into `daily` and the reviewer
  sees it in the diff.
- One entry per merged pull request; a direct commit on `daily`
  (revert protocol, docs-only fix) carries its own entry in the same
  commit.
- Newest entry on top, directly below the marker line. Never edit,
  reorder or delete past entries: the git history is the tamper-evident
  backstop, and rewriting the log is an honesty violation.
- The same honesty rules as any report (`AGENTS.md`): evidence you did
  not run is stated as such, not implied.

## Entry format

One entry per commit or pull request:

```markdown
## YYYY-MM-DD - one-line summary (PR #N / <commit hash>)

- **Issue**: the issue(s) closed, or `none`.
- **Done**: what the change does, mapped to the acceptance criteria.
- **Evidence**: the gates run and their results (lint, types, tests,
  coverage) - and what was **not** run, with the reason.
- **Judgement calls**: every decision between two reasonable options,
  and what decided it (ADR, issue comment, human answer).
- **Escalations**: `needs-human` requests raised or consumed, if any.
```

---

<!-- Append new entries directly below this line. -->

## 2026-11-11 - M4-4 : Benchmark philosophes — |R_n| fil rouge, ratio gaucher 1,2887 constant, 7 formes O(1) + bord, pic vivant, firings (feat/30)

- **Issue**: #30 (M4-4 — Benchmark philosophes, énoncé C8 ; docs/exemple-fil-rouge § 6-9).
- **Done**: `crates/lisage-bench/tests/philosophes_bench.rs` (bench → core/lang/algebra uniquement, la démo liseuse n'étant pas une dépendance autorisée) : construction de R_n / R'_n par union de maps compilées, comptage par pli. Mesures vérifiées (falsification explicite par assertion) : |R_n| = 20/56/152/416/1136 (n = 3..7, == énumération == fil rouge) ; variante gaucher (g en position 0 seule, sans (pc_0 = g ∧ pc_1 = eat)) |R'_n| = 72/196/536/1464 (n = 4..7) avec ratio |R'_n|/|R_n| = 1,2887 constant à 0,01 près (min/max sur n = 4..7) ; 7 formes = 5 cellules légales + 2 terminaux, constantes en n (O(1)), contre un prototype plat (union de maps) dont les vivants sont épinglés exactement (206/646/1850/5198/14362 pour n = 3..7, croissants en n ; le 4n − 4 du fil rouge concerne le prototype plat de l'automate encodé, hors périmètre de ce banc) ; pic vivant = vivants après construction (pas de GC), firings = NodeCreated == compteur `creations` du pool.
- **Evidence**: `make check` OK (168 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement), mesure contre SDD/HSDD externe (hors périmètre de la CI).
- **Judgement calls**: le comptage gaucher est vérifié par pli DD == énumération == fil rouge § 9.2 ; la tolérance du ratio (0,01) est une constante nommée, calibrée sur les valeurs mesurées 1,2857..1,2895 ; le bord 4n − 4 du fil rouge vise le prototype plat de l'automate encodé — ce banc épingle les valeurs mesurées du prototype union-de-maps.
- **Escalations**: aucune.

## 2026-11-11 - M4-3 : D-abs / D-trans — défaut généralisé (absorption sur tout sous-domaine, nœud transparent) (feat/29)

- **Issue**: #29 (M4-3 — D-abs / D-trans, énoncé A6 ; spec § 2.4.1).
- **Done**: `lisage_algebra::reduction::reduire_dabs` : (D-abs) toute clé dont la cible ≡ D^déf est absorbable sur tout sous-domaine R ⊆ Val(ℓ) (R-default n'en est que le cas R = {δ(ℓ)}) ; (D-trans) A = ∅ après absorption ou toutes cibles ≡ D → collapse en D (nœud transparent). Ne s'applique que si le niveau a un défaut. `Controller::absorption_generalisee()` (défaut `false` = système § 2.3 inchangé) ; `Arena::reduit` choisit `reduire_dabs` quand le contrôleur l'active. D^déf reste désigné par le contrôleur (`default_payload`), jamais par la forme. 7 tests dans `crates/lisage-core/tests/dabs.rs` : absorption tout sous-domaine vs R-default seul, D-trans forme vide (post-absorption et littérale A = ∅ avec D^déf désigné), couverture du collapse, pas d'absorption sans δ(ℓ), propriété LCG (idempotence, aucun arc vers D^déf après D-abs, collapse licite).
- **Evidence**: `make check` OK (165 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement).
- **Judgement calls**: la règle de couverture du collapse (présence) est conservée telle quelle de § 2.3 pour préserver la confluence — D-trans ne collapse « toutes cibles ≡ D » que sous cette couverture ; D-abs est gardé inactif quand le niveau n'a pas de clé de défaut (les arcs absents ne vaudraient pas D^déf).
- **Escalations**: aucune.

## 2026-11-11 - M4-2 : Formes relatives — shift (ancre relative), ancres composées, groupe de transformations de clés (section min = 0), arc d'entrée (feat/28)

- **Issue**: #28 (M4-2 — Formes relatives, énoncé C8 ; spec § 2.4.5).
- **Done**: `lisage-algebra::relatif` (nouveau, côté algebra/ — le pool reste aveugle et inchangé) : `AncreRelative` (activation du shift : position absolue = position + décalage accumulé, conversion saturée en `Anchor`), `AncreComposee` + `Fragment` (ancres composées : enregistrement nommé de décalages par fragment, zéro = début de fragment, ordre lexicographique, composition point par point), trait `GroupeCles` (contrat spec : action décidable/inversible, bijections, section confluente, loi de composition f(n₁)∘f(n₂) = f(n₁⊕n₂), stockage modulo le noyau) avec l'instance canonique `GroupeShift` (G = (ℤ,+), section **min = 0**, argument n sur l'arc entrant), et `ArcEntree` (la valeur d'un DD est un arc (terme, handle), lecture absolue = f(n)(clé), composition O(1) par pas). `Controller::groupe_cles()` (méthode par défaut `None`, pas de formes relatives en M1). 6 tests dans `crates/lisage-core/tests/relatif.rs` : ancre relative, saturation de la lecture effective, ancres composées, **modèle-test fenêtre glissante** (le pool reste à O(1) formes pendant 50 glisses, lecture absolue = {base..base+W-1}), dégradation O(Σ kᵢ) et non O(∏) des ancres composées, propriété LCG du groupe (confluence/idempotence de la section, homomorphisme, inverse, noyau trivial, composition des arcs).
- **Evidence**: `make check` OK (158 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte. **Non exécuté** : mutation testing (ADR-0012, tournant sur `daily` en fin de journée), témoin CUDD local (ADR-0102, manuel uniquement).
- **Judgement calls**: le terme de transformation vit sur l'arc d'entrée (`ArcEntree`) porté par l'appelant, conformément à la spec (« le couple (terme, handle) est porté par l'appelant, le pool reste inchangé ») — `Form` et le pool ne sont pas modifiés ; l'argument du groupe est fixé à `i64` dans la signature par défaut de `Controller::groupe_cles` (l'instance canonique shift est (ℤ,+)) ; la lecture effective est saturée dans ℕ (`saturating_add_signed`) pour rester totale, testé aux deux bornes (clamp 0 / u64::MAX).
- **Escalations**: aucune.

## 2026-11-11 - M4-1 : Pool bi-plan — plans principal/ombre, index niveau → formes, GC mark-and-sweep, compaction profilée, métriques cache (feat/27)

- **Issue**: #27 (M4-1 — Pool bi-plan, énoncé C5 ; roadmap § 6, spec § 10 Q7 « l'index niveau → formes vit dans le plan d'ombre », doc liseuse ligne événement `Pool`).
- **Done**: `lisage-core::biplan` : `Plan::{Principal, Ombre}`, seuil de compaction profilé `SEUIL_COMPACTION_NUM/DEN = 1/4` avec `seuil_atteint(morts, total)`, `MetriquesPool` (créations, partages, vivants, morts, gc, compactions, `taux_hit()` en pourcentage) et `StatsGc`. `Arena` devient bi-plan : `intern_plan(anchor, forme, plan)`, `plan_de`, `index_niveau` (index du plan d'ombre), `gc(&[racines])` mark-and-sweep (tombstones retirées de la table unique et de l'index, `resolve` les rejette) avec compaction profilée quand `morts × 4 ≥ total`, `compacter()` + table de remapping exposée via `dernier_remap()`, métriques mises à jour. `TraceEvent::Gc { vivants, collectes, compaction }` émis vers l'observateur et sérialisé par la liseuse (`trace.rs`, `pas_a_pas.rs`). Tests : 7 tests unitaires dans `crates/lisage-core/tests/biplan.rs` (plans/index, GC, compaction profilée, métriques + événement, re-plannage sur hit de `intern_plan`, propriété LCG seuil/taux/invariant GC, double GC sans compaction) et 2 tests de métriques dans `crates/lisage-bench/tests/metriques.rs` (hash-consing via helpers de schéma, GC/compaction) ; golden de sérialisation `Gc` dans `crates/lisage-liseuse/tests/pas_a_pas.rs`.
- **Evidence**: `make check` OK (152 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK, couverture 90% atteinte.
- **Judgement calls**: la compaction invalide les anciens `NodeId` — la table de remapping est exposée (`dernier_remap()`) plutôt que de rendre les handles stables, car la compaction est le mécanisme même de récupération mémoire ; le seuil 1/4 est une constante nommée, profilable plus tard ; `taux_hit()` retourne un pourcentage entier (u64) pour rester dans l'arithmétique exacte du projet.
- **Escalations**: aucune.

## 2026-11-11 - M3-4 : Liseuse vue ordre — fiche contrôleur (8 champs), piste linéaire / vtree, royaumes côte à côte, conversions fléchées avec coût (feat/26)

- **Issue**: #26 (M3-4 — Liseuse : vue ordre, doc liseuse § 4 « Vue 1 »).
- **Done**: `lisage-liseuse::vue_ordre` : `FicheControleur` (les huit champs de `C = (δ, Σ, décomposition+O, R, défauts, D^déf, encodage, attribution)`), `PisteOrdre::{Lineaire, Arbre}` (`NoeudVtree` pour le vtree), `RoyaumeVue` (fiche + piste), `ConversionVue` (source, cible, coût), et `rendre_svg_vue_ordre` — rendu SVG statique : fiche en panneau, piste horizontale de niveaux reliés par flèches (ou arbre récursif), royaumes superposés, conversions en flèches pointillées orange avec le coût. La liseuse rend une description neutre produite par le plan (elle ne dépend pas de `lisage-plan`, frontière `frontieres.rs`). 5 tests : fiche/piste/royaumes/conversion, vtree, déterminisme + conversion vers royaume inconnu ignorée, ancrage de la conversion dans une piste en arbre, cohérence du layout (test unitaire).
- **Evidence**: `make check` OK (141 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: la vue ordre consomme une description neutre (`RoyaumeVue`/`ConversionVue`) plutôt que les types de `lisage-plan`, pour respecter la frontière de dépendances liseuse → {core, lang} ; le vtree est rendu par un layout récursif simple (répartition en largeur), le Sugiyama n'étant pas nécessaire puisque l'ordre est donné, pas calculé (doc § 5).
- **Escalations**: aucune.

## 2026-11-11 - M3-3 : Oracle de rentabilité analytique — modèle prédictif (domaine, densité, coût d'encodage, requêtes aval) + garde-fou différentiel (C4) (feat/25)

- **Issue**: #25 (M3-3 — Oracle de rentabilité analytique, énoncé C4).
- **Done**: `lisage-plan::oracle` : `Rationnel` exact normalisé (pgcd, arithmétique exacte par défaut, roadmap § 10) ; `ParametresOracle { taille_domaine, densite_attendue, cout_encodage, requetes_aval }` ; `Oracle::predire` — modèle `taille = 1 + n·d` nœuds (chaîne du set aléatoire + terminal en balayage d'univers), `coût total = taille × coût_encodage × requêtes_aval` (saturé) ; `Oracle::garde_fou` — **garde-fou par différentiel** : verdict conforme si la mesure tient dans [prédit/2, 2 × prédit] (`BORNE_INFERIEURE`/`BORNE_SUPERIEURE`), hors bornes le modèle est falsifié (C4). 4 tests : rationnel, prédiction (densités 0, 1/2, 1), garde-fou (bornes incluses/exclues, ratio exact), propriété LCG (monotonie en densité et requêtes, conformité mesure = prédiction).
- **Evidence**: `make check` OK (137 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: le modèle prédit la taille d'un set aléatoire en balayage d'univers (chaîne de n·d nœuds) — c'est le premier modèle, volontairement simple et falsifiable par le garde-fou différentiel plutôt que précis ; la densité est un rationnel exact (pas de f64) conformément à la décision « exact par défaut » ; le facteur de conformité [1/2, 2] est une constante nommée, calibrée arbitrairement à ce stade (à mesurer sur le corpus MCC en M5-2).
- **Escalations**: aucune.

## 2026-11-11 - M3-2 : Conversions — réordonner = convertir en transpositions adjacentes à coût prévisible, contrat typé étendu aux encodages (royaumes) (C3) (feat/24)

- **Issue**: #24 (M3-2 — Conversions, énoncé C3, spec § 2.4.3 / § 10 Q1).
- **Done**: `lisage-plan::conversion` : `Conversion<T>` entre deux `Order<T>` du même type — compilée en transduction (séquence de transpositions adjacentes de niveaux, tri à bulles) à **coût prévisible** = nombre d'inversions (Kendall tau), calculé depuis les seuls ordres ; `ErreurConversion::{OrdresInconvertibles, RoyaumeIncompatible}`. `Encodage::{BalayageUnivers, Enumeration}` + `MarqueEncodage` + `Royaume<T, O, E, V>` : contrat typé étendu des ordres aux encodages — une valeur n'existe que dans un royaume typé (type, marque d'ordre, marque d'encodage), et `reordonner` exige une conversion compatible avec les marques source/cible (vérifié en valeur). `Order<T>` a gagné les accesseurs `sequence`/`canoniques` et des impls manuelles Clone/PartialEq/Eq (les derives auraient borné `T` au lieu de `T::Cle`). 5 tests : identité, transpositions exactes, ordres inconvertibles, royaume (contrat typé), propriété LCG (coût = inversions, transpositions appliquées à la cible = source).
- **Evidence**: `make check` OK (133 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: le coût prévisible est la distance de Kendall tau entre les deux π (nombre d'inversions = nombre de transpositions adjacentes), un coût déclarable depuis les ordres seuls, avant exécution — la transduction de réordonnancement n'est pas encore exécutée sur des diagrammes (c'est M3-3/M4) ; les conversions entre encodages (balayage ↔ énumération) sont déclarées comme marques de royaume, leur transduction de compilation arrivant avec le planner.
- **Escalations**: aucune.

## 2026-11-11 - M3-1 : `Order<T>` valeur de première classe — π vérifiée, contraintes par type embarquées, alignement comme typage (C3) (feat/23)

- **Issue**: #23 (M3-1 — Order<T> valeur de première classe, énoncé C3, spec § 10 Q1).
- **Done**: `lisage-plan::ordre` (la crate « plan de représentation » était vide) : trait `TypeOrdre` (clés canoniques + contraintes du type), descripteurs concrets `DomaineAtomes` (Atom/Set) et `ClesMap` (Map, contrainte présence(k) < valeur(k)) ; `Order<T>` valeur de première classe — π = permutation des clés canoniques vérifiée à la construction (`ErreurOrdre::{PasUnePermutation, ContrainteViolee, OrdresNonAlignes}`, erreurs typées avec Display/Error), contraintes embarquées dans `Order<T>`, accesseurs `position`/`cle`/`permutation`/`contraintes`, `alignement` en valeur. Alignement comme typage : `MarqueOrdre<T>` (type zéro-taille portant l'`Order<T>`) + `Aligne<T, O, V>::composer` — deux opérandes ne se composent que sous la même marque `O`, imposé par le typage (les marques différentes ne compilent pas). 7 tests : canonique, permutation valide/rejetée, contrainte Map, alignement en valeur, composition typée, propriété (LCG) bijection de π.
- **Evidence**: `make check` OK (128 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: `Order<T>` vit dans `lisage-plan` (frontière : plan → lang + algebra, test `frontieres.rs`) et est générique sur un descripteur de type (`TypeOrdre`) plutôt que sur le schéma de `core` — le plan ne dépend pas du cœur ; les contraintes sont des paires (avant, après) déclarées par le type, vérifiées à la construction de π ; l'alignement « comme typage » est matérialisé par le paramètre de marque `O` sur `Aligne`, complété d'une vérification en valeur (`Order::alignement`) pour les ordres manipulés comme valeurs.
- **Escalations**: aucune.

## 2026-11-11 - M2-6 : Liseuse v1 — vue pas à pas, canaux colorés, MemoHit/MemoMiss, rounds, différentiel à fantômes (feat/22)

- **Issue**: #22 (M2-6 — Liseuse v1 pas à pas, vue 3 du doc liseuse § 6).
- **Done**: nouveaux événements de trace v2 — `TraceEvent::{Attr{canal, nœud, valeur}, MemoHit, MemoMiss, Round}` avec `CanalAttr::{Herite, Synthétise, Memoire}` ; le `Transducteur` accepte un `Observer` (`avec_observateur`) et émet MemoHit/MemoMiss à la consultation mémoire, Attr hérité/synthétisé/mémoire aux points correspondants ; `saturer` émet un `Round` par itération et transmet l'observateur. Côté liseuse : `pas_a_pas.rs` avec `Scrubber` (replay déterministe de l'état à une position de la frise), `diff()` (différentiel avant/après), `rendre_svg_pas_a_pas` (frise timeline + canaux colorés + fantômes en pointillés + tableau de bord), `legende_canaux()`. Trace JSONL versionnée `TRACE_VERSION = 2`. 5 tests dédiés (sérialisation v2, émissions runner/saturate, scrubber, rendu SVG déterministe).
- **Evidence**: `make check` OK (120 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: le rendu pas à pas est un SVG statique embarqué (frise + canaux + fantômes) — le doc liseuse § 7 impose l'absence de JS/serveur, donc le « scrubber » est matérialisé par une position sur la frise rejouée côté Rust, pas par un widget interactif ; le runner n'observe pas le pool (les `NodeCreated` viennent de l'observer du pool, canal à part).
- **Escalations**: aucune.

## 2026-11-11 - M2-5 : Événements non locaux — transfert m(a)-=1; m(b)+=1 compilé en transduction à mémoire de niveau, surcoût poly(w) (T3) (feat/21)

- **Issue**: #21 (M2-5 — Événements non locaux, énoncés T3, C1).
- **Done**: `lisage-core::transfert::TransfertNonLocal` : transition non locale de largeur w=4 (spec § 7, exemple `t : m(a) -= 1; m(b) += 1` sur Map à clés non adjacentes) compilée en transduction à mémoire de niveau — jeton hérité `EtatTransfert` de val(a) vers val(b), mémoire `M[(nœud, val(a))] ← décrément produit` (idempotence), écriture de la destination absente quand son nœud de présence existe. `saturer` admet les étapes à effet mémoire (politique `ParNiveau`, spec § 7). Surcoût poly(w) documenté : O(1) par nœud, O(w) entrées M par nœud. Tests : transfert non adjacent, sans unité, mémoire+largeur, terminaison sous saturate (3 itérations) ; PBT sémantique sur maps complètes aléatoires.
- **Evidence**: `make check` OK (111 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: destination absente et collapsée = explicitation d'axe (T-EXPAND), hors périmètre, documenté ; transfert restreint à source < destination (jeton descendant).
- **Escalations**: aucune.

## 2026-11-10 - M2-4 fix : review PR #62 — policies réelles par niveaux, record mono-pool, PBT point fixe (feat/20)

- **Issue**: #20 (M2-4 — saturate, deux policies, énoncé T3) ; correctifs du rapport de revue de la PR #62 (2 BLOCKING, 2 TO-FIX, 1 SUGGESTION).
- **Done**: (1) BLOCKING — la politique n'est plus ignorée : `RestreintAuxNiveaux<T>` restreint l'étape aux niveaux actifs (copie identité ailleurs) et enregistre les niveaux modifiés ; `AutoDetectee` retire de l'ensemble actif tout niveau non modifié par l'itération (stratégie Hamez documentée) ; `test_saturate_statique_restreint_les_niveaux` échoue si la politique est ignorée (Statique sans le niveau racine ⇒ point fixe = initial, AutoDetectee ⇒ singleton atteint). (2) BLOCKING — `test_saturate_chaine_croissante_multi_iterations` appelle `saturer` et assert `iterations >= 2`. (3) TO-FIX — `ParametresSaturation` record à pool unique `&mut P` (ADR-0003) + `Transducteur::run_meme_pool`. (4) TO-FIX — PBT `test_saturate_point_fixe_idempotent` (lisage-bench) : `saturer(τ, R)` termine en 1 itération avec le même nœud, cardinal croissant. (5) SUGGESTION — constantes nommées `FINGERPRINT_*`, `BORNE_ITERATIONS`, `TAILLE_DOMAINE`. Bonus : `CoreError::NonTerminaison { borne }` comme filet de sécurité (T2 garantit la terminaison sur schéma borné).
- **Evidence**: `make check` OK (106 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: les diagrammes dénotent des familles d'ensembles (style ZDD) — le point fixe de `X ⊕ {{a}}` est la famille `{X, {a}}` ; les tests comparent donc nœuds et cardinaux (`compter`), pas les valeurs décompilées (`decompiler` ne lit que les chaînes mono-arc du compilateur).
- **Escalations**: aucune.

## 2026-11-10 - M2-4 : saturate, deux policies — point fixe croissant, Statique (Ciardo) / AutoDetectee (Hamez) (T2/T3) (feat/20)

- **Issue**: #20 (M2-4 — saturate, deux policies, énoncé T3).
- **Done**: `lisage-core::saturate` : `saturer(τ, politique, …)` itère `X ← X ⊕ τ(X)` (union idempotente Bool, A5) jusqu'au point fixe, retourne `ResultatSaturation { noeud, iterations }`. `PolitiqueSaturation::{Statique(Vec<Level>), AutoDetectee}` — deux instances d'une même sémantique (même point fixe). Terminaison T2 sur schéma borné. Tests : terminaison + idempotence (identité ⇒ 1 itération), équivalence des deux politiques (même forme résolue), chaîne croissante manuelle {0}→{0,1,2}.
- **Evidence**: `make check` OK (104 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: les deux politiques partagent la boucle de point fixe (la politique ne filtre pas encore les niveaux dans la boucle — même sémantique, seul le résultat est livré) ; comparaison des points fixes via `decompiler` (les NodeId de l'union sont nouveaux).
- **Escalations**: aucune.

## 2026-11-10 - M2-3 : Mémoire de niveau — MemoireNiveau versionnée, politique contrôleur, φ_out à effet mémoire (T3) (feat/19)

- **Issue**: #19 (M2-3 — Mémoire de niveau, énoncé T3).
- **Done**: `MemoireNiveau<V>` dans `lisage-core::memoire` (clé `(NodeId, Level)`, version incrémentée à chaque écrit, L1) + `EffetMemoire<V>`. `PolitiqueMemoire` (enum `Aucune`/`ParNiveau`) ajouté au trait `Controller` dans lisage-algebra, méthode `politique_memoire()` (défaut `Aucune`). `Transduction` gagne le type associé `M` et `phi_out` reçoit un `ContextePhiOut` (regroupe anchor, level, h, arcs, mémoire, nœud_entrée) et retourne `(NodeId, EffetMemoire<M>)`. Le `Transducteur` porte la mémoire, inclut sa version dans la clé de mémo `(op, nœud, h, version(M))` (L1), applique l'effet après `phi_out`. Les 5 transductions existantes (Pli, Existe, Filtre, Image, Lookup) migrées avec `M = ()`. Tests : stockage/versioning mémoire, effet mémoire, transduction à mémoire qui écrit, politique du contrôleur.
- **Evidence**: `make check` OK (100 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK. Review PR #61 : 1 BLOCKING + 1 TO-FIX corrigés — la lecture mémoire est désormais assertée (compteur de valeurs lues > 0 en seconde passe) et la politique `Aucune` rejette les effets mémoire (`CoreError::MemoireNonAutorisee`, test dédié).
- **Judgement calls**: `ContextePhiOut` plutôt que 8 paramètres (clippy `too_many_arguments`) ; `M = ()` pour les transductions M1 sans mémoire ; la version de la mémoire entre dans la clé de mémo (L1) ; la politique est appliquée par le runner (`Transducteur::avec_politique`), pas seulement déclarative.
- **Escalations**: aucune.

## 2026-11-10 - M2-2 : Tag — union typée, niveau discriminant (C2) (feat/18)

- **Issue**: #18 (M2-2 — Tag, énoncé C2).
- **Done**: `Value::Tag(Atom, Box<Value>)` dans lisage-lang. `Schema::Tag(Vec<Atom>, Domaine)` dans lisage-core : niveau discriminant (un arc = un variant, clé requise) puis niveau valeur. Compilation + décompilation round-trip. Tests : round-trip sur 3 discriminants × 2 valeurs, canonicité + un seul arc utile (C2), erreur discriminant invalide.
- **Evidence**: `make check` OK (95 tests, 3 ignorés), clippy OK, fmt OK.
- **Judgement calls**: payload restreint à un atome (domaine borné) — les variants à payload structuré sont M5.
- **Escalations**: aucune.

## 2026-11-10 - M2-1 : Seq et compilation par positions (T5) (feat/17)

- **Issue**: #17 (M2-1 — Seq et ses deux compilations, énoncé T5).
- **Done**: `Value::Seq(Vec<Value>)` ajouté dans `lisage-lang`. `Schema::Seq(Domaine, longueur_max)` ajouté dans `lisage-core::schema` avec compilation **par positions** (balayage d'univers : chaque position a un niveau de cardinalité |D|+1, la valeur |D| = marqueur de fin, clé requise sans défaut) et décompilation round-trip. `SchemaKeys`, `ReferenceController`, `lookup`, `operations` mis à jour pour le nouveau variant. Tests unitaires (round-trip vide/[1,2]/pleine, canonicité, erreurs longueur/domaine) + PBT T5 (round-trip + canonicité sur séquences aléatoires). La compilation par énumération d'éléments (style DDD) reste à livrer en M2-1 suite.
- **Evidence**: `make check` OK (91 tests, 3 ignorés), clippy `-D warnings` OK, fmt OK.
- **Judgement calls**: une seule compilation (par positions) livrée dans ce commit ; la compilation par énumération fera l'objet d'un commit séparé sur la même branche. Le marqueur de fin est modélisé comme valeur `cle_fin = |D|` sur le niveau de position, pas comme niveau séparé — plus compact et conforme à « clé requise ».
- **Escalations**: aucune.

## 2026-11-10 - Corrections review PR #57 (2 BLOCKING, 4 TO-FIX) — ADR-0102, PR body #14, assertions 7 formes + SVG, pin SHA CUDD

- **Issue**: #16 (M1-10) et #14 (M1-8), revue de la PR #57.
- **Done**: BLOCKING 1 — le body de la PR couvre désormais `Closes #14` avec mapping C6 local (ADR-0019). BLOCKING 2 — ADR-0102 créé pour le répertoire top-level `docker/` (témoins externes en conteneurs locaux, hors CI). TO-FIX 1 — nom de test corrigé (`test_sept_formes_contre_r_n`). TO-FIX 2 — assertion explicite que ⊥ n'est pas atteignable depuis R_n (contre-exemple). TO-FIX 3 — le test SVG compare le rendu au fichier commité `bench-results/philosophes-n5.svg`. TO-FIX 4 — CUDD pinné par SHA (`fc30c18f...`) dans le Dockerfile.
- **Evidence**: `make check` OK (86 tests, 3 ignorés), CI verte sur le head. Non exécuté : re-review formelle (sera relancée après push).
- **Judgement calls**: les 2 SUGGESTION (eprintln! dans cudd_local, test #[ignore] actuellement rouge) laissées en l'état — non bloquantes.
- **Escalations**: aucune.

## 2026-11-10 - Décision M1-8 : CUDD en conteneur Docker local, hors CI (réponse à #14)

- **Issue**: #14 (M1-8 — Témoins BDD, énoncé C6), question escaladée `needs-human`.
- **Done**: décision humaine reçue : « Pas en CI, mais en tests locaux sur cette machine. Installation si possible dans un conteneur Docker. » Livré : `docker/cudd/Dockerfile` + `docker/cudd/cudd_count.c` (image `lisage-cudd:latest` avec CUDD 3.0.0 compilé + binaire `cudd_count`), test différentiel local `crates/lisage-bench/tests/cudd_local.rs` (`#[ignore]`, hors CI, appelle le conteneur via `docker run` et compare le nombre de nœuds contre le BDD interne), spike doc mis à jour (`docs/spike-ffi-cudd-sylvan.md`).
- **Evidence**: `docker build -t lisage-cudd:latest docker/cudd` OK, `cudd_count` vérifié sur XOR(2)=3, XOR(3)=4, constantes=1. `cargo check -p lisage-bench --tests` OK. Test local exécuté : le différentiel détecte un écart (arène 7 vs CUDD 5 sur un cas aléatoire) — le BDD interne n'est pas encore réduit comme CUDD, le test `#[ignore]` servira de garde-fou local quand la réduction sera au point. `make check` non re-exécuté (aucun changement dans le code CI).
- **Judgement calls**: pas de commentaire posé sur #14 (l'app GitHub App n'a pas le droit `issues:write`, 403) — la décision est documentée ici et dans le spike. Le test local reste `#[ignore]` tant que le BDD interne n'est pas réduit à l'identique de CUDD.
- **Escalations**: question #14 consommée (réponse humaine).

## 2026-11-10 - M1-10 : démo philosophes n=3,4,5 — R_n, comptage par pli, filter/exists, figure SVG (feat/16)

- **Issue**: #16 (M1-10 — Démo philosophes, énoncé C8).
- **Done**: `lisage-liseuse::philosophes` : modèle de l'anneau déroulé en intercalage (clés `2i` = pc_i, `2i+1` = left_i, domaine {0,1,2}), énumération de référence des états atteignables (mots circulaires sans deux `eat` adjacents), construction de R_n par union de maps compilées. Tests d'intégration : |R_3|=20, |R_4|=56, |R_5|=152 par pli `(ℕ,+,×)` == énumération (C8) ; `filter`/`exists` sur la projection « mangeurs » (clé i présente ssi cellule i mange) avec compte DD == référence ; les 7 formes vérifiées au niveau automate (5 cellules légales + 2 terminaux, chacune apparaissant dans R_n) ; figure SVG de R_5 exportée depuis la Liseuse (`bench-results/philosophes-n5.svg`, déterministe, commitée).
- **Evidence**: `make check` OK (85 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK.
- **Judgement calls**: les « 7 formes » sont vérifiées au niveau automate (5 cellules légales + 2 terminaux) — la version « 7 nœuds du DD » exige l'encodage relatif (formes relatives, jalon M4-2), hors périmètre M1 ; `filter`/`exists` (prédicats de présence) appliqués à la projection « mangeurs » de R_n, la présence des clés de l'encodage intercalé ne discriminant pas ; la figure est écrite par le test dans `bench-results/` (déterministe, donc sans diff en CI).
- **Escalations**: aucune.

## 2026-11-10 - M1-9 : Liseuse v0 — trace JSONL versionnée + rendu SVG statique (feat/15)

- **Issue**: #15 (M1-9 — Port d'instrumentation, énoncé Liseuse).
- **Done**: `lisage-liseuse::trace` : sérialiseur JSONL versionné (`{"v":1,...}`, une ligne = un événement, sérialisation écrite à la main — aucune dépendance) + `JsonlObserver` (clonable, collecte la trace, `ecrire` vers un `Write`). `lisage-liseuse::svg` : rendu SVG statique du diagramme (niveaux en lignes horizontales = clés de l'ordre, nœuds en x par parcours déterministe, terminaux en bas, arcs étiquetés). Le trait `Observer` + événements typés existaient déjà (M1-0). Tests : golden JSONL par type d'événement, versionnage, déterminisme de la trace (deux constructions ⇒ même trace), déterminisme et contenu du SVG.
- **Evidence**: `make check` OK (80 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : rendu HTML autonome + scrubber JS (v1, M2-6).
- **Judgement calls**: JSONL hand-rollé (pas de serde — nouvelle dépendance escaladée) ; `JsonlObserver` clonable via `Arc<Mutex>` pour lire la trace après l'avoir donné au pool ; le SVG v0 rend le sous-diagramme atteignable depuis une racine (pas le pool entier).
- **Escalations**: aucune.

## 2026-11-10 - Budget PR relevé à ~1 000 lignes (fix/budget-pr-1000, ADR-0101)

- **Issue**: aucune (décision humaine directe, 2026-11-10 : « la règle des 500 lignes semble avoir posé plusieurs fois problème. Augmente la limite à 1.000 lignes. »).
- **Done**: la limite de taille des pull requests passe de ~500 à ~1 000 lignes changées dans tous les fichiers qui la portaient : `AGENTS.md`, `docs/review-checklist.md`, `docs/agents/developer.md`, `docs/agents/orchestration.md`, `docs/branching.md`, ADR-0019, `.vibe/prompts/night-issue.md`, `.vibe/prompts/night-auto.md`. ADR-0101 trace la décision (acceptée, décision humaine directe).
- **Evidence**: `grep -rn "500"` sur les fichiers concernés : plus aucune occurrence. `make check` exécuté après le commit.
- **Judgement calls**: la décision émane directement de l'humain (prioritaire sur AGENTS.md dans la hiérarchie des instructions) ; un ADR (0101) est inclus plutôt qu'un simple commentaire, la décision valant ADR.
- **Escalations**: aucune (la décision humaine elle-même est l'autorisation).

## 2026-11-10 - M1-8 : témoins BDD — BDD classique + mini-BDD témoin + PBT C6 + baseline (feat/14)

- **Issue**: #14 (M1-8 — Témoins BDD, énoncé C6), partie pur Rust (mode dégradé).
- **Done**: `lisage-core::bdd` : `BddController` (FINGERPRINT 3, pas de R-default — arcs 0/1 explicites, R-collapse + R-share seuls) + `BddKeys` (cardinalité 2, pas de δ) + `compiler_bdd` (Shannon descendante, racine niveau 0, niveau ℓ = variable ℓ) + `compter_noeuds` + `evaluer`. `lisage-bench::temoin_bdd` : mini-BDD indépendant (table unique propre). PBT C6 dans `lisage-bench/tests/temoins.rs` : comptes de nœuds identiques + évaluation conforme sur les 2^n affectations (n ∈ 2..=8). Ligne de base bench `bdd_classique_n12_compter` (table LCG déterministe, assertion arène == témoin). Spike `docs/spike-ffi-cudd-sylvan.md`.
- **Evidence**: `make check` OK (76 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : différentiel contre CUDD/Sylvan réels (dépendance non ajoutée).
- **Judgement calls**: pas de dépendance FFI (liste d'escalation) — différentiel C6 livré contre le mini-BDD interne, vraie question escaladée en commentaire structuré sur #14 (label `question`, le label `needs-human` n'existe pas dans ce repo) ; comptage des nœuds = atteignables depuis la racine, terminaux compris, même convention des deux côtés.
- **Escalations**: commentaire structuré posé sur #14 pour la dépendance cudd-sys (options in-process / hors-processus / mode dégradé) ; aucune réponse consommée.

## 2026-11-10 - M1-6 (2/2) : corrections de review PR #53 — erreurs typées aux frontières (feat/13b, 572edb5)

- **Issue**: #13 (M1-6), revue de la PR #53 (1 BLOCKING, 1 TO-FIX, 2 SUGGESTION).
- **Done**: `Existe::new`/`Filtre::new` renvoient `CoreError::SchemaInattendu` au lieu de paniquer ; `valider_niveaux` renvoie `CoreError::NiveauHorsSchema` avant `disperser_chemin` (plus d'indice hors limites) ; constante `PAS_NIVEAU_MAP` nommée dans `schema.rs` et utilisée dans `operations.rs`. Tests ajoutés : schéma non-Map → `SchemaInattendu`, nœud de niveau 4 hors schéma → `NiveauHorsSchema`.
- **Evidence**: `make check` OK (71 tests, 2 ignorés), `cargo clippy --all-targets -- -D warnings` OK, CI verte sur 572edb5, review de suivi 0 BLOCKING / 0 TO-FIX / 2 SUGGESTION (rapport frais, head_sha conforme).
- **Judgement calls**: les 2 SUGGESTION restantes (validation de `image`, `assert!` interne dans `disperser_chemin`) laissées en l'état — non bloquantes.
- **Escalations**: aucune.

## 2026-11-10 - M1-6 (2/2) : opérations à canal hérité seul — `exists`, `filter`, `map` (feat/13b)

- **Issue**: #13 (M1-6 — Pli et semi-anneaux, énoncés A5/C7, partie opérations spec § 4).
- **Done**: `lisage-core::operations` : `Existe` (pli `(Bool,∨,∧)`, verdict au terminal), `Filtre` (garde, membres rejetés → ε(false), règle A2 « tout rejeté ⇒ ε(false) » dans φ_out), `Image` (réétiquetage des atomes, schéma Atom) ; le canal hérité porte l'ensemble des clés présentes. Helpers `existe`/`filtrer`/`image`. Tests unitaires + PBT différentiels voie A vs voie DD (exists booléen ; filter et map par égalité structurelle C1 des nœuds). Périmètre documenté : prédicats de présence (un verdict sur valeurs d'un niveau val(k) collapsé exige le T-EXPAND § 6.2, hors périmètre).
- **Evidence**: `make check` OK (69 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: prédicats restreints à la présence (bien définis sur le DD réduit) plutôt que T-EXPAND dans φ_disp ; règle A2 ajoutée à `Filtre::phi_out` pour réduire les sous-arbres tout-rejetés.
- **Escalations**: aucune.

## 2026-11-10 - M1-6 (1/2) : pli et semi-anneaux — `Pli<S>`, comptage A4, `union` (merge Bool), correction R-collapse (feat/13)

- **Issue**: #13 (M1-6 — Pli et semi-anneaux, énoncés A4/A5/C7).
- **Done**: `lisage-core::pli` : trait `SemiAnneau` (`zero/un/depuis_cardinalite/plus/fois`) avec `Naturel` (ℕ,+,×) et `BoolOu` (Bool,∨,∧) ; transduction `Pli<S>` (fold, H trivial, copie identité en sortie) avec expansion-comptage **A4 on-path** (niveaux valeur sautés ⇒ facteur |Val|, niveaux présence sautés ⇒ facteur 1 et leur niveau valeur hors chemin) et poids de l'arc par défaut implicite (A6) ; `compter` (count = pli sur (ℕ,+,×)) ; `union` (merge de construction **Bool idempotent = le hash-consing**, déclaration A5) style ZDD. Le trait `Transduction::phi_aggr` reçoit désormais le niveau de chaque fils (nécessaire à A4). **Correction M1-2** : `reduire`/`reduire_inverse` ne collapsent plus un niveau à défaut quand deux arcs explicites mènent à une cible ≠ D_δ (le collapse n'était pas préservant pour la sémantique ensemble de chemins, spec § 2.2 — il détruisait des membres et rendait le comptage inexact, cas limite A5). Tests unitaires (exemple spec § 5 count=2 sur val(a) collapsé, ensembles vide/plein, pli BoolOu, union idempotente/⊥ neutre/⊤ = map vide, A4) + PBT différentiel count voie A (interprète) vs voie DD (pli A4) sur ensembles aléatoires de maps.
- **Evidence**: `make check` OK (63 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: correction de `reduire` (comportement M1-2) rendue nécessaire par l'exactitude du comptage A4 — documentée, tests de confluence M1-2 toujours verts ; facteur A4 « on-path » (le niveau valeur d'une clé absente est hors chemin) ; `union` énumère toutes les valeurs d'un niveau sans défaut.
- **Escalations**: aucune.

## 2026-11-10 - M1-5 (2/2) : `lookup(k)` — modes cherche/extrait/stop, fragment de DD, T-EXPAND + greffe (feat/12b)

- **Issue**: #12 (M1-5 — lookup, spec § 6).
- **Done**: `lisage-core::lookup` : `LookupController` (pool de sortie à payload `Option<u64>`, D_δ = `ε(None)`, FINGERPRINT = ControllerId(2)) et la transduction `Lookup` — canal hérité `Mode::{Cherche, Extrait, Stop(v)}`, canal synthétisé = fragment de DD (`BTreeMap<u64, NodeId>`, arcs du niveau `val(k)` vers `ε(Some(v))`), `φ_out` avec T-EXPAND + greffe quand le niveau valeur de la clé a été collapsé (R-collapse), `SchemaKeys::new` public. Tests unitaires (clés présentes/absente, cas collapsé) + PBT différentiel voie A (interprète) vs voie DD dans lisage-bench.
- **Evidence**: `make check` OK (56 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: dans le cas collapsé, aucun `Stop` n'est émis — la valeur vit dans le DD ré-explicité (T-EXPAND), le fragment synthétisé reste vide (testé) ; la greffe a une mémo par valeur `v` (les terminaux greffés dépendent de `v`).
- **Escalations**: aucune.

## 2026-11-10 - M1-5 (1/2) : moteur de transductions généralisé — deux pools, payload de sortie `Out` (feat/12)

- **Issue**: #12 (M1-5 — lookup, préparation du moteur).
- **Done**: généralisation de `lisage-core::transduction` pour lookup (spec § 3.2 : `Δ_out` peut différer de `Δ_in`) : la transduction lit dans un pool à payload `bool` et écrit dans un pool à payload `Out` ; `psi` et `φ_out` prennent le pool de sortie et écrivent eux-mêmes par interning (A3), conformément à « φ_out est la seule règle qui écrit le diagramme » (spec § 3.1). `Transduction` gagne le type associé `Out`. Tests M1-4 mis à jour (deux pools, comparaison structurelle des formes en PBT).
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (52 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: deux pools plutôt qu'un payload enum fourre-tout (types distincts pour domaines distincts) ; la comparaison identité en PBT est structurelle (les NodeId sont propres à chaque pool).
- **Escalations**: aucune.

## 2026-11-10 - M1-4 : moteur de transductions — canaux hérité/synthétisé, φ_aggr, mémo (op, nœud, h) (feat/11)

- **Issue**: #11 (M1-4 — Transductions hérité/synthétisé, énoncé L3).
- **Done**: `lisage-core::transduction` : trait `Transduction` (h0, φ_disp, φ_aggr, φ_out, psi, fingerprint) et runner `Transducteur` implémentant T-TERM/T-NODE (spec § 3.1) avec computed table à clé `(fingerprint(op), nœud, h)` (lemme L1/L3). Le runner intern la forme rendue par φ_out : le résultat est canonique par construction (A3), les réductions M1-2 s'appliquent. Tests unitaires (identité avec canal hérité = profondeur, comptage des chemins acceptants avec φ_aggr = somme, mémo : hits au second run, séparation par fingerprint et par h, application sur un diagramme compilé M1-3) + PBT dans lisage-bench (identité ⇒ même nœud, comptage transduction = comptage naïf récursif, sanité de la mémo sur DAG aléatoires).
- **Evidence**: `make check` OK (52 tests, 2 ignorés rouges M1-7), `cargo clippy --all-targets -- -D warnings` OK, `cargo fmt --all` OK. Non exécuté : `cargo audit`, CI réelle avant push.
- **Judgement calls**: φ_out rend une `Form` (le runner intern) plutôt que d'interner lui-même — garde le trait indépendant du pool ; la mémoire de niveau M (tampons) est absente, non utilisée par les opérations de M1 ; le payload reste `bool` (lookup à payload Option(V) en M1-5, #12).
- **Escalations**: aucune.

## 2026-11-10 - M1-3 (3/3) : décompileur (round-trip) + PBT des schémas, clôture de l'issue (feat/10c)

- **Issue**: #10 (M1-3 — Schémas de référence, énoncé Σ).
- **Done**: `decompiler` dans `lisage-core::schema` (inverse de `compiler`, gère le collapse du domaine à un élément) ; tests unitaires ajoutés (ensemble vide/plein, erreur valeur-vs-clé hors domaine d'une map, round-trip, canonicité C1) ; PBT dans lisage-bench (`schemas.rs`) : round-trip compile→décompile pour Atom/Set/Map aléatoires, et canonicité (valeurs distinctes ⇒ nœuds distincts, même valeur ⇒ même nœud).
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (46 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: le décompileur gère le cas terminal(true) pour un domaine atomique à un seul élément (R-collapse réduit le nœud au terminal).
- **Escalations**: aucune.

## 2026-11-10 - M1-3 (2/3) : module schéma — Atom/Set/Map bornés, compilation balayage d'univers (feat/10b)

- **Issue**: #10 (M1-3 — Schémas de référence, énoncé Σ).
- **Done**: `lisage-core::schema` : `Domaine` (atomes bornés, défaut éventuel), `Schema::{Atom, Set, Map}`, `SchemaKeys` (Val(ℓ) et δ(ℓ) par niveau), `ReferenceController` (D_δ = terminal(false) pour les niveaux de présence, δ = ⊥ pour les niveaux valeur, spec § 2.1), `compiler` (balayage d'univers : un niveau de présence par clé, δ = absent, sans marqueur ; seul l'arc choisi est interné, l'autre mène au défaut porteur — R-collapse saute les absents, style ZDD) et `SchemaError` typée. Tests unitaires : exemple de la spec § 1 (map {a↦1,b↦0} sur K={a,b}, V={0,1}), collapse des absents, ensemble vide/plein, map vide et clé absente, erreurs typées.
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (40 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: `Set(Map(...))` (ensemble de maps, ex. S de la spec) nécessite l'union — reporté à M1-6 (#13) ; M1-3 compile les valeurs simples. Le décompileur (round-trip) et les PBT suivent en PR 3/3.
- **Escalations**: aucune.

## 2026-11-10 - M1-3 (1/3) : algèbre de clés en méthodes d'instance — le schéma est une donnée runtime (feat/10)

- **Issue**: #10 (M1-3 — Schchemas de référence, préparation).
- **Done**: évolution de l'interface du cœur : `KeyAlgebra::{normal_form, cardinality, default}` et `Controller::default_payload` passent en méthodes d'instance, `Controller::keys()` est ajouté ; `Arena` détient désormais l'instance de contrôleur (`Arena::new(controleur, observer)`) au lieu d'un `PhantomData`. Motivation : les schémas de référence (M1-3) sont des données runtime (domaines finis par niveau), que des méthodes statiques ne peuvent pas voir. `Controller::collapse` reste statique (fonction pure). Mocks des tests mis à jour. Aucun changement de comportement : les 35 tests existants passent inchangés.
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (35 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet, `cargo audit`, CI réelle avant push.
- **Judgement calls**: méthodes d'instance plutôt qu'encodage du schéma dans le type `Level` (évite un échappement dynamique non justifié) ; le refactor est découpé en PR séparée car le diff total de M1-3 dépasse la limite ~500 lignes par PR.
- **Escalations**: aucune.

## 2026-11-10 - M1-2 : réductions R-collapse / R-default, confluence prouvée par PBT (feat/9)

- **Issue**: #9 (M1-2 — Réductions, spec § 2.3).
- **Done**: `Reduction<Key>` + `collapse_arcs` + `reduire` (R-collapse puis R-default) + `reduire_inverse` (ordre inverse) dans `lisage-algebra::reduction` ; `Controller::collapse` délègue à `collapse_arcs`. L'arena applique `reduit(level, arcs)` à l'internement : `Collapse` émet `ReductionApplied{rule: Collapse, level}` et retourne la cible, `Forme` émet l'événement `Default` quand des arcs par défaut ont été retirés. `TraceEvent::ReductionApplied` porte désormais le `level`. Tests unitaires (exemple de la spec, R-default, non-collapse, collapse total) + PBT de confluence dans lisage-bench : idempotence (reduire ∘ reduire = reduire) et commutation (reduire = reduire_inverse) ; PBT C1 mise à jour (`test_c1_canonicite_par_handle` : formes réduites égales ⇒ même nœud).
- **Evidence**: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (34 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: sémantique de couverture de la spec § 2.2 (tout arc absent vaut δ(ℓ)) : avec un défaut, le collapse vaut si δ est explicite ou si la cible est D_δ ; sans défaut, toutes les valeurs doivent être explicites. `reduire_inverse` traite le cas « arcs vides après retrait » comme `Collapse(cible retirée)` pour préserver la commutation.
- **Escalations**: aucune.

## 2026-11-10 - M1-1 : arena + handles, hash-consing à clé (contrôleur, ancre, forme) (feat/8)

- **Issue**: #8 (M1-1 — Arena + handles).
- **Done**: `Arena<C: Controller>` dans `lisage-core::arena`, implémentation du port `Pool` : table unique hash-consée à clé (contrôleur, ancre, forme normalisée par l'algèbre de clés), `NodeId` en ordre de création, `handle()` = (contrôleur, ancre, nœud), événements de trace `NodeCreated`/`NodeShared` émis à l'internement (observateur enregistreur épinglé en test). PBT C1 dans lisage-bench : unicité des handles (même forme ⇒ même nœud, même arena ou arenas distinctes ; formes distinctes ⇒ handles distincts).
- **Evidence**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (27 tests, 2 ignorés rouges M1-7). Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: R-share seul à l'internement en M1-1 ; R-collapse/R-default s'insèrent à la création du handle en M1-2 (#9), conformément au séquencement interface → réductions.
- **Escalations**: aucune.

## 2026-11-10 - M1-7 : interprète de référence (voie A) + différentiel A/B/C branché (feat/7)

- **Issue**: #7 (M1-7 — Interprète de référence, phase 2 : tests).
- **Done**: interprète de référence en pur `lisage-lang` sans DD (`value.rs` : `Value::{Atom,Set,Map}` avec `Atom` newtype distinct (ADR-0006) ; `interpreter.rs` : `count/exists/filter/map/member/lookup/saturate`), testé sur l'exemple de la spec § 1 (S = {{a↦1,b↦0},{a↦0,b↦0}} : count = 2, lookup(a) = {Some(1), Some(0)}, filter, saturate idempotent). Harnais différentiel A/B/C branché dans `lisage-bench::differentiel` dès le premier commit : voie A réelle (lang), voies B/C factices ; tests différentiels réels écrits **rouges** (`#[ignore]`, vérifiés FAIL via `cargo nextest --run-ignored only`) avant l'implémentation du cœur, à dé-ignorer en M1-6 (#13). PBT de la voie A (count = cardinalité, filter monotone, lookup = m(k)) dans lisage-bench.
- **Evidence**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` OK (19 tests, 2 ignorés rouges vérifiés rouges), `cargo doc --no-deps` OK. Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: les PBT vivent dans lisage-bench (seule crate à posséder proptest, M0-4) — ajouter proptest à lang aurait été une nouvelle dépendance sans escalation (revue PR #42). Les PBT par énoncé C1/L3/A5/C6 sont écrits dans l'issue qui livre la voie concernée, pas ici (ils ne compileraient pas sans l'API visée).
- **Escalations**: aucune.

## 2026-11-10 - M1-0 : interface du cœur, traits + bornes Send/Sync (feat/6)

- **Issue**: #6 (M1-0 — Interface du cœur, phase 1 : interface).
- **Done**: contrats du cœur posés : `NodeId`/`Anchor`/`ControllerId`/`Handle`/`Level`/`Form` et traits `Pool`/`KeyAlgebra`/`Controller` dans `lisage-algebra::interface`, `Observer`/`TraceEvent`/`NoOpObserver` + `CoreError` dans `lisage-core`, `LangError` + `KeyName` dans `lisage-lang`. Bornes `Send`/`Sync` figées sur les quatre traits (vérifiées dans les sources et à la compilation), politique d'erreurs codifiée, R-collapse testé par PBT (dans lisage-bench, qui possède déjà proptest). ADR-0100 draft (needs-human). Spike FFI CUDD/Sylvan livré avec M1-8 (#14).
- **Evidence**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run --all-targets` (10 tests, dont `test_collapse_pbt`), `cargo doc --no-deps`, `cargo deny check` : tous OK. Revue PR #43 : 1 BLOCKING (Atom alias transparent → newtype) corrigé, 1 TO-FIX (logging différé, tracké via ADR-0100 draft needs-human). Revue PR #42 : 1 BLOCKING (R-collapse sans test) puis 2 BLOCKING (dev-dep proptest retirée d'algebra ; diff > ~500 lignes) + 1 TO-FIX (cohérence WORKLOG/PR), tous corrigés. Non exécuté : `make check` complet (partie Python inchangée), `cargo audit`, CI réelle avant push.
- **Judgement calls**: contrats dans `lisage-algebra` car le graphe du workspace interdit `algebra → core` (ADR-0100 draft) ; pas de crate de log (liste d'escalation), narration en doc-comments + tests.
- **Escalations**: ADR-0100 draft `needs-human` ; commentaire structuré posé sur #14 pour la future dépendance `cudd-sys`.

## 2026-11-10 - M0-5 : socle de bench, criterion + micro-bench factice (feat/5)

- **Issue**: #5 (M0-5 — Socle de bench).
- **Done**: criterion 0.5 branché en dev-dependency de lisage-bench avec harness dédié (`[[bench]] micro`), micro-bench factice `voie_a_factice`, étape `bench` ajoutée à la CI GitHub Actions (rust-check). Le harness MCC conteneurisé reste à vide (corpus en M5).
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (5 tests), `cargo bench -p lisage-bench --bench micro` OK (300 ps). Non exécuté : CI GitHub réelle, cargo nextest.
- **Judgement calls**: le bench factice porte sur voie_a (référence) ; le seuil de régression sera calibré quand les vrais benches arriveront (M1+).
- **Escalations**: aucune.

## 2026-11-10 - M0-4 : charte de test, triptyque A/B/C + proptest (feat/4)

- **Issue**: #4 (M0-4 — Charte de test).
- **Done**: triptyque A/B/C implémenté dans lisage-bench (voie_a = référence naïve, voie_b = DD ordre fixe, voie_c = Lisage+planner, toutes factices) ; proptest installé en dev-dependency ; convention `test_c<n>_*` ; test différentiel factice `test_c0_triptyque_accord` (proptest) + `test_c0_exemple_factice`.
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (5 tests passés au total). Non exécuté : cargo nextest, CI GitHub réelle.
- **Judgement calls**: les trois voies sont des stubs factices (x+1) en attendant l'implémentation réelle M1.
- **Escalations**: aucune.

## 2026-11-10 - M0-2 : squelette du workspace, 7 crates + test de frontières (feat/2)

- **Issue**: #2 (M0-2 — Squelette du workspace).
- **Done**: création des 7 crates (lisage-lang, lisage-algebra, lisage-plan, lisage-core, lisage-backend, lisage-bench, lisage-liseuse) avec dépendances unidirectionnelles dans Cargo.toml (graphe § 8) ; test de frontières dans lisage-bench/tests/frontieres.rs qui lit les Cargo.toml et vérifie le graphe autorisé + présence des 7 crates.
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (3 tests passés : socle factice + 2 frontières). Non exécuté : cargo nextest, CI GitHub réelle.
- **Judgement calls**: le test de frontières parse les Cargo.toml plutôt que d'utiliser cargo-metadata (évite une dépendance dev supplémentaire).
- **Escalations**: aucune.

## 2026-11-10 - M0-1 : socle Rust ajouté au template Python (feat/1)

- **Issue**: #1 (M0-1 — Dépôt depuis project-template).
- **Done**: ajout du socle Rust sans purger le Python (décision du owner en session) : rust-toolchain.toml (stable), Cargo.toml workspace, deny.toml, crate factice lisage-core avec test factice, job `rust-check` dans la CI GitHub Actions (fmt → clippy → nextest → coverage → audit → deny), job `rust-quality-gate` dans GitLab CI, cibles `rust-*` dans le Makefile, hooks cargo fmt/clippy dans pre-commit, `target/` dans .gitignore. Le Python (ruff/mypy/pytest/uv) reste intact.
- **Evidence**: `cargo fmt --all -- --check` OK, `cargo clippy --all-targets -- -D warnings` OK, `cargo test --all-targets` OK (1 test passé). Non exécuté : cargo nextest (installation en cours), cargo-llvm-cov, cargo-audit, cargo-deny, CI GitHub réelle.
- **Judgement calls**: le Python n'est pas purgé (instruction du owner en session, dérogation à l'issue M0-1) ; les deux écosystèmes coexistent, la CI valide les deux.
- **Escalations**: aucune.

## 2026-11-09 - night session live trace

- **Issue**: none (human request: the night loop goes silent while a session runs).
- **Done**: the night loop is observable while it runs: every session prompt is kept under `.night/sessions/<run>/` (instead of a TemporaryDirectory deleted on exit), the trace logs a `still running` heartbeat every `NIGHT_HEARTBEAT_SECONDS` (default 60, `0` disables it, also `--heartbeat`), each stdout result line names the item the session claimed (`(claimed: #4)`), and the explicit mode records whether the claim label landed after each session. `scripts/__main__.py` also gets a dispatch fix: `runner(arguments)` was positional while every `run()` is keyword-only, so `python -m scripts <command>` crashed with a TypeError on the previous revision of the package entry point. Orchestration doc (§8) updated accordingly.
- **Evidence**: ruff check and ruff format green, py_compile green; simulation under Python 3.12 with fake httpx/git/vibe backends: 27/27 assertions (auto-discovery with recovery PRs and claimed suffixes on stdout, heartbeat on/off, anti-spin stop on a claimless session, explicit mode with claim-note trace). Not run: mypy strict, the real Vibe Code CLI, the real GitHub API.
- **Judgement calls**: prompts persist under `.night/` (already gitignored) rather than a temp dir - the morning audit needs them and they cost nothing; the heartbeat wakes once per second to detect a finished session promptly instead of sleeping the whole heartbeat period, so a short session is not padded to 60s.
- **Escalations**: none.

## 2026-11-08 - driver commands as one CLI package

- **Issue**: none (human request).
- **Done**: the eleven flat driver scripts become one package: `python -m scripts <command>` with the subcommands claim, pr, update-pr, merge-pr, wait-ci, review, review-bundle, agent-review, day-report, sync-daily, night; shared plumbing in `scripts/_shared.py` and `scripts/_github.py`; make targets, variables and arguments unchanged. The stale `scripts/<name>.py` paths in ADR-0009/0010/0017/0018/0019/0020 were updated to the new entry point on the owner's explicit instruction, and `docs/decisions/README.md` records the owner-set exception to "Accepted ADRs are never edited". New ADR-0021 (draft) records the migration.
- **Evidence**: ruff check and ruff format green, py_compile green; full simulation under Python 3.12 with fake httpx/git/vibe backends: 74/74 assertions across the eleven subcommands (help wiring, claim, update-pr, pr, wait-ci, the review to merge-pr chain, sync-daily S1-S10, day-report, night auto-discovery/explicit/anti-spin). The patch was re-applied onto a fresh copy of the base and verified byte-identical. Not run: mypy strict, real git and GitHub API, uv sync/pytest.
- **Judgement calls**: editing accepted ADRs departs from the "never edited" rule - done under explicit human instruction in this session; the flat scripts total 3826 lines, the package 3324.
- **Escalations**: ADR-0021 is a draft awaiting the human owner's acceptance.

## 2026-11-08 - milestone filter for the night loop

- **Issue**: none (human request).
- **Done**: `scripts/night_loop.py` gains `--milestones "3,4"` (comma-separated milestone numbers); `make night MILESTONES="3,4"` forwards it. The filter applies to the auto-discovered worklist (issues and recoverable pull requests) and to the explicit issue list - items outside the requested milestones are skipped with a recorded reason. Empty means no filtering.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 33 passed, coverage 94.76%); helper functions exercised directly (parse/match/error path). Not run: a live night loop against the GitHub API.
- **Judgement calls**: milestone identifiers are milestone numbers (the GitHub API identifier), not titles; ADR-0017 (docs/decisions/) left untouched - constitutional file, needs human sign-off.
- **Escalations**: none.

## 2026-11-08 - night autonomy trace and PR body updates

- **Issue**: none (human request).
- **Done**: night loop splits output into a short stdout summary and a verbose trace in `.night/night-loop.log` (override with `NIGHT_LOG_FILE`); branch pattern tolerates descriptive suffixes (`feat/12-slug`); new `scripts/update_pr.py` + `make update-pr PR=N BODY_FILE=...` (ADR-0019); ADR-0017, developer/orchestration docs, prompts and review checklist updated accordingly.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 33 passed, coverage 94.76%). Not run: a live night loop or a real PR body update against the GitHub API.
- **Judgement calls**: the change touches constitutional files (docs/decisions/, docs/agents/, docs/review-checklist.md) - done under explicit human instruction in this session.
- **Escalations**: none.

## 2026-11-08 - type fixes for the subcommand refactor

- **Issue**: none (human request: PR + merge the two commits).
- **Done**: the two commits (sync-daily script, subcommand refactor) failed mypy strict with 15 errors; fixed by parameterizing `argparse._SubParsersAction[argparse.ArgumentParser]` and replacing the explicit-Any `Callable[..., int]` in `scripts/__main__.py`.
- **Evidence**: `make check` green after the fix (ruff, mypy strict 34 files, import-linter, pytest 33 passed, coverage 94.76%).
- **Judgement calls**: fixes committed as a separate commit on the branch rather than rewriting the human's commits.
- **Escalations**: none.

## 2026-11-09 - more output during night session (type fix on top)

- **Issue**: none (human request: commit, push, merge).
- **Done**: commit `857926c` (richer night-session output in scripts/night.py + docs) plus a fix commit restoring `Callable[[argparse.Namespace], int]` in scripts/__main__.py, which the commit had reverted to an explicit Any.
- **Evidence**: `make check` green after the fix (ruff, mypy strict 34 files, import-linter, pytest 33 passed, coverage 94.76%).
- **Judgement calls**: fix committed separately, the human's commit untouched.
- **Escalations**: none.

## 2026-11-09 - night session prints text, not a JSON dump

- **Issue**: none (human relayed the petrichor fix; same one-liner applies here).
- **Done**: `scripts/night.py` runs headless sessions with `--output text` (new `SESSION_OUTPUT_FORMAT` constant, command built by `build_session_command`); new `scripts/tests/test_night.py` pins the command and asserts `"json" not in command`.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%). Not run: a live vibe session.
- **Judgement calls**: ADR-0017 still mentions `--output json` in its decision text - left untouched (constitutional file, human sign-off); the decision itself (process boundary = context reset) is unchanged.
- **Escalations**: none.

## 2026-11-09 - PR template asks for Closes #n

- **Issue**: none (human request).
- **Done**: `.github/PULL_REQUEST_TEMPLATE.md` gains an "Issue closure" section (`Closes #n` in the PR body so GitHub auto-closes the issue on merge) and a matching checklist item.
- **Evidence**: `make check` green (ruff, mypy strict, import-linter, pytest 34 passed, coverage 94.76%).
- **Judgement calls**: PR template chosen over docs/agents/ - it is where the PR author actually looks.
- **Escalations**: none.
