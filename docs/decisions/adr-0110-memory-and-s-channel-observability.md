# ADR-0110: Memory observability and complete channel S (the deferred ADR-0109 tracks)

- Status: **draft** (drafted by the agent in the memory observability PR;
  only the human owner accepts it)
- Date: 2026-11-24

> Draft written in the memory-and-S-channel PR: a project ADR is
> *drafted* in a PR (numbered from 0100 upward); only the human accepts
> it. This draft settles the two tracks that ADR-0109 deferred on the
> record: the event-vs-level granularity of the reference kernel, and
> the observability of the memories `M≠()` and the implication
> channel S.

## Context

ADR-0109 delivered the trace of the in-place firing order (`LocalFire`,
v6) and explicitly deferred two tracks so they were not silently
dropped:

1. **Event vs level granularity** — what is the chronological unit of
   the *reference* kernel's trace (`saturate`, `Round` events)?
2. **Memories `M≠()` and channel S** — the transduction-level memory
   and the synthesized channel were not fully observable.

At v6 the gaps were concrete. The runner (`crates/lisage-core/src/
transduction.rs`) traces its computation table (`MemoConsult`,
`MemoHit`/`MemoMiss`, `MemoInsert`) and its memory *writes*
(`MemoryWrite`), but:

- a **read** of the level memory by `φ_out` was invisible — a memory
  `M≠()` could decide a visit and the trace would not say which key
  was consulted, whether it hit, or what value came back;
- the **nil memory** (`M = ()`) had no traceable witness: nothing in
  the trace proved, per op, that the level memory stayed empty (version
  0, no entry) — the premise of the computed-table theorem had to be
  taken on faith;
- the **synthesized channel S** was incomplete: the runner emitted
  `Attr { channel: Synthesized }` after `φ_aggr` at internal-node
  misses only. A terminal visit (`ψ`) emitted no S, and a memo hit
  returned the memoized S silently — so a parent aggregating S over
  its children could aggregate inputs the trace never showed, and
  `count(synthesized Attr) != count(VisitExit)`.

The kernel-level questions (firing order, rounds) were settled by
ADR-0109; these tracks touch the *transduction runner*, below both
kernels — the fix applies to the reference kernel, the in-place kernel
and every pooled transduction at once.

## Options considered

For the granularity of the reference kernel (track 1):

1. **One event per fire of the reference kernel** (mirror `LocalFire`
   into `saturate`) — rejected: the round kernel's unit is the round
   (`Round` events), and the per-node chronology *is* already
   observable inside a round span through the D1-2 `StructureChange`
   events (one per completed visit). Adding per-fire events would
   double the event volume of the reference kernel without a driving
   view: the firing-order question is the in-place kernel's question,
   answered by ADR-0109.
2. **Per-level events** — rejected in ADR-0109 already (redundant with
   the fire number plus the visited level); same reasoning at the
   reference kernel, where the round already lists
   `active_levels`/`modified_levels`.
3. **Two vocabularies, one per kernel (chosen)** — the reference
   kernel keeps `Round`; the in-place kernel keeps `LocalFire`. The
   reference kernel gains nothing in this PR: its answer to
   "chronological unit" stays the round, and its per-node detail
   stays the `StructureChange` events inside the round spans.

For the memory reads (track 2):

1. **Snapshot events of the whole memory** at each op — rejected: the
   interleaving with the visit chronology is the point (which read
   decided which visit); snapshots only give posture, not use.
2. **Key-registration callbacks from `MemoryLevel` itself** —
   rejected: the level memory (`MEM-15` dense arrays) is a hot-path
   data structure; routing every `read` through an observer hook
   there would also trace reads that are not `φ_out` consultations
   (memo bookkeeping included). The trace is about the *transduction*,
   not the data structure.
3. **A probe wrapping the memory for `φ_out`** (chosen): the runner
   exposes its memory to `φ_out` through a `MemoryProbe` (a thin
   wrapper: `read` → one `MemoryRead` event when an observer is
   attached, then the underlying `MemoryLevel::read`). Only the
   consultations of `φ_out` are traced, exactly where the spec writes
   them (`φ_out(ℓ, h, {v ↦ (D', s)}, M)` — spec § 3.1), and with no
   probe cost when no observer is attached (one `Option` check).

For the per-op state and the S channel, the options were placement
questions; the chosen placements are the decision below.

## Decision

1. **`MemoryRead` at every `φ_out` memory consultation (D1-3).** The
   runner's `PhiOutContext.memory` becomes a `MemoryProbe` wrapping
   the runner's `MemoryLevel<M>`; each probe `read(node, level)`
   returns the stored value and emits
   `TraceEvent::MemoryRead { node, level, hit, value, span }`, where
   `value` is lazily rendered and **elided on a miss** (`TraceVal::Elided`)
   — a miss has no value to show. The event is contextualised into
   the innermost open span, which is the `Visit` span of the node
   being restructured (`φ_out` runs inside it). A transduction whose
   `φ_out` never reads memory (the nil memory `M = ()` never does —
   the type says why) emits nothing.

2. **`MemoryState` once per op, just before `OpEnd` (the nil-memory
   witness).** All three runner entry points (`run`, `run_same_pool`,
   `run_same_pool_from`) close the op with
   `TraceEvent::MemoryState { version, entries, span }` — the L1
   version and the stored-entry count of the level memory at the end
   of the op, contextualised into the `Op` span. A nil memory keeps
   `(0, 0)` at every op: the traceable witness that the level memory
   never falsified the computed table (each op starts on an empty,
   version-0 memory). A memory `M≠()` shows its effective bumps
   (version growth, entry count) per op. Placement just before
   `OpEnd` keeps the register-sequence shape of the trace documented
   in the tests: `OpEnd` stays the second-to-last event of an op, the
   last being the outer `SpanClose` — no view that anchors on
   `OpEnd` shifts.

3. **Channel S completed: one synthesized `Attr` per completed
   visit.** Every completed visit now emits
   `Attr { channel: Synthesized }`: internal-node misses after
   `φ_aggr` (unchanged), **terminal visits after `ψ`** (new), and
   **memo hits before returning** (new, re-emission: the memoized S
   flows up again). Invariant, stated and tested:
   `count(synthesized Attr) == count(VisitExit)` on a successful run —
   a parent aggregating S over its children never aggregates an input
   the trace never showed. This holds in both pool variants
   (`run_with_inner`, `run_with_same_pool_inner`); a failed visit
   emits no `VisitExit` and no S on the failing path.

4. **The reference kernel keeps `Round` (track 1 settled by
   documentation, no code).** The two kernels keep different
   vocabularies: rounds for the reference kernel, fires for the
   in-place kernel; per-node chronology stays observable through the
   `StructureChange` events inside round spans. Nothing in this PR
   changes the reference kernel's events.

5. **Trace version 7 (hard gate, English wire fields).** The trace
   format version moves 6 → 7: v7 adds `MemoryRead` (fields
   `node`/`level`/`hit`/`value`) and `MemoryState` (fields
   `version`/`entries`), English per the M6 charter (the v1-v5 French
   keys stay byte-unchanged). Unknown versions — v6 now included —
   are rejected explicitly (`0, 2, 3, 4, 5, 6, 99`): re-run to produce
   a v7 trace, no silent upgrade.

6. **Liseuse surfaces the memory channel like any other.** The
   debugger gains the event kinds `memory-read`/`memory-state`
   (`until` targets, `where` naming), the inspection `memory` view
   gains the read counters (`H hit / M miss`) and the per-op state
   lines (`v{V} e{E} @ {i}`), the predicates/index route
   `MemoryRead` with its node and level like the other node events,
   and the offline page serialises both events (committed artifact
   regenerated, per the byte-identity discipline).

## Consequences

- Every exhaustive match on `TraceEvent` had to be completed (core
  `set_span`/`span`, the liseuse serializer/deserializer, the offline
  page, the step-by-step ignore groups, the debugger, the predicates,
  the index, the bench observer) — the exhaustive matches again made
  the change mechanical.
- With no observer attached, the added hot-path cost is one `Option`
  check per `φ_out` memory read and one per op; with an observer, one
  event per read and one per op. The probe reads exactly where `φ_out`
  already read; no data-structure hook was added to `MemoryLevel`.
- The trace grows by construction: one `MemoryState` per op (4 in the
  n=4 philosophers artifact — the nil memory writes `(0, 0)` four
  times), one synthesized `Attr` per terminal visit and per memo hit
  (10 in the same artifact). The committed offline page artifact moves
  from 202 to 216 events and is regenerated by
  `test_d6_1_committed_page_artifact`.
- The v7 gate means every stale v6 trace is now rejected on read;
  re-running the producing program is the migration (documented in
  the deserializer tests).
- New invariants are tested at the core level
  (`crates/lisage-core/tests/memory_observability.rs`): miss-then-hit
  of the traced reads, the `(0, 0)` nil-memory witness before `OpEnd`,
  the per-op version growth of a writing memory, and
  `count(synthesized Attr) == count(VisitExit)` across a scenario
  covering the three visit kinds.

## Open questions (for the human owner)

1. `MemoryState` carries `(version, entries)`. Should it also carry a
   byte footprint of the level memory (the MEM-15 counters exist)
   for a golden memory profile per op, or is the volume counter in
   `MemoryWrite` enough for now?
2. Is the granularity settlement (two vocabularies, `Round` stays)
   final for the reference kernel, or is there a view you want that
   the round + `StructureChange` pair cannot answer?
3. The memo-hit re-emission of S duplicates a value that the
   computing visit already emitted (`Attr` at `MemoInsert` time).
   Acceptable duplication for the aggregation argument, or should a
   future format mark the re-emitted ones (e.g. an origin field)?
