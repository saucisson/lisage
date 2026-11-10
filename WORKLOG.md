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
