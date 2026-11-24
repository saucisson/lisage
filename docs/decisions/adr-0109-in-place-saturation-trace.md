# ADR-0109: In-place saturation — trace of the firing order (M≠() and channel S deferred)

- Status: **draft** (drafted by the agent in the in-place firing-order
  trace PR; only the human owner accepts it)
- Date: 2026-11-24

> Draft written in the firing-order trace PR: a project ADR is
> *drafted* in a PR (numbered from 0100 upward); only the human accepts
> it. This draft also records the two deferred tracks of the original
> three-track plan so they are not lost.

## Context

PR #397 delivered the in-place saturation kernel (`crates/lisage-core/src/
inplace.rs`): instead of global rounds, the kernel fires **locally at
nodes**, in depth-first post-order — a node fires its own level
(`τ` restricted to `[level]`), the result is unioned in place, the
traversal re-runs the children that the fire may have changed, and the
root loops on its own level until stabilisation. `ResultSaturation.
iterations` counts the local fires. On the philosophers benchmark
(n = 13), the kernel measured 47.3 ms and 245 fires with 129 live
nodes, against 96.1 ms for ciardo and roughly 1 500 ms for the naive
kernel; `saturate` stays the reference kernel and `PolicySaturation::
AutoDetected` remains a rounds-only heuristic, ignored by the in-place
kernel.

But the trace of the new kernel was silent. Liseuse reads saturation
through `Round` events — one per global round of the reference kernel —
and the in-place kernel emits none: a reviewer replaying an in-place
trace could see the runner's ops but not the firing order, could not
tell a productive fire from the sterile fire that closes a local
fixpoint, and could not check `ResultSaturation.iterations` against
the trace. The draft that motivated this PR therefore carried three
tracks:

1. **Granularity — event vs level.** What is the chronological unit of
   the saturation trace: the event, the level, or the round?
2. **Memories `M≠()` and channel S.** The kernel's per-level runner
   memoises (`MemoHit`/`MemoMiss` exist), but the transduction-level
   memory `M≠()` of the nil-memory theorem and the implication channel
   S are not observable in the trace.
3. **Firing-order trace.** Make Liseuse replay the order in which the
   in-place kernel fired.

## Options considered

For the chronological unit of the in-place trace:

1. **Reuse `Round` events** (emit one per local fire, `numero` = fire
   number) — rejected: the field names lie. `Round` carries per-level
   lists (`niveaux_actifs`, `niveaux_modifies`) of a global round; a
   local fire is a different animal, and overloading the event would
   make every round-based view (convergence, `diff_rounds`) silently
   wrong.
2. **Per-level events** (one event per level per fire) — rejected:
   redundant with the fire number plus the level of the visited node
   (the runner restricted to `[level]` fires only there — documented in
   `inplace.rs`), and it would double the event volume on the hot
   path.
3. **One event per local fire** (chosen): a `LocalFire` event carrying
   the fire number, the level, the visited node, the local image and
   the merged node — the smallest payload from which level granularity
   is derivable and the firing order is explicit.

For the transport:

- **Silent extension of v5** — rejected: the trace format versioning
  rule (documented in the v5 tests) is explicit rejection of unknown
  versions; a deserialiser that accepts `LocalFire` cannot also accept
  v5 lines, so the version moves to 6.
- **French field names for consistency with v1-v5** (`feu`, `avant`,
  `image`, `apres`) — rejected: milestone M6 made the repository
  English-only; the legacy French keys of v1-v5 stay byte-unchanged
  for the old traces, and new-in-v6 fields follow the charter.

## Decision

1. **One `Fire` span and one `LocalFire` event per local fire, in
   firing order (D1-1).** The in-place kernel opens a `Fire` span
   (`SpanKind::Fire`) before the level runner runs and closes it on
   every exit path — including the two `Err` paths of the fire —
   then emits `TraceEvent::LocalFire { number, level, before, image,
   merged, span }` contextualised into that span, before the union of
   the children is re-saturated. The `Fire` span is closed **before**
   the children re-saturate (they open their own fires), so every
   `Fire` span is a root of the span tree, each holding exactly one
   `Op` span (the runner). Consequences: the spans stay properly
   nested (D1-1); the runner's ops attach to the fire that caused
   them; and `ResultSaturation.iterations` equals both the highest
   fire number and the count of `LocalFire` events — a stated
   invariant, tested.

2. **Sterile-fire marker instead of a stored state.** A fire with
   `merged == before` is the sterile fire that closes the local
   fixpoint of the visited node. The kernel does not store a
   `productive` flag: the trace stores the payload, the views derive
   the marker (`sterile: before == merged` in `FireLine`). One
   source of truth, no drift between kernel and views.

3. **Trace version 6.** `TRACE_VERSION` moves from 5 to 6; v6 adds the
   `fire` span kind (`"fire"`) and the `LocalFire` event with English
   fields `fire`/`level`/`before`/`image`/`merged`. The v1-v5 French
   field names stay byte-unchanged (M6: the charter binds new code,
   not historical bytes). Versions 0-2-3-4-5 are rejected explicitly,
   as before.

4. **Liseuse surface mirrors the rounds surface.** The firing-order
   view is the exact structural twin of the rounds view, one line per
   fire instead of one per round: `fire_intervals`/`fire_lines`/
   `render_fire_lines` in `spans.rs` (balance lines with the
   `\ fixpoint` marker on the sterile fire, folded from the fire's
   `Fire`-span interval), the `fires` command and `until fire` in the
   debugger (same idiom as `rounds` and `until round`), and the
   offline page gains a `fires: N` counter and a `fires` view in the
   embedded JS. A trace without `LocalFire` events (any v5 trace, any
   reference-kernel trace) renders the empty view gracefully — the
   views are additive, nothing round-based changes.

## Deferred tracks (documented, not decided here)

- **Event vs level granularity for the reference kernel.** The
  reference kernel `saturate` keeps its `Round` events as they are.
  Whether its trace should also carry a finer unit is an open
  question; it is not needed to observe the in-place kernel (track 3,
  settled above), and changing the reference kernel's trace is a
  separate decision with its own compatibility analysis.
- **Memories `M≠()` and channel S.** Making the transduction-level
  empty-memory (`M = ≠()`) and the implication channel S observable in
  the trace is a real observability gap of the current trace, but it
  touches the transduction runner (not the saturation kernels) and
  needs a designed event vocabulary of its own. Deferred to a future
  PR; this ADR keeps the two tracks on the record so they are not
  silently dropped.

## Consequences

- The trace format carries a new event and a new span kind; the
  version gate (explicit rejection, no silent upgrade) makes the
  change safe for old readers and loud for stale ones.
- Every match on `TraceEvent` and `SpanKind` had to be completed
  (core observers, Liseuse views, benches); the exhaustive matches
  made the change mechanical.
- The committed offline page artifact
  (`crates/lisage-liseuse/offline/philosophers-n4.html`) is regenerated
  by the byte-identity discipline (`test_d6_1_committed_page_artifact`):
  the philosophers trace contains no fires, so its fire count is 0 —
  the counter and the fires button are present, the view is empty.
- The tracing adds per-fire observation work to the hot path of the
  in-place kernel. With no observer attached the added cost is one
  span-id fetch and two branch checks per fire; with an observer it is
  the emitted events themselves (the `Arbitrary` observers of the
  benches measure the same share discipline as the rounds of the
  reference kernel).

## Open questions (for the human owner)

1. Is per-fire granularity the right unit for a future reference-kernel
   trace as well, or should the two kernels keep different vocabulary?
2. When the M≠() / channel S track is designed, should it version the
   trace again (v7) or ride the next scheduled format change?
