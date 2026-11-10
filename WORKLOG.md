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
