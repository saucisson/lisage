# ADR-0103: Arc storage port and per-controller-instance layouts (MEM-A track)

- Status: **draft** (drafted by the agent in the MEM-1 PR, issue #117;
  only the human owner accepts it)
- Date: 2026-11-18

> Draft written in the MEM-1 PR: a project ADR is *drafted* in a PR
> (numbered from 0100 upward); only the human accepts it. The 0001-0099
> range belongs to the template.

## Context

The MEM-A track (issues #117-#123, milestone MEM-A) ports the arc storage
behind a `Pool`-level `ArcStore` port and gives each controller instance its
own layout, so that layout effects become measurable per class of diagrams
instead of being global. Two findings of the critical analysis drive the
track:

- **Content addressing vs. usage addressing** (canvas
  `docs/critical-analysis-dd-ir-design-note-and-memory-locality.md` § 4.4):
  hash-consing assigns identities by content, placement wants to arrange by
  usage; the default creation-order layout captures construction affinity,
  not operational affinity. Measuring the difference requires layouts to be
  switchable *per class*, which a single flat arena cannot express (§ 4.6:
  compaction on/off must be controlled against density and hashing effects).
- **Order/operations entanglement** (canvas § 5.3): inter-level information
  must be stored somewhere; co-locating it with the node solves the semantic
  and the locality problem together. The arc store is where that co-location
  lands, so its layout is part of the physical plan, not an afterthought.

Current state (M1-M7): `lisage-core::arena::Arena` is a two-plane
hash-consed pool keyed by `(ControllerId, Anchor, Form)` — the semantic
form is already the hash-consing key — with mark-and-sweep GC and profiled
compaction; `NodeId(u32)` is assigned in creation order; the
`core`/`algebra` boundary is enforced by a workspace-graph test
(ADR-0100); schemas are runtime data parameterizing the `KeyAlgebra`
(M1-3).

## Options considered

1. **Keep one flat arena, switch layouts globally.** Rejected: a global
   layout switch conflates the classes (canvas § 4.6 pitfall 1) and makes
   the MEM-7 pilot measurement (layout on/off confrontation, issue #123)
   unable to attribute an effect to a class of diagrams.
2. **Per-class arenas behind an `ArcStore` port, tripartite `NodeId`
   (chosen).** Each controller instance declares its class; the class map
   is frozen at pool creation; the identifier carries the class so a handle
   is self-describing and the store can route without a lookup.
3. **Fully dynamic class map (classes created at runtime).** Rejected:
   breaks identifier stability — a class bit pattern assigned twice would
   recycle identifiers — and makes the frozen class map unverifiable.

## Decision

The MEM-A track is governed by **seven invariants**:

1. **Hash-consing key = semantic form.** The unique table is keyed by
   `(ControllerId, Anchor, Form)`; two nodes with the same key are the same
   node. No layout decision may change the key.
2. **Frozen class map.** The set of classes and their layouts is fixed when
   the pool is created; adding a class afterwards is a new pool, never a
   mutation of the live one.
3. **Identifier space never recycled.** `NodeId` is append-only between
   compactions; compaction renumbers globally and records the remap table;
   a stale identifier is an `UnknownNode` error, never a silent reuse.
4. **Verifiable core/algebra boundary preserved.** The `ArcStore` port
   lives in `lisage-algebra` next to `Pool`; `lisage-core` implements it;
   the workspace-graph boundary test keeps rejecting `algebra → core`
   (ADR-0100).
5. **Schema as runtime data.** Layouts, cardinalities and defaults are
   declared by the schema and read at runtime (M1-3); nothing about the
   layout is hard-coded in the store.
6. **Tripartite identifier budget.** `NodeId(u32)` = **class bits + index
   bits + tag bits** (see sizing below).
7. **Order: interface → tests → implementation.** Every MEM-A issue lands
   the port/trait first, then the tests (including the differential and
   boundary tests), then the implementation (roadmap § 1 execution model).

### Sizing: the tripartite budget of the identifier

`NodeId` is a `u32` split into three fields, tag on top:

| Field | Width (default) | Role |
| ----- | --------------- | ---- |
| tag   | 4 bits          | **arbitrated by MEM-13 (#129)**: bit 0 = CUDD-style complement bit (`TAG_COMPLEMENT`), bits 1..=3 = shift nibble of M4-2 (`TAG_SHIFT_MASK`) |
| class | `b` bits, runtime-derived (8 max) | index into the frozen class map |
| index | `32 − 4 − b` bits | position inside the class arena |

The arc-key width `b` is **not** part of the identifier: it is the smallest
integer with `2^b − 2 ≥ max_ℓ C(ℓ)`, derived from `cardinality()` at
runtime (MEM-8, #124), with a compile-time window `b_max` (default 8) and a
catch-all class beyond the window, without loss of correctness. Known
sizing figures: BDD → `b = 2` (index 26 bits ≈ 67M per class); philosophers →
`b = 3` or `4`; open domain → low default with declared overload.

#### Tag arbitration (MEM-13, #129) — decided

The 4-bit tag field is **reserved unconditionally** at the top of the
identifier, not borrowed from the key bits on small domains. Motivation:

- The class width `b` is runtime data (schema-dependent), so borrowing key
  bits would make the identifier layout depend on the domain size — two
  pools over different schemas would split the same 32-bit space
  differently, and a handle could not be read without its pool's `b`.
  A fixed tag width keeps the layout pool-independent and the decoder
  parameterized by `b` alone.
- Cost: the index field loses 4 bits. At the typical `b = 2` (BDD) the
  index keeps 26 bits (≈ 67M nodes per class); at the window maximum
  `b = 8` it keeps 16 bits (≈ 65k per class) — the same order as the
  negative already recorded below, accepted deliberately.
- Split inside the tag: bit 0 is the complement bit (CUDD-style edge
  transformer), bits 1..=3 the per-arc shift nibble of M4-2 (shift amount
  0..=7). The two uses are disjoint (`TAG_COMPLEMENT & TAG_SHIFT_MASK = 0`),
  so a complemented edge can still carry a shift. M4-2 does not open
  without this arbitration (issue #129 criterion).

## Consequences

- Every PR of the MEM-A track (and of MEM-B/C/D, which build on the
  port) cites ADR-0103 in its body.
- The reviewer can verify each invariant independently: (1) by the unique
  table's key type, (2) by a test that mutating the class map after pool
  creation fails to compile, (3) by a PBT that a stale `NodeId` errors
  instead of aliasing, (4) by the existing boundary test, (5) by schema
  parameterization tests, (6) by decoder round-trip tests per field,
  (7) by the PR order itself.
- Negative: the tripartition shrinks the per-class index space (20 bits
  default ≈ 1M nodes per class); classes needing more must declare a
  narrower class field at build time — a deliberate trade-off, recorded
  here so it is revisited consciously.
- Negative: freezing the class map means a new diagram class requires a
  new pool; cross-class sharing goes through handles, not through merged
  arenas.

## Open questions — gates

- **MEM-7 (#123)**: pilot measurement (randomized reproducible IDs,
  philosophers / sliding window) and the generalization gate — constant
  factor ⇒ layout, complexity gap ⇒ stop and escalate.
- **MEM-13 (#129)**: arbitration of the tag bits (CUDD-style complement
  bit vs. shift nibble for M4-2). M4-2 does not open without this
  decision; the sizing above is merged with it.

## References

- Canvas: `docs/critical-analysis-dd-ir-design-note-and-memory-locality.md`
  § 4.4, § 4.6, § 5.3.
- ADR-0100 (core interface, boundary), ADR-0101 (PR budget).
- Issues #117-#123 (MEM-A), #124/#129 (MEM-B gates).
