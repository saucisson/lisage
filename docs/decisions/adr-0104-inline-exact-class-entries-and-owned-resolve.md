# ADR-0104: Inline exact-class entries and owned `Pool::resolve` (MEM-16)

- Status: **draft** (drafted by the agent in the MEM-16 PR, issue #132;
  only the human owner accepts it)
- Date: 2026-11-18

> Draft written in the MEM-16 PR: a project ADR is *drafted* in a PR
> (numbered from 0100 upward); only the human accepts it. The 0001-0099
> range belongs to the template.

## Context

MEM-16 (issue #132) declares a key width per level (`u8`/`u16`/`u32`
according to `cardinality()`) and requires the stride of the exact
classes to drop to `8·d + header` (one `(u32, NodeId)` per arc instead of
a 12-byte `(Key, NodeId)` in a boxed run), verified by a static test,
with no loss on open domains (the key/value run path unchanged).

The stride criterion is incompatible with the frozen `Pool::resolve ->
&Form` API (frozen since M1, ~100 call sites): a `&Form` return requires
the `Form` to be stored as-is in the entry at constant stride, while
`8·d + header` requires monomorphic per-degree entries that store the
arcs inline and hold no `Form` in memory.

The needs-human question was posted on issue #132 and answered by the
owner on 2026-11-18 (delegation to the agent's recommendation): take the
architecture-change path.

## Decision

1. **Exact classes store arcs inline.** The entry of an exact class `d`
   is `InlineEntry<d> { plan, live, level, arcs: [(u32, NodeId); d] }`,
   slot stride exactly `8 + 8·d` bytes (header: `plan` + `live` + `level`,
   padded to 8). The key is stored in its narrowed 32-bit form (the
   ordinal rank of the key in the level's enumeration, `KeyAlgebra::rank`),
   lossless on bounded domains. The terminal class 0 and the catch-all
   class keep the `NodeEntry` with the full `Form` — the key/value run
   path is unchanged on open domains (cardinality beyond 32 bits or no
   rank: `narrow` returns `None`, the node is routed to the catch-all).
2. **`Pool::resolve` returns an owned `Form`.** The signature changes from
   `Result<&Form<..>>` to `Result<Form<..>>`: an exact-class entry no
   longer holds a `Form`, so resolution rebuilds it, widening the narrowed
   keys back (`KeyAlgebra::widen`/`unrank`) and reapplying the canonical
   dense-family choice (MEM-12), so the resolved form is exactly what a
   boxed storage would have returned. The ~100 call sites are unchanged
   (`match resolve()? { Form::.. }` and `resolve()?.clone()` compile as-is).
   The hot paths inside the pool (unique-table `same_form`, GC marking)
   use borrowed accessors on the arena trait and never rebuild a form.
3. **Key widths are declared per level** by `KeyAlgebra::key_width`:
   `u8` when `|Val(ℓ)| ≤ 2^8`, `u16` up to `2^16`, `u32` up to `2^32`,
   `None` on an open domain. `narrow`/`widen` default to
   `rank`/`unrank` truncated to 32 bits.
4. **The unique table is unchanged semantically** (ADR-0103 invariant 1,
   C1): the hash-consing key remains the full-width semantic form; equality
   is decided through the stored entry, widening the narrowed keys — the
   form is never cloned into the table.

## Consequences

- Memory: an exact-class node of degree `d` costs `8 + 8·d` bytes per slot
  instead of `size_of::<NodeEntry>()` (~40 bytes) plus a boxed run of
  `12·d` bytes — roughly a 2-3× reduction on small degrees, measured by
  the existing `bytes_per_node` counter.
- `resolve` allocates a boxed run per call on exact classes; callers that
  only iterate arcs pay one allocation per resolved node. The pool's own
  hot paths (interning, GC) do not go through `resolve`.
- The class of a node now depends on the narrowability of its keys, not
  only on its degree: a small-degree node on an open-domain level goes to
  the catch-all class. The class map itself (width `b`, frozen window) is
  unchanged.
- `choose_dense_family` now uses checked arithmetic: an open domain
  (cardinality near `u64::MAX`) can never satisfy the dense threshold
  (previously a latent overflow).

## Alternatives rejected

- Keep `resolve -> &Form` and narrow only the boxed run (8 B/arc): the
  stride criterion `8·d + header` would not be met — rejected by the owner.
- Borrowed `NodeView` as the `resolve` return type: avoids the per-call
  allocation but rewrites all ~100 call sites' patterns; the owned `Form`
  keeps the migration mechanical, and the hot paths use borrowed
  accessors anyway.
