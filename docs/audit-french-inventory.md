# Audit: inventory of French-language content (M6-0)

Machine-checkable inventory of every French-language string in the
repository, produced for milestone M6 (English-only documentation and
code), issue #103. Every number below is reproducible with the
grep/regex commands documented in the last section of this report.

Scope: `docs/`, root Markdown files, `crates/`, `tools/`, `scripts/`,
`.github/`, manifests and assets. Excluded: `.git/`, `.venv/`,
`target/`, `.review/`, `.confrontation/`, `.hypothesis/`, lock files.

Files scanned: 323 (see the `find` command in the Reproducibility section); 190 files carry at least one French signal (per-file table below).

## Totals by category

| Category | Files with French | Accented lines | Stop-word lines | French-ident lines |
|---|---|---|---|---|
| Code (crates/, tools/, scripts/) | 120 | 3176 | 3605 | 6378 |
| Documentation (root + docs/ + .github/) | 46 | 1459 | 1495 | 0 |
| Other | 13 | 11 | 43 | 0 |
| Config / manifests | 10 | 6 | 8 | 0 |
| Assets (svg, bench-results) | 1 | 1 | 0 | 0 |

## Per-file detail (files with any signal, sorted by accented-line count)

The report itself (`docs/audit-french-inventory.md`) is excluded from its own table: it does not exist at the audited commit. The French-ident column is measured on `crates/` only (signal 3 greps Rust sources); tools/ and scripts/ rows show 0 because they are not scanned by that pipeline.

| File | Category | Accented lines | Stop-word lines | French-ident lines (crates/ only) | Total lines |
|---|---|---|---|---|---|
| crates/lisage-core/src/atelier.rs | Code (crates/, tools/, scripts/) | 141 | 154 | 302 | 720 |
| crates/lisage-core/src/schema.rs | Code (crates/, tools/, scripts/) | 82 | 94 | 299 | 862 |
| crates/lisage-core/tests/atelier.rs | Code (crates/, tools/, scripts/) | 111 | 119 | 229 | 651 |
| crates/lisage-bench/src/confrontation.rs | Code (crates/, tools/, scripts/) | 109 | 114 | 230 | 705 |
| WORKLOG.md | Documentation (root + docs/ + .github/) | 290 | 262 | 0 | 607 |
| crates/lisage-core/src/transduction.rs | Code (crates/, tools/, scripts/) | 58 | 53 | 146 | 365 |
| docs/spécification-le-calcul-des-transductions-attribuées-instance-de-référence.md | Documentation (root + docs/ + .github/) | 196 | 194 | 0 | 425 |
| crates/lisage-core/tests/schema.rs | Code (crates/, tools/, scripts/) | 31 | 32 | 159 | 356 |
| crates/lisage-bench/src/pnml.rs | Code (crates/, tools/, scripts/) | 68 | 81 | 114 | 779 |
| crates/lisage-core/tests/biplan.rs | Code (crates/, tools/, scripts/) | 57 | 68 | 109 | 401 |
| crates/lisage-bench/tests/confrontation.rs | Code (crates/, tools/, scripts/) | 64 | 46 | 98 | 347 |
| crates/lisage-core/src/transfert.rs | Code (crates/, tools/, scripts/) | 55 | 61 | 103 | 313 |
| docs/proposition-lisage-le-cadre-attribué-des-diagrammes-de-décision.md | Documentation (root + docs/ + .github/) | 158 | 141 | 0 | 311 |
| crates/lisage-core/tests/transfert.rs | Code (crates/, tools/, scripts/) | 36 | 40 | 118 | 277 |
| crates/lisage-core/src/operations.rs | Code (crates/, tools/, scripts/) | 43 | 47 | 110 | 397 |
| crates/lisage-core/tests/relatif.rs | Code (crates/, tools/, scripts/) | 53 | 64 | 95 | 302 |
| docs/état-de-l-art-diagrammes-de-décision.md | Documentation (root + docs/ + .github/) | 148 | 152 | 0 | 300 |
| crates/lisage-plan/src/ordre.rs | Code (crates/, tools/, scripts/) | 74 | 84 | 73 | 356 |
| crates/lisage-core/src/pli.rs | Code (crates/, tools/, scripts/) | 45 | 83 | 100 | 350 |
| crates/lisage-core/tests/saturate.rs | Code (crates/, tools/, scripts/) | 32 | 33 | 111 | 334 |
| docs/feuille-de-route-lisage-jalons-et-issues.md | Documentation (root + docs/ + .github/) | 138 | 124 | 0 | 219 |
| crates/lisage-core/tests/dabs.rs | Code (crates/, tools/, scripts/) | 47 | 52 | 90 | 339 |
| crates/lisage-liseuse/tests/pas_a_pas.rs | Code (crates/, tools/, scripts/) | 40 | 37 | 97 | 468 |
| crates/lisage-liseuse/src/vue_ordre.rs | Code (crates/, tools/, scripts/) | 69 | 89 | 64 | 415 |
| crates/lisage-algebra/src/reduction.rs | Code (crates/, tools/, scripts/) | 46 | 46 | 86 | 182 |
| crates/lisage-core/src/cdd.rs | Code (crates/, tools/, scripts/) | 55 | 64 | 77 | 281 |
| crates/lisage-core/src/add.rs | Code (crates/, tools/, scripts/) | 27 | 39 | 104 | 322 |
| crates/lisage-core/src/saturate.rs | Code (crates/, tools/, scripts/) | 51 | 54 | 80 | 257 |
| crates/lisage-core/tests/cdd.rs | Code (crates/, tools/, scripts/) | 37 | 38 | 87 | 234 |
| crates/lisage-bench/tests/philosophes_bench.rs | Code (crates/, tools/, scripts/) | 43 | 52 | 80 | 286 |
| crates/lisage-core/src/arena.rs | Code (crates/, tools/, scripts/) | 41 | 48 | 81 | 350 |
| crates/lisage-bench/src/sdd.rs | Code (crates/, tools/, scripts/) | 29 | 36 | 91 | 214 |
| crates/lisage-core/src/lookup.rs | Code (crates/, tools/, scripts/) | 30 | 37 | 89 | 278 |
| docs/exemple-fil-rouge-n-philosophes-de-bout-en-bout.md | Documentation (root + docs/ + .github/) | 118 | 110 | 0 | 271 |
| crates/lisage-liseuse/tests/philosophes.rs | Code (crates/, tools/, scripts/) | 40 | 48 | 76 | 258 |
| crates/lisage-core/src/integrale.rs | Code (crates/, tools/, scripts/) | 44 | 39 | 70 | 216 |
| crates/lisage-liseuse/src/pas_a_pas.rs | Code (crates/, tools/, scripts/) | 51 | 62 | 60 | 373 |
| crates/lisage-core/tests/memoire.rs | Code (crates/, tools/, scripts/) | 32 | 33 | 78 | 223 |
| crates/lisage-algebra/src/relatif.rs | Code (crates/, tools/, scripts/) | 52 | 71 | 57 | 246 |
| crates/lisage-core/src/dbm.rs | Code (crates/, tools/, scripts/) | 64 | 73 | 42 | 333 |
| crates/lisage-plan/tests/ordre.rs | Code (crates/, tools/, scripts/) | 32 | 34 | 74 | 321 |
| crates/lisage-core/tests/operations.rs | Code (crates/, tools/, scripts/) | 17 | 15 | 87 | 212 |
| crates/lisage-liseuse/tests/vue_memoire.rs | Code (crates/, tools/, scripts/) | 30 | 30 | 73 | 218 |
| crates/lisage-bench/src/triptyque.rs | Code (crates/, tools/, scripts/) | 40 | 48 | 62 | 233 |
| crates/lisage-algebra/src/interface.rs | Code (crates/, tools/, scripts/) | 53 | 56 | 47 | 180 |
| crates/lisage-core/tests/transduction.rs | Code (crates/, tools/, scripts/) | 21 | 24 | 78 | 291 |
| crates/lisage-core/tests/lookup.rs | Code (crates/, tools/, scripts/) | 16 | 18 | 79 | 170 |
| crates/lisage-liseuse/src/publication.rs | Code (crates/, tools/, scripts/) | 34 | 40 | 61 | 245 |
| crates/lisage-bench/tests/sdd.rs | Code (crates/, tools/, scripts/) | 20 | 24 | 72 | 184 |
| crates/lisage-plan/src/oracle.rs | Code (crates/, tools/, scripts/) | 56 | 52 | 36 | 228 |
| crates/lisage-core/tests/region.rs | Code (crates/, tools/, scripts/) | 23 | 22 | 68 | 211 |
| crates/lisage-core/tests/pli.rs | Code (crates/, tools/, scripts/) | 11 | 19 | 79 | 173 |
| crates/lisage-liseuse/src/svg.rs | Code (crates/, tools/, scripts/) | 17 | 21 | 70 | 200 |
| crates/lisage-liseuse/tests/vue_ordre.rs | Code (crates/, tools/, scripts/) | 48 | 38 | 38 | 241 |
| crates/lisage-plan/src/conversion.rs | Code (crates/, tools/, scripts/) | 42 | 55 | 42 | 198 |
| crates/lisage-core/tests/add.rs | Code (crates/, tools/, scripts/) | 20 | 27 | 63 | 207 |
| crates/lisage-core/tests/dbm.rs | Code (crates/, tools/, scripts/) | 30 | 28 | 53 | 205 |
| crates/lisage-core/tests/integrale.rs | Code (crates/, tools/, scripts/) | 37 | 38 | 46 | 227 |
| crates/lisage-bench/tests/schemas.rs | Code (crates/, tools/, scripts/) | 13 | 10 | 68 | 176 |
| crates/lisage-bench/tests/lookup.rs | Code (crates/, tools/, scripts/) | 13 | 14 | 67 | 150 |
| crates/lisage-bench/tests/transductions.rs | Code (crates/, tools/, scripts/) | 11 | 13 | 69 | 226 |
| crates/lisage-core/tests/integrale_dbm.rs | Code (crates/, tools/, scripts/) | 41 | 33 | 39 | 237 |
| crates/lisage-liseuse/src/vue_memoire.rs | Code (crates/, tools/, scripts/) | 39 | 55 | 41 | 221 |
| crates/lisage-core/tests/ddd.rs | Code (crates/, tools/, scripts/) | 30 | 27 | 45 | 157 |
| docs/analyse-critique-σdd-structurels-ordres-et-bibliothèque-en-couches.md | Documentation (root + docs/ + .github/) | 73 | 70 | 0 | 136 |
| crates/lisage-bench/tests/operations.rs | Code (crates/, tools/, scripts/) | 12 | 12 | 58 | 164 |
| crates/lisage-liseuse/src/philosophes.rs | Code (crates/, tools/, scripts/) | 28 | 40 | 40 | 177 |
| docs/deuxième-exemple-fil-rouge-le-protocole-à-fenêtre-glissante.md | Documentation (root + docs/ + .github/) | 68 | 62 | 0 | 134 |
| crates/lisage-bench/tests/saturate.rs | Code (crates/, tools/, scripts/) | 18 | 20 | 47 | 159 |
| crates/lisage-core/src/region.rs | Code (crates/, tools/, scripts/) | 27 | 30 | 38 | 150 |
| docs/analyse-critique-note-de-conception-dd-ir-et-localité-mémoire.md | Documentation (root + docs/ + .github/) | 65 | 68 | 0 | 148 |
| crates/lisage-liseuse/tests/liseuse_v0.rs | Code (crates/, tools/, scripts/) | 20 | 21 | 43 | 160 |
| docs/liseuse-visualiseur-d-ordres-de-diagrammes-et-d-opérations.md | Documentation (root + docs/ + .github/) | 63 | 64 | 0 | 121 |
| crates/lisage-bench/tests/metriques.rs | Code (crates/, tools/, scripts/) | 27 | 30 | 35 | 126 |
| crates/lisage-core/src/bdd.rs | Code (crates/, tools/, scripts/) | 19 | 23 | 40 | 161 |
| crates/lisage-lang/src/interpreter.rs | Code (crates/, tools/, scripts/) | 18 | 23 | 41 | 120 |
| crates/lisage-bench/src/temoin_bdd.rs | Code (crates/, tools/, scripts/) | 15 | 17 | 43 | 108 |
| crates/lisage-liseuse/tests/atelier_philosophes.rs | Code (crates/, tools/, scripts/) | 14 | 20 | 42 | 77 |
| crates/lisage-plan/tests/oracle.rs | Code (crates/, tools/, scripts/) | 20 | 16 | 35 | 147 |
| crates/lisage-core/src/observer.rs | Code (crates/, tools/, scripts/) | 34 | 29 | 20 | 89 |
| crates/lisage-liseuse/tests/publication.rs | Code (crates/, tools/, scripts/) | 19 | 19 | 35 | 116 |
| crates/lisage-bench/tests/handles.rs | Code (crates/, tools/, scripts/) | 16 | 18 | 36 | 116 |
| crates/lisage-bench/tests/transfert.rs | Code (crates/, tools/, scripts/) | 13 | 15 | 38 | 115 |
| crates/lisage-core/src/ddd.rs | Code (crates/, tools/, scripts/) | 21 | 24 | 29 | 93 |
| crates/lisage-bench/tests/pnml.rs | Code (crates/, tools/, scripts/) | 16 | 22 | 33 | 274 |
| crates/lisage-core/tests/arena.rs | Code (crates/, tools/, scripts/) | 12 | 14 | 37 | 263 |
| crates/lisage-core/src/biplan.rs | Code (crates/, tools/, scripts/) | 25 | 28 | 19 | 85 |
| crates/lisage-core/src/memoire.rs | Code (crates/, tools/, scripts/) | 17 | 18 | 27 | 97 |
| crates/lisage-bench/tests/triptyque.rs | Code (crates/, tools/, scripts/) | 14 | 21 | 28 | 179 |
| crates/lisage-bench/tests/cudd_local.rs | Code (crates/, tools/, scripts/) | 14 | 14 | 25 | 96 |
| crates/lisage-plan/tests/vtree.rs | Code (crates/, tools/, scripts/) | 17 | 20 | 22 | 152 |
| docs/spike-ffi-cudd-sylvan.md | Documentation (root + docs/ + .github/) | 39 | 39 | 0 | 71 |
| crates/lisage-bench/tests/reductions.rs | Code (crates/, tools/, scripts/) | 8 | 12 | 30 | 93 |
| crates/lisage-liseuse/src/trace.rs | Code (crates/, tools/, scripts/) | 14 | 15 | 24 | 149 |
| crates/lisage-bench/tests/pli.rs | Code (crates/, tools/, scripts/) | 8 | 8 | 29 | 83 |
| crates/lisage-bench/tests/differentiel.rs | Code (crates/, tools/, scripts/) | 12 | 14 | 23 | 113 |
| crates/lisage-lang/tests/interpreter.rs | Code (crates/, tools/, scripts/) | 4 | 8 | 30 | 162 |
| crates/lisage-plan/tests/conversion.rs | Code (crates/, tools/, scripts/) | 22 | 22 | 12 | 190 |
| crates/lisage-algebra/src/vtree.rs | Code (crates/, tools/, scripts/) | 18 | 28 | 13 | 133 |
| crates/lisage-bench/tests/temoins.rs | Code (crates/, tools/, scripts/) | 10 | 9 | 19 | 47 |
| crates/lisage-core/tests/bdd.rs | Code (crates/, tools/, scripts/) | 6 | 7 | 23 | 68 |
| crates/lisage-core/src/error.rs | Code (crates/, tools/, scripts/) | 18 | 16 | 10 | 75 |
| docs/decisions/adr-0100-interface-coeur-m1.md | Documentation (root + docs/ + .github/) | 26 | 30 | 0 | 49 |
| crates/lisage-bench/tests/memoire.rs | Code (crates/, tools/, scripts/) | 6 | 3 | 15 | 34 |
| crates/lisage-bench/benches/micro.rs | Code (crates/, tools/, scripts/) | 4 | 10 | 16 | 60 |
| crates/lisage-bench/src/bin/confrontation.rs | Code (crates/, tools/, scripts/) | 6 | 7 | 11 | 40 |
| .github/pr-body-16.md | Documentation (root + docs/ + .github/) | 16 | 18 | 0 | 47 |
| .github/pr-body-14.md | Documentation (root + docs/ + .github/) | 15 | 13 | 0 | 51 |
| crates/lisage-bench/src/differentiel.rs | Code (crates/, tools/, scripts/) | 7 | 7 | 6 | 28 |
| crates/lisage-bench/tests/collapse.rs | Code (crates/, tools/, scripts/) | 3 | 4 | 10 | 85 |
| .github/pr-body-13.md | Documentation (root + docs/ + .github/) | 12 | 13 | 0 | 57 |
| crates/lisage-bench/tests/frontieres.rs | Code (crates/, tools/, scripts/) | 9 | 12 | 3 | 149 |
| crates/lisage-core/src/lib.rs | Code (crates/, tools/, scripts/) | 3 | 6 | 9 | 39 |
| crates/lisage-lang/src/error.rs | Code (crates/, tools/, scripts/) | 8 | 8 | 4 | 37 |
| .github/pr-body-15.md | Documentation (root + docs/ + .github/) | 11 | 9 | 0 | 40 |
| crates/lisage-lang/src/value.rs | Code (crates/, tools/, scripts/) | 7 | 3 | 4 | 22 |
| .github/pr-body-12b.md | Documentation (root + docs/ + .github/) | 9 | 7 | 0 | 53 |
| .github/pr-body-13b.md | Documentation (root + docs/ + .github/) | 8 | 5 | 0 | 50 |
| docs/les-7-formes-des-philosophes-diagramme-svg.svg | Other | 7 | 7 | 0 | 90 |
| crates/lisage-algebra/src/lib.rs | Code (crates/, tools/, scripts/) | 3 | 2 | 3 | 11 |
| crates/lisage-bench/src/lib.rs | Code (crates/, tools/, scripts/) | 3 | 1 | 2 | 24 |
| crates/lisage-plan/src/vtree.rs | Code (crates/, tools/, scripts/) | 3 | 6 | 2 | 25 |
| crates/lisage-lang/src/lib.rs | Code (crates/, tools/, scripts/) | 3 | 3 | 1 | 12 |
| crates/lisage-core/tests/socle.rs | Code (crates/, tools/, scripts/) | 0 | 0 | 3 | 7 |
| crates/lisage-plan/src/lib.rs | Code (crates/, tools/, scripts/) | 1 | 1 | 2 | 7 |
| docker/cudd/Dockerfile | Other | 3 | 3 | 0 | 30 |
| .github/pr-body-budget.md | Documentation (root + docs/ + .github/) | 2 | 2 | 0 | 36 |
| crates/lisage-core/tests/interface.rs | Code (crates/, tools/, scripts/) | 2 | 2 | 0 | 79 |
| docs/decisions/adr-0102-docker-local-witnesses.md | Documentation (root + docs/ + .github/) | 2 | 2 | 0 | 37 |
| .github/workflows/ci.yml | Config / manifests | 1 | 1 | 0 | 112 |
| .gitignore | Other | 1 | 1 | 0 | 45 |
| bench-results/philosophes-n5.svg | Assets (svg, bench-results) | 1 | 0 | 0 | 336 |
| crates/lisage-algebra/Cargo.toml | Config / manifests | 1 | 0 | 0 | 11 |
| crates/lisage-backend/Cargo.toml | Config / manifests | 1 | 0 | 0 | 12 |
| crates/lisage-backend/src/lib.rs | Code (crates/, tools/, scripts/) | 1 | 0 | 0 | 2 |
| crates/lisage-bench/Cargo.toml | Config / manifests | 1 | 0 | 0 | 25 |
| crates/lisage-lang/Cargo.toml | Config / manifests | 1 | 1 | 0 | 10 |
| crates/lisage-liseuse/src/lib.rs | Code (crates/, tools/, scripts/) | 0 | 1 | 1 | 10 |
| crates/lisage-plan/Cargo.toml | Config / manifests | 1 | 1 | 0 | 12 |
| docs/decisions/adr-0019-pull-request-body-updates.md | Documentation (root + docs/ + .github/) | 1 | 4 | 0 | 67 |
| docs/decisions/adr-0101-budget-pr-1000-lignes.md | Documentation (root + docs/ + .github/) | 1 | 0 | 0 | 33 |
| .coverage | Other | 0 | 8 | 0 | 1459 |
| .env | Other | 0 | 1 | 0 | 28 |
| .env.example | Other | 0 | 1 | 0 | 28 |
| .github/ISSUE_TEMPLATE/task.md | Documentation (root + docs/ + .github/) | 0 | 2 | 0 | 28 |
| .github/PULL_REQUEST_TEMPLATE.md | Documentation (root + docs/ + .github/) | 0 | 2 | 0 | 41 |
| .gitlab/issue_templates/task.md | Other | 0 | 2 | 0 | 22 |
| .gitlab/merge_request_templates/default.md | Other | 0 | 1 | 0 | 26 |
| .pre-commit-config.yaml | Config / manifests | 0 | 1 | 0 | 48 |
| .vibe/config.toml | Config / manifests | 0 | 1 | 0 | 88 |
| .vibe/prompts/night-auto.md | Other | 0 | 6 | 0 | 74 |
| .vibe/prompts/night-issue.md | Other | 0 | 5 | 0 | 45 |
| AGENTS.md | Documentation (root + docs/ + .github/) | 0 | 6 | 0 | 113 |
| LICENSE | Other | 0 | 6 | 0 | 22 |
| Makefile | Other | 0 | 1 | 0 | 119 |
| README.md | Documentation (root + docs/ + .github/) | 0 | 4 | 0 | 104 |
| crates/lisage-liseuse/Cargo.toml | Config / manifests | 0 | 1 | 0 | 12 |
| docs/agents/developer.md | Documentation (root + docs/ + .github/) | 0 | 18 | 0 | 178 |
| docs/agents/escalation.md | Documentation (root + docs/ + .github/) | 0 | 5 | 0 | 62 |
| docs/agents/orchestration.md | Documentation (root + docs/ + .github/) | 0 | 11 | 0 | 200 |
| docs/agents/reviewer.md | Documentation (root + docs/ + .github/) | 0 | 6 | 0 | 100 |
| docs/architecture.md | Documentation (root + docs/ + .github/) | 0 | 3 | 0 | 155 |
| docs/branching.md | Documentation (root + docs/ + .github/) | 0 | 3 | 0 | 101 |
| docs/decisions/README.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 67 |
| docs/decisions/adr-0001-toolchain.md | Documentation (root + docs/ + .github/) | 0 | 3 | 0 | 58 |
| docs/decisions/adr-0004-readability-first-style.md | Documentation (root + docs/ + .github/) | 0 | 2 | 0 | 43 |
| docs/decisions/adr-0006-precise-typing.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 53 |
| docs/decisions/adr-0008-security-gates.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 44 |
| docs/decisions/adr-0009-branch-model.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 83 |
| docs/decisions/adr-0010-agent-protocol.md | Documentation (root + docs/ + .github/) | 0 | 2 | 0 | 95 |
| docs/decisions/adr-0013-delegated-maintenance.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 46 |
| docs/decisions/adr-0015-agent-runtime-permissions.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 70 |
| docs/decisions/adr-0016-apis-rest-best-practices.md | Documentation (root + docs/ + .github/) | 0 | 6 | 0 | 62 |
| docs/decisions/adr-0017-unattended-loop.md | Documentation (root + docs/ + .github/) | 0 | 12 | 0 | 153 |
| docs/decisions/adr-0018-active-ci-wait.md | Documentation (root + docs/ + .github/) | 0 | 5 | 0 | 66 |
| docs/decisions/adr-0021-scripts-cli-package.md | Documentation (root + docs/ + .github/) | 0 | 1 | 0 | 90 |
| docs/review-checklist.md | Documentation (root + docs/ + .github/) | 0 | 9 | 0 | 93 |
| pyproject.toml | Config / manifests | 0 | 2 | 0 | 199 |
| scripts/_github.py | Code (crates/, tools/, scripts/) | 0 | 4 | 0 | 103 |
| scripts/_shared.py | Code (crates/, tools/, scripts/) | 0 | 3 | 0 | 118 |
| scripts/ci.py | Code (crates/, tools/, scripts/) | 0 | 5 | 0 | 288 |
| scripts/claims.py | Code (crates/, tools/, scripts/) | 0 | 11 | 0 | 199 |
| scripts/day.py | Code (crates/, tools/, scripts/) | 0 | 7 | 0 | 358 |
| scripts/merge.py | Code (crates/, tools/, scripts/) | 0 | 2 | 0 | 331 |
| scripts/night.py | Code (crates/, tools/, scripts/) | 0 | 10 | 0 | 904 |
| scripts/pulls.py | Code (crates/, tools/, scripts/) | 0 | 10 | 0 | 297 |
| scripts/review.py | Code (crates/, tools/, scripts/) | 0 | 6 | 0 | 360 |
| scripts/session.py | Code (crates/, tools/, scripts/) | 0 | 5 | 0 | 440 |
| tools/notify/README.md | Other | 0 | 1 | 0 | 35 |
| tools/notify/src/notify/models/errors.py | Code (crates/, tools/, scripts/) | 0 | 1 | 0 | 14 |

## Reproducibility

All counts are produced by the pipelines below, run from the repository
root. The `EXCLUDE` regex filters the excluded directories (`.git/`,
`.venv/`, `target/`, `.review/`, `.confrontation/`, `.hypothesis/`) on
every pipeline; lock files (`Cargo.lock`, `uv.lock`) are excluded only
in the `find` denominator command at the end. `grep -c` counts matching
lines per file; `grep -v ':0$'` keeps only files with at least one
match; the totals are the column sums of the per-file table. `grep -I`
skips binary files (e.g. `.gz`), so no binary row appears in the table.

```sh
EXCLUDE='^\./(\.git|\.venv|target|\.review|\.confrontation|\.hypothesis)/'

# Signal 1 - accented characters (primary, unambiguous French signal),
# per-file matching-line counts over every text file:
grep -rIcP '[àâäéèêëîïôöùûüçÀÂÄÉÈÊËÎÏÔÖÙÛÜÇœæ]' . \
  | grep -vE "$EXCLUDE" | grep -v ':0$' | sort -t: -k2 -rn

# Signal 2 - French stop-words in prose (secondary signal):
grep -rIcE '\b(le|la|les|un|une|des|du|de|et|est|en|dans|pour|par|sur|avec|sans|sous|que|qui|quoi|dont|au|aux|ce|cet|cette|ces|son|sa|ses|leur|leurs|notre|votre|il|elle|ils|elles|nous|vous|etre|avoir|fait|faire|plus|moins|tres|tout|tous|toute|toutes|autre|autres|meme|ici|cela|ne|pas|si|mais|donc|or|ni|car|afin|lors|lorsque|bien|deja|encore|chaque|entre|parmi|selon|contre|chez|vers|depuis|jusqu)\b' . \
  | grep -vE "$EXCLUDE" | grep -v ':0$' | sort -t: -k2 -rn

# Signal 3 - French snake_case identifier fragments, Rust sources only:
grep -rIcE '\b[a-z_]+_(fenetre|temoin|construire|mesurer|reduire|reduction|pli|atelier|arene|schema|transduction|transfert|memoire|observateur|biplan|forme|racine|feuille|jalon|charge|erreur|noeud|table|chemin|glisser|ancre|entree|sortie|appel|cle|base|arc|paire|ensemble|saturer|integrale|compter|compteur|taille|niveau|valeur|ordre|tri|borne|seuil|paquet|morceau|visiter|parcours|premier|dernier|suivant|precedent|profondeur|largeur|etiquette|marquer|enfant|parent|branche|produit|somme|quotient|intervalle|couleur|legende|vue|region|transition|etat|attribut|variable|compte|mesure|domaine|cardinal|cible|hauteur|poids|exemple|nombre|destination|source|texte|nom|point|piste|terminal|lire|contre|disperser|decompiler|compiler|filtrer|trier|valider|comparer|convertir|serialiser|extraire|empiler|depiler|iterer|tester|epingle|rejeter|notifier|dessiner|ecrire|ouvrir|fermer|decouper|fusionner|coller|parcourir|visiter|reduire|saturer|normaliser|canoniser|fabriquer|appliquer|resoudre|verifier|exporter|importer|publier|reproduire|estimer|evaluer|calculer|analyser|synthetiser|observer|marquer|etiqueter|plier|deplier|glisser|ancrer|fenetrer|temoigner|confronter|differentiel|ordonner|filtrer|projeter|transferer|memoriser|visualiser|afficher|imprimer|deserialiser)\b' crates/ \
  | grep -v ':0$' | sort -t: -k2 -rn

# Total lines column (per file):
find . -type f | grep -vE "$EXCLUDE" | grep -vE '\./(Cargo\.lock|uv\.lock)$' | xargs wc -l

# Files scanned (denominator of the report):
find . -type f | grep -vE "$EXCLUDE" | grep -vE '\./(Cargo\.lock|uv\.lock)$' | wc -l
```
