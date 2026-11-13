# Lisage parallelism: work specification

Work plan to introduce shared-memory parallelism in the decision-diagram engine
of `saucisson/lisage` (branch `daily`), in response to the unique-table
contention problem. The plan extends the existing roadmap (parallelism was
pushed beyond M5 there) and builds on the confrontation with field experience
(Sylvan, the Paris school/libDDD/pnmc, Ciardo/Hamez saturation, Adiar,
quantum DDs).

## 1. Context and objective

The current `Arena` is a single-threaded hash-consed pool: `intern(&mut self)`
over a unique `HashMap<(ControllerId, Anchor, Form), NodeId>`, a `Vec` of
nodes, a shadow index per level, one memo per `Transducteur`. Saturation
(`saturate.rs`, Ciardo/Hamez policies) concentrates work per level: any
level-based split of the table, or any global lock, re-serializes precisely the
parallelizable phases.

Objective: a correct-by-construction multi-core engine (canonicity and node
counts identical to single-thread), scalable on operations and on saturation,
without regressing the sequential path.

Chosen design (from the analysis and the confrontation with the field):

- **flat, content-addressed** unique table, lock-free buckets (CAS), routing
  blind to structure (never by level nor by controller);
- flat, level-neutral `NodeId`s, claimed by CAS;
- the level structure organizes only **derived** structures (shadow plan), kept
  deferred and merged at round barriers;
- parallelism in **the engine** (`Transducteur`, `Arena`), never in the
  declarative transductions;
- GC without compaction (free lists), removal of remapping.

## 2. Scope and out of scope

**Scope**: shared-memory multi-core operations and saturation; deterministic
equivalence; instrumentation and contention benches; design ADR; local Sylvan
witness (outside CI, like CUDD).

**Out of scope**: distributed memory / MPI; disk parallelism (Adiar) beyond the
study spike; GPU; semantic changes to transductions; mass renaming of existing
French identifiers.

## 3. Conventions: the code is in English

**All new code is in English**: identifiers (types, functions, variables,
modules), rustdoc comments (`///`, `//!`), error messages, test and benchmark
names, associated scripts and tools.

Application details:

1. **New modules** (`concurrent_arena.rs`, `parallel_ops.rs`, concurrent ports,
   bench harness): exclusively English.
2. **Existing code with French identifiers**: not renamed within the scope of
   this plan; new modules call existing items under their current names. Any
   renaming is a separate follow-up.
3. **Commits and PR descriptions**: in English. Design documents (`docs/`,
   ADRs, specifications) remain in French, per repository conventions.
4. **FR → EN glossary** (to keep docs and code aligned): contrôleur →
   controller ; table unique → unique table ; plan d'ombre → shadow plan /
   level index ; mémoire de niveau → level memory ; ancre → anchor ; pli →
   fold ; saturation → saturation ; internement → interning ; routage aveugle à
   la structure → structure-blind routing ; rendez-vous → rendezvous (dedup
   point).
5. Existing CI gates apply as-is: `rustfmt`, `clippy -D warnings`,
   `cargo-nextest`, coverage ≥ 90 %, `cargo-deny`, `cargo-audit`, no C/C++
   dependency in CI.

## 4. Design invariants

| # | Invariant | Content |
|---|---|---|
| INV1 | Structure-blind routing | The bucket depends on the mixed content `(anchor, arcs, payload)` through an avalanche hash (wyhash/xxhash/SipHash). `ControllerId` and `Level` are **checked** at probe time, never routed on. No nested `controller → …` table, no routing prefix on a structural component. |
| INV2 | Lock-free buckets | Insert-or-find by CAS on buckets (position claim bit, publication, fallback to the winner). Duplicated work is accepted: two workers may compute the same operation, the first published result wins. |
| INV3 | Flat, neutral `NodeId`s | The `NodeId` is a position in the data array, claimed by CAS (bit D), with a free-position index per thread. Never level slabs in the addressing: level bits must not leak into arcs, hence into the hash entry. |
| INV4 | Derived structure only | The level structure organizes only derived structures (shadow plan), rebuildable, kept off the hot `intern` path. |
| INV5 | Round barriers | Saturation rounds are the synchronization points: merge of index journals, freeze of the L1 version of level memory during the round. |
| INV6 | GC without compaction | Parallel mark by slices, free lists by position; no more remapping table. GC runs at round barriers or in idle phases, never during an active round. |
| INV7 | Deterministic equivalence | Any multi-core execution produces the same node counts and the same semantics as single-thread (canonicity by construction, A3; C6 differential as guardrail). Individual `NodeId`s may differ between runs; canonical forms do not. |
| INV8 | Memo without hotspot | Key `(fingerprint, node, h, version)`; per-worker local caches with lazy publication to the sharded lock-free shared table. |
| INV9 | Engine, not operations | Parallelism lives in the engine (`Transducteur`, `Arena`); transductions stay declarative and pure. The `Transduction` contract is not modified by parallelism. |
| INV10 | Shared read | The input pool is never mutated by a read (`resolve: &self`); `φ_out` is the only write. Several operations read the same input pool in parallel without a lock. |

## 5. Falsifiable statements (verification labels)

| Label | Statement | Falsification |
|---|---|---|
| PAR-1 | Equivalence: for any input, any multi-core execution has the same node count and semantics as single-thread. | PBT + differential against the reference interpreter and the mini-BDD witness (extension of C6). |
| PAR-2 | Routing uniformity: the distribution of interns per bucket is uniform, independent of level and phase (including saturation). | Statistical test on pool metrics (chi² on the per-bucket distribution, per round). |
| PAR-3 | Rendezvous: two concurrent interns of the same reduced form return the same `NodeId`; no duplicate form survives. | Multi-thread PBT; post-execution uniqueness check on the table. |
| PAR-4 | No global lock: no mutex on the `intern` path; a blocked worker never prevents others from progressing. | Path audit (review + design assertion); bench with an artificially slowed worker. |
| PAR-5 | Scalability: speedup ≥ 4x on 8 cores on the reference benches (large-n philosophers saturation, massive unions), with identical result. | Criterion, baselines published in `bench-results/`, no regression of the 1-thread path. |
| PAR-6 | Barrier consistency: at each round barrier, the shadow plan and level-memory versions are complete and consistent. | PBT: index reconstruction and comparison; versions observed frozen during the round. |

## 6. Labels

- **Areas** (existing, reused): `area/core`, `area/algebra`, `area/bench`,
  `area/liseuse`, `area/docs`, `area/infra`.
- **Types** (existing): `type/feat`, `type/test`, `type/doc`, `type/spike`.
- **Milestones** (new): `jalon/P0` … `jalon/P4`.
- **Statements** (new): `PAR-1` … `PAR-6`. Each issue cites its statements in
  the body; each PR closes with `Closes #n` (repository convention).

## 7. Milestones and issues

Execution order per milestone, per the repository model: **interface → tests →
implementation**, with just-in-time escalation via `notify` remaining the rule
on any undecided point.

### P0 — Measure before building (1 cycle)

| Issue | Title | Content | Acceptance criterion | Statements |
|---|---|---|---|---|
| P0-1 | Contention bench | Instrument existing metrics (shares, creations): intern distribution per level, R-share and memo hit rates, saturation phase profile. Anchored criterion bench (philosophers, saturation n=5+). | Baselines published in `bench-results/`; CI rejects time regressions. | PAR-2 (measurement) |
| P0-2 | Multi-thread equivalence harness | PBT mono vs multi on the same inputs (node counts, semantics), written red before any implementation; census of existing tests comparing `NodeId`s (to be requalified as canonical-form comparisons). | Harness green on the current single-thread, ready to falsify PAR-1..PAR-4. | PAR-1 |
| P0-3 | Design ADR | ADR "Structure-blind routing": invariants INV1-INV10, decision against level sharding (and why), position vs Sylvan (per variable) and vs the Paris school/pnmc; Adiar plan B documented as fallback condition. | ADR merged in `docs/decisions/`; boundary tests validate the signatures. | contract |

### P1 — Inter-operation parallelism (1 cycle)

| Issue | Title | Content | Acceptance criterion | Statements |
|---|---|---|---|---|
| P1-1 | Independent operations, separate pools | Module `parallel_ops.rs`: parallel (rayon) execution of independent transductions reading the same input pool in shared read, each in its own output pool (INV10). | PAR-1 PBT on n concurrent operations; bench shows the gain on multi-request workloads. | PAR-1 |
| P1-2 | Result merging | Pairwise parallel union of output pools (merge at barriers), including inter-pool sharing. | Equivalence to sequential merges (PBT); scaling bench vs sequential merge. | PAR-1 |

### P2 — Fork-join engine (2 cycles)

| Issue | Title | Content | Acceptance criterion | Statements |
|---|---|---|---|---|
| P2-1 | Fork-join of children in the `Transducteur` | Parallel traversal of children above a calibrated cut-off, sequential below; at this stage the output pool stays protected (correct first). The cut-off is calibrated on the P0-1 measurements. | PAR-1 green; cut-off curve published; no 1-thread regression. | PAR-1, PAR-5 |
| P2-2 | Concurrent memo | Lock-free sharded memo, key `(fingerprint, node, h, version)`; per-worker local caches, lazy publication (INV8). | PBT memo hit/miss identical to single-thread; memo scaling bench. | PAR-1 |
| P2-3 | Buffered observer | `Observer` trait buffered per thread (never on the hot path); the Liseuse rebuilds per-node order. | Readable multi-thread trace in the Liseuse; zero measurable serialization on the hot path. | — |
| P2-4 | Concurrency boundaries | Boundary test extended to parallel signatures; the `intern(&mut self)` debt documented as pre-P3; `Send`/`Sync` bounds already frozen (M1-0) verified in use. | `clippy -D warnings` and boundaries green; clean `cargo doc`. | contract |

### P3 — Concurrent pool (3 cycles)

| Issue | Title | Content | Acceptance criterion | Statements |
|---|---|---|---|---|
| P3-1 | `ConcurrentArena`: flat content-addressed table | Hash array + data array à la Sylvan: CAS buckets, insert-or-find; routing on avalanche-hashed `(anchor, arcs, payload)`, full check of `(ControllerId, Anchor, Form)` at probe (INV1, INV2). Single-thread becomes the one-worker case. | PAR-2 and PAR-3 green; C6 differential unchanged. | PAR-2, PAR-3 |
| P3-2 | Flat `NodeId`s | Positions claimed by CAS (bit D, per-thread free index, rehash without moving); removal of the central allocation counter (INV3). | PAR-3 green; bench shows allocation without measurable hotspot. | PAR-3, PAR-4 |
| P3-3 | Deferred shadow plan | Per-worker add journals merged at barriers; never on the `intern` path (INV4, INV5). Rebuildable index, eventual consistency. | PAR-6 green; zero overhead on the `intern` path. | PAR-6 |
| P3-4 | GC without compaction | Parallel mark by slices, free lists; removal of the remapping table and its barrier (INV6). GC at round barriers only. | PBT live sets identical; GC metrics published; remap removal effective. | PAR-1, PAR-6 |
| P3-5 | Concurrent `Pool` port | Contract migration: `intern` with interior mutability or a dedicated concurrent port (`PoolPar`), ADR "abandon compaction" merged; the single-thread version remains the reference path for tests. | Boundaries green; `cargo doc`; ADR merged. | contract |

### P4 — Parallel saturation (2 cycles)

| Issue | Title | Content | Acceptance criterion | Statements |
|---|---|---|---|---|
| P4-1 | Parallel rounds | Nodes of an active level processed in parallel; barrier per round; level-memory versions frozen during the round (INV5). | PAR-1 on philosophers n=5 and up; per-round scaling bench. | PAR-1, PAR-6 |
| P4-2 | Ciardo/Hamez policies under parallelism | Both policies as instances of the same fixpoint; active-level set → work-stealing; Hamez detection unchanged. | Same fixpoint as single-thread (PBT); published comparison of the two policies. | PAR-1, PAR-5 |
| P4-3 | Sylvan witness and baselines | Differential against Sylvan (local Docker container, outside CI, like CUDD); publication of final baselines. | Report in `bench-results/`; PAR-5 demonstrated on 8 cores. | PAR-5 |

### Conditional spikes (outside milestones, human decision required before code)

| Issue | Title | Content | Trigger condition | Criterion |
|---|---|---|---|---|
| SP-1 | Adiar study | Level sweeps (apply-reduce, time-forward) on the existing bi-plan index; decision note only. | PAR-2/PAR-5 not met after P3, or memory need > speed need. | Note in `docs/`, no code. |
| SP-2 | Dockerized Sylvan witness | Local Sylvan LDD image, differential measurements. | From P4-3, or earlier if the ADR asks for it. | Local measurement script, `#[ignore]` in CI. |

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Insufficient avalanche hash → invisible hotspots | INV1 mandates a strong hash; PAR-2 is a permanent statistical test; the lock-free choice is documented in the ADR. |
| Deep recursion + fork-join (stack) | Calibrated cut-off (P2-1); optional de-recursivation documented if measured necessary. |
| Existing tests comparing `NodeId`s | Census from P0-2; requalified as canonical-form comparisons. |
| `Observer` on the hot path | Per-thread buffering from P2-3, before P3. |
| Locality loss due to flat `NodeId`s | Iteration via the shadow plan (level order) + prefetch; the trade-off is documented in the ADR (INV3 vs locality). |
| Heisenbug concurrency errors | Systematic multi-thread PBT + C6 differential at each step; never two invariants enacted in the same PR. |

## 9. Program exit criteria

1. PAR-1 to PAR-6 green, falsified by the harness, baselines published.
2. Equivalence to single-thread over the whole A/B/C triptych (interpreter,
   fixed-order DD, Lisage) and against the mini-BDD witness; local Sylvan
   differential.
3. No regression of the 1-thread path (budget: to be defined with the P0-1
   measurements).
4. ADRs merged (design, compaction, `Pool` port), invariants INV1-INV10 cited
   in code by cross-reference comments.
5. CI gates unchanged; no C/C++ dependency in CI; all new code in English
   (identifiers, comments, commits).
