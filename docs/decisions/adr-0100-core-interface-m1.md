# ADR-0100 — M1 core interface (status: **accepted** 2026-11-11, human decision received in a needs-human session)

> Draft written in the M1-0 PR: a project ADR is *drafted* in a PR
> (numbered from 0100 upward); only the human accepts it. The 0001-0099
> range belongs to the template.

## Context

M1-0 (issue #6) fixes the core interface: traits `Pool`/`KeyAlgebra`/
`Controller`/`Observer`, types `NodeId`/`Handle`/`Anchor`/`Level`/`Form`,
`Send`/`Sync` bounds, error policy (roadmap § 10).

## Proposed decisions

1. **Contract placement**: identifiers and traits live in
   `lisage-algebra` (level 2 sees the port "store/retrieve a node by
   key", proposal § 8); `lisage-core` implements the port and exposes
   `Observer`/`CoreError` by re-exporting the contract types. The
   workspace graph (boundary test) forbids `algebra → core`.
2. **Arena + `NodeId` in creation order, never moved** (proposal § 6).
3. **Form**: `Form::Node { level, arcs: BTreeMap<Key, NodeId> }`, default
   edge stored like the others; nodes owned by the arena, referenced
   by `NodeId` (no shared `Arc` in M1).
4. **`Send`/`Sync` bounds fixed now** on the four traits:
   single-threaded execution in M1, parallelism beyond M5.
5. **Errors**: internal `core`/`algebra` invariants = `assert!`;
   boundaries = typed errors (`PoolError`, `CoreError`, `LangError`).
6. **Logging**: narration through **structured DEBUG events** with
   named fields, pinned by tests (human decision of 2026-11-11,
   needs-human session — the constitutional contract "logging replaces
   comments" applies). No logging dependency added for now (escalation
   list): as long as no logging crate is authorized, emission goes
   through the existing `Observer` channel. The migration of existing
   modules (doc-comments + tests) to DEBUG events is progressive, module
   by module.

## Consequences

- Boundary test extended to signatures: bounds checked in the sources
  (lisage-bench) and at compile time (lisage-core).
- C1 is tested by PBT on the handles (controller, anchor), not on bare
  nodes (spec § 2.4.3).

## Rejected alternatives

- Contracts in `core`: impossible without inverting the workspace graph
  (architecture change, escalation).
- `Arc<…>` shared between pool/transductions: `NodeId` is enough and stays
  simple in M1.
