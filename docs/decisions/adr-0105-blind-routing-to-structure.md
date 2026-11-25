# ADR-0105: Blind routing to structure — the concurrency invariants INV1-INV10

- Status: **draft** (drafted by the agent in the P0-3 PR, issue #138;
  only the human owner accepts it)
- Date: 2026-11-19

> Draft written in the P0-3 PR: a project ADR is *drafted* in a PR
> (numbered from 0100 upward); only the human accepts it. The 0001-0099
> range belongs to the template.

## Context

Milestone P0 installs the reading grid before any parallel code: the
concurrency design must be decided *before* P1-P4 implement it, so that the
equivalence harness of P0-2 (issue #137, `par_equivalence.rs`) has a fixed
target to falsify (PAR-1..PAR-4). The state of the art
(`docs/state-of-the-art-decision-diagrams.md`) places the project between
two parallel lineages: Sylvan (multi-core, sharded **per variable**) and the
Paris school / pnmc (DDD/SDD lineage, level-wise apply-reduce saturation,
massively parallel Petri net model checking). Lisage's nodes are not
Sylvan's: a node is `(ControllerId, Anchor, Form)` with relative forms and
controller-designated defaults, so no existing engine's table can host them
(proposal § "sharp reversal").

## Decision

**Routing is blind to structure.** The unique table is a single
content-addressed table: a node is routed by the avalanche hash of its full
semantic key `(ControllerId, Anchor, Form)` and verified by full equality at
the probe. Nothing in the routing path inspects the level, the variable, or
the degree of the node. The mono-thread pool is the one-worker case of the
same table.

The ten invariants the parallel milestones (P1-P4) must preserve:

- **INV1** — Content-addressed routing: the unique table routes on the hash
  of `(ControllerId, Anchor, Form)`; the full key is verified at the probe
  (P3-1).
- **INV2** — Lock-free table access: buckets are CAS insert-or-find; no
  global lock guards the unique table (P3-1).
- **INV3** — Flat NodeIds: positions are claimed by CAS (tag bit + free
  index per thread, rehash without displacement); no central allocation
  counter (P3-2).
- **INV4** — The shadow plane is never on the intern path: per-worker
  addition journals, merged at round barriers; the index stays
  reconstructible (P3-3).
- **INV5** — Level memory versions are frozen during a round: a round reads
  a consistent version of every level's memory (P3-3, P4-1).
- **INV6** — GC without compaction: parallel mark by slices, free lists, no
  remapping table; GC runs at round barriers only (P3-4).
- **INV7** — Round barrier: workers synchronize at each round boundary and
  agree on the active-level set (work-stealing over it); no worker crosses a
  round alone (P4-1, P4-2).
- **INV8** — Sharded lock-free memo: key `(fingerprint, node, h, version)`;
  per-worker local caches, lazy publication (P2-2).
- **INV9** — The variable order is a contract of the controller: no dynamic
  reordering anywhere in the engine (inherited from the proposal).
- **INV10** — Independent operations read the shared input pool read-only
  and each writes its own output pool (P1-1).

The crate-level signatures these invariants rely on (the unidirectional
dependency graph: `lisage-lang` → `lisage-algebra` → `lisage-core` /
`lisage-plan` → `lisage-bench` / `lisage-liseuse` / `lisage-backend`) are
validated by the boundary test `crates/lisage-bench/tests/frontieres.rs`.

## Alternatives rejected

- **Sharding the unique table by level** (one table per level, as level-wise
  engines naturally do): rejected. Routing must stay blind to structure —
  a node's identity depends on its children's NodeIds, which live in other
  shards, so level-sharding leaks structure into routing, breaks
  cross-level sharing of subtrees, and turns every CAS insert-or-find into a
  cross-shard coordination. The single content-addressed table keeps INV1
  and INV2 absolute: one hash, one probe, one CAS.
- **Sylvan-style per-variable sharding**: rejected for the same reason, and
  because Sylvan's nodes are `(variable, low, high)` — classical BDD nodes
  with a fixed order — while Lisage's forms are relative and
  controller-parameterized. Sylvan stays what the proposal decided: a
  **witness** (differential oracle on the classical BDD instance, bench
  baseline), never a backend.
- **Paris school / pnmc style** (level-wise apply-reduce sweeps,
  time-forward processing on a biplan index): kept for the *saturation
  semantics* (Ciardo/Hamez policies, P4-2) but not for node routing; pnmc's
  parallelism is exactly the level-sharding rejected above.

## Plan B: Adiar as the documented fallback

If **PAR-2/PAR-5 are not held after P3**, or if the memory need exceeds the
speed need, the fallback is the Adiar route (spike SP-1, issue #153):
level-wise sweeps (apply-reduce, time-forward) on the existing biplan index,
decision note only, no code. The trigger condition is recorded here so that
P3's results are judged against it mechanically, not rhetorically.

## Consequences

- P1-P4 implement against INV1-INV10; the P0-2 equivalence harness
  (`par_equivalence.rs`) is the falsifier for PAR-1..PAR-4 on the same
  inputs (node counts, semantics).
- The mono-thread path remains the one-worker case: no separate
  single-threaded engine is maintained.
- The boundary test `frontieres.rs` keeps the crate signatures honest while
  the parallel modules (`parallel_ops.rs`, `ConcurrentArena`) land.
