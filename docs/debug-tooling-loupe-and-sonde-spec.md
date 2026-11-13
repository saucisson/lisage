# Work specification: Loupe & Sonde debugging tooling

Statement of work, derived from the design document ("Loupe & Sonde: debugging
operations on decision diagrams", same project). Applies to the repository
`saucisson/lisage` (branch `daily`). The issues are written to be filed in the
repository tracker; the milestones slot in after M2-6 without blocking M3-4.

## 1. Objective

Deliver semantic debugging tooling for operations on decision diagrams: a
reconstructible v3 trace, an index/mirror with O(log n) navigation, a REPL
debugger (Loupe) over traces, a hot-attach probe (Sonde), a step-by-step
semantic oracle, and offline/live web screens. Replay mode is the primary
mode; live mode is for attaching to a long-running program not started for
tracing.

## 2. Working rules (transverse, enforceable on every issue)

### 2.1 Language: the code is in English

**All produced code is in English**: identifiers (types, functions, variables,
modules), doc comments, code comments, test names, error messages of the new
tooling, commit messages (Conventional Commits in English).

- Documentary artifacts remain in French: `docs/`, ADRs, issue titles and
  descriptions, this specification (repository doctrine).
- Coexistence: the new modules **wrap** the existing repository APIs (French
  identifiers) without renaming them; no renaming of existing code within the
  scope of this work.
- This rule deviates from the current repository style (French identifiers):
  the deviation and its scope (only the new crates/modules of the tooling) are
  recorded in ADR-0103.

### 2.2 Quality and budget

- `make check` green before any report (lint, typecheck, imports, tests,
  rust-check), ADR-0007/0008.
- PR budget: ~1,000 changed lines maximum (ADR-0101); one issue may require
  several PRs, never the reverse.
- Repository style respected in the English code: named parameters, no
  abbreviations (`id`/`url`/`http` excepted), named constants, complexity ≤ 10,
  nesting ≤ 4, doc comments on every public callable (ADR-0003/0004/0006).
- Hand-written serialization, zero new external dependency without escalation
  (escalation list in `AGENTS.md`).
- Performance: the three budgeted regimes (§ milestone D1) are a **blocking**
  review criterion; any regression is a finding.
- Non-obvious decisions: ADR (numbering ≥ 0100).

## 3. Labels

| Label | Description | Color |
|---|---|---|
| `debug-outillage` | parent epic: the Loupe & Sonde tooling as a whole | `#5c6bc0` |
| `trace-v3` | trace v3 format, spans, events, `TraceVal`, serializer | `#7e57c2` |
| `index-miroir` | index brick: mirror, snapshots, seek, inverse indexes, span tree | `#42a5f5` |
| `loupe` | REPL: navigation, predicates, text views, scriptable mode | `#26a69a` |
| `sonde` | hot attach: gate, local socket, gate verbs, densities | `#ef6c00` |
| `ecrans` | GUIs: WASM, offline/live pages, gateway, DAP | `#ec407a` |
| `oracle` | sampled semantic differential, growth control | `#8d6e63` |
| `perf` | performance budgets, micro-benches, non-regression | `#c0ca33` |
| `ci-gates` | CI properties: mirror == real state, byte-identical trace, on/off results | `#78909c` |
| `adr` | decision to record (ADR-0103 and following) | `#9e9d24` |
| `tests-pbt` | property-based tests added or turned green | `#66bb6a` |
| `docs` | documentation, figures, Liseuse doc update | `#b0bec5` |

## 4. Milestones

| Milestone | Name | Objective | Exit criterion |
|---|---|---|---|
| **D1** | Registry | complete v3 trace: spans, new events, `TraceVal`, serializer + deserializer, cost benches | v3 trace on philosophers n=3 replayed identically; the three budgeted perf regimes blocking in CI |
| **D2** | Mirror | index brick: reconstruction from the trace alone, snapshots, O(log n) seek, inverse indexes, unfoldable span tree | CI property mirror == real state; seek < 50 ms at 100k events; Scrubber migrated with preserved behavior |
| **D3** | Loupe | REPL debugger over traces: steps, predicates (node/path/state), views, `explain`, scriptable mode | standard philosophers n=4 scenario solved in < 5 min (first unshared node of a round; why a miss) |
| **D4** | Sonde | hot attach: compiled-inactive gate, local socket, header snapshot, gate verbs, densities, ADR-0103 | attach to a program not started for tracing; CI property: paused trace byte-identical |
| **D5** | Oracle | semantic anti-regression: sampled differential at boundaries, fixpoint growth, saturation dashboard | a deliberately faulty `τ` (test injection) caught at the faulty round, not at the end of the chain |
| **D6** | Screens | index in WASM, committable offline page, loopback gateway + live page, optional DAP | offline page committed, openable without infra; live page drives pause/step of an attached program |

**Sequencing**: D1 → D2 → D3 on the critical path (replay is the primary mode).
D4 depends on D1 (events) and on D3-2 (shared predicate evaluator). D5 depends
on D2. D6 depends on D2 (index) and D3 (command bus). D4, D5 and D6 are partially
parallelizable after D2.

## 5. Issues

### Milestone D1 "Registry"

**D1-1 — Span model (trace v3).**
`SpanOpen`/`SpanClose` (`SpanId`, kinds `Op`/`Round`/`Visite`, lane number for
future parallelism), contextualization of all existing events by their span.
Emission at boundaries in `transduction.rs`, `saturate.rs`, `integrale.rs`,
`atelier.rs`.
DoD: every event carries its span; v2 → v3 tests; no engine behavior change
(`tests-pbt`, `trace-v3`).

**D1-2 — New registry events.**
`OpStart`/`OpEnd`, `VisiteEntree`/`VisiteSortie`, `MemoConsult`/`MemoInsert`
(full key: op, node, `h`, version), `EcritureMemoire` (version before → after),
`StructureChange` (classification Identité/Réétiquetage/Collapse/Expansion/
Restructuration + arc correspondence), extended `NodeCreated` (full form),
extended `Gc` (collected list), extended `Round` (active/modified levels,
before/after handles, growth). Generic emission in the runner (input/output
form comparison), not per operation.
DoD: `StructureChange` visible on T-EXPAND (`lookup`) and collapse A2
(`Filtre`) in dedicated tests (`trace-v3`).

**D1-3 — `TraceVal` and the `Traceable` trait.**
Bounded mini-ADT (Entier/Texte/Ensemble/Carte/Fragment/Élidié), lazy conversion
(the value is computed only if the filter passes), `Debug` fallback by default.
Implementations for `H`/`S`/`M` of the existing transductions.
DoD: no more `format!` on the hot path outside filters; bound and elision
tests (`trace-v3`, `perf`).

**D1-4 — v3 serializer + deserializer.**
Hand-written `{"v":3,...}` schema, explicit rejection of unknown versions,
documented decision on reading v2 traces. Round-trip and rejection tests.
DoD: identical round-trip on all test traces; deserializer covered by tests
(`trace-v3`).

**D1-5 — Instrumentation micro-bench and CI budgets.**
Extension of `lisage-bench/benches/micro.rs`: three regimes (no observer;
compiled-inactive gate; active trace). Property: identical results with trace
off/on.
DoD: per-regime budgets quantified, blocking non-regression in review; on/off
property in CI (`perf`, `ci-gates`).

### Milestone D2 "Mirror"

**D2-1 — Mirror of pool, memo and level memory.**
Full reconstruction from the trace alone (forms in `NodeCreated`, lists in
`Gc`, memo keys, versions).
DoD: CI property mirror == real pool/memo/memory state at end of trace, on
philosophers and a DDD/DBM instance (`index-miroir`, `ci-gates`).

**D2-2 — Snapshots, O(log n) seek, prefix sums.**
Snapshots at boundaries (budget-throttled), seek = last snapshot ≤ k + replay,
migration of the existing `Scrubber`.
DoD: seek < 50 ms at 100k events; existing Liseuse tests green without
behavior change (`index-miroir`).

**D2-3 — Inverse indexes.**
Node → event ranges, round → interval, op → interval, `explain`/`goto` queries
in O(log n).
DoD: `explain n42` on a philosophers trace answered by index query, no linear
scan (`index-miroir`).

**D2-4 — Span tree and folded summaries.**
Tree reconstruction (nested intervals), folded summaries in O(1) via prefix
sums (events, nodes created, hits, growth), convergence marker, **comparison of
two iterations** (diff between intervals).
DoD: n=4 saturation presented as N summary lines; round 11 vs 12 diff
operational (`index-miroir`).

### Milestone D3 "Loupe"

**D3-1 — Command bus and REPL.**
`lisage-debug` binary: `step/step-over/step-out/next-op/next-round/back/goto/
until/where/frame/continue`, visit stack, granularity parity with the span
tree.
DoD: standard n=4 scenario < 5 min (`loupe`).

**D3-2 — Predicate AST and shared evaluator.**
JSON predicates over (event, reconstructed state): node conditions,
prefix-path conditions (incremental automaton at visit openings, "or" of paths
= union of regions), state-valuation conditions (evaluated at `OpEnd`/`Round`).
`PathPredicate` port implemented **controller-side** (arc equality for the
reference instance, region inclusion for `DbmKeys`). Same evaluator for replay
and gate.
DoD: the same predicate script runs in replay and at the gate; path condition
verified on the DBM instance (`loupe`, `tests-pbt`).

**D3-3 — Inspection views.**
`node`, `memo` (computed table, current key, entries invalidated by version
bump), `memoire`, `diff` (input/output fragments), `eval <valuation>`,
`display`, `ops`, current SVG rendering (reuse of `svg.rs`/`pas_a_pas.rs`).
DoD: an L1 invalidation (version bump → dead memo entries) visible in one
command (`loupe`).

**D3-4 — Scriptable mode.**
`--until "<predicate>"`, `--dump-frame`, exit codes, non-interactive execution
for CI and triage of archived traces.
DoD: triage of an archived trace in a CI pipeline without interaction (`loupe`,
`ci-gates`).

### Milestone D4 "Sonde"

**D4-1 — Compiled-inactive gate.**
Process-wide `AtomicU64` (off/capture/pause) + per-engine-instance override,
`Relaxed` load at span boundaries only. Never mutates engine state.
DoD: inactive-regime cost measured in the D1-5 bench, no allocation; ADR-0103
draft (`sonde`, `perf`, `adr`).

**D4-2 — Hot attach.**
Unix-domain socket endpoint (filesystem permissions, loopback only), activation
by toggling the atomic, **header snapshot** at attach (live handles, counters,
versions), lazy pulls of forms while connected, full flush at detach
(`--archive`).
DoD: attach to a program started without tracing; the trace produced from the
attach is replayable on its own (`sonde`).

**D4-3 — Gate verbs and densities.**
`pause`/`continue`/`break`/`until`/`step` during execution (asynchronous,
effective at the next boundary), stop-the-world parking at a boundary, **fine**
density in manual stepping (park at every event) and return to **coarse** on
`continue`. A parked runner consumes exactly one command.
DoD: live pause/step/resume on philosophers saturation; CI property: trace of a
paused session byte-identical to the bare run (`sonde`, `ci-gates`).

**D4-4 — ADR-0103.**
Amendment of the "not a debugger" non-role of the Liseuse doc (the Sonde is a
brick distinct from the Liseuse); global gate state (attach ergonomics vs
purity); "binary built with lisage" limit; non-blocking default; **language
rule: tooling code in English**.
DoD: ADR accepted per the repository process (`adr`, `docs`).

### Milestone D5 "Oracle"

**D5-1 — Sampled semantic differential.**
At `OpEnd`/`Round`: sample witness valuations, evaluate input and output
(reusing `ddd::evaluer` and the `lisage-lang` interpreter), report any
denotation divergence.
DoD: a `Filtre` with a faulty predicate caught at the faulty operation of a
chain (`oracle`).

**D5-2 — Fixpoint growth and dashboard.**
Check `X_k ⊆ X_{k+1}` on a sample at each round (any decrease = bug in `τ` or
`⊕`); saturation dashboard: active levels (AutoDetectee narrowing), modified
levels, growth, convergence.
DoD: injected non-monotone `τ` test caught at the faulty round, not at the end
(`oracle`, `tests-pbt`).

### Milestone D6 "Screens"

**D6-1 — WASM index and offline page.**
Compile the index brick to WASM (same Rust source), offline Loupe page:
self-contained HTML embedding trace + index + SVG renders, full navigation
(steps, predicates, span tree), buttons = REPL bus commands.
DoD: page committed, openable offline without infra, on a philosophers trace
(`ecrans`).

**D6-2 — Loopback gateway and live page.**
`lisage-debug gateway`: Sonde socket ↔ loopback WebSocket bridge (never
exposed). Same page, WASM index fed by the v3 stream, gate verbs by
keyboard/buttons.
DoD: the live page drives pause/step/break of an attached program (`ecrans`,
`sonde`).

**D6-3 — DAP adapter (optional).**
`stackTrace` ↔ visit stack, `variables` ↔ `h`/`s` per frame, breakpoints ↔
serialized AST.
DoD: VS Code: `round 12` breakpoint operational on philosophers (`ecrans`).

## 6. Global acceptance scenario (end-of-tooling barrage)

On philosophers n=4, full saturation traced:

1. the v3 trace replays identically and the mirror rebuilds == real state (CI);
2. in under 5 minutes in Loupe: (a) first unshared node created at round k,
   (b) the explanation of a memo miss (version bump), (c) `explain` of an
   output node (op, round, visit, `h`, reduction);
3. comparison of rounds 11 and 12 via the folded span tree;
4. hot attach to the same program relaunched without tracing: capture, then
   pause, then fine `step`, then detach with archive;
5. an injected faulty `τ` is reported by the oracle at the faulty round;
6. the offline page of the scenario is committed and opens without infra; the
   performance budgets of the three regimes are met.
