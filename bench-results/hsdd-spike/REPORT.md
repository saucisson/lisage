# HSDD spike — Hierarchical set-valued instantiation above the flat pool

Spike (issue #382, milestone HSDD), no core change. Question: can lisage's
existing flat pool, controllers and M4-2 relative-form machinery carry the
HSDD model of Couvreur & Thierry-Mieg — one module stored once, instantiated
by a set-valued arc — and reproduce the exact |R_n| semantics with O(1)
storage in the number of instantiated positions?

## Method

New module `crates/lisage-bench/src/hsdd.rs` + test file
`crates/lisage-bench/tests/hsdd.rs`, installed as a bench-language assembly
(the M5-3 / `sdd.rs` pattern, C6 discipline: zero modification to
`lisage-core`).

- **Module**: the 5 legal local states of the philosopher automaton
  `(pc, left)`, built once per pool at `LEVELPC`/`LEVELLEFT`. It compiles to
  4 live nodes (true terminal, 2 `left` nodes, 1 `pc` node): the local
  incoherent pairs are pruned by construction.
- **Ring**: a single node at a new pool-opaque level `LEVEL_INSTANCE` (2),
  carrying one set-valued arc — key = n-bit mask of the instantiated
  positions (`KeyAlgebra` `TowerKeys`, `Key = u64`, max 63 positions),
  target = the shared module root. This is the HSDD-4 in-arc as a bitmask.
- **Controller** `TowerController`: `collapse_active` and `default_active`
  both false. This is the load-bearing finding: with the default
  controller, the 1-arc instance node is collapsed on insert and the
  hierarchy edge (the instantiation record) is destroyed. Disabling
  reductions per controller is an existing `Controller` mechanism.
- **Semantics** (`read_ring`): structural fold that unwinds the module
  once per set position through the same `InputArc` relative form (M4-2
  reading: n positions, one module handle, no new node), reuses the
  existing adjacent-cell coupling matrix, then computes |R_n| by u128
  matrix exponentiation.
- **Oracles**: brute-force enumeration of legal markings
  (`words_reference`), and the pinned golden series 20 / 56 / 152 / 416 /
  1136 for n = 3..7. The flat-pool prototype from the milestone analysis
  is rebuilt in-test and revalidates its pinned live counts
  (206 / 646 / 1850 / 5198 / 14362) as the contrast baseline.

## Results

| n | golden \|R_n\| | tower fold | enumeration | tower live nodes | flat live nodes |
|---|---|---|---|---|---|
| 3 | 20 | 20 | 20 | 4 + 1 ring | 206 |
| 4 | 56 | 56 | 56 | 4 + 1 ring | 646 |
| 5 | 152 | 152 | 152 | 4 + 1 ring | 1850 |
| 6 | 416 | 416 | 416 | 4 + 1 ring | 5198 |
| 7 | 1136 | 1136 | 1136 | 4 + 1 ring | 14362 |

Tower storage is constant in n: the module (4 nodes) plus exactly one ring
node per n, versus the flat pool's exponential blow-up. Canon: hash-consing
hit verified by rebuilding the module a second time (same `NodeId`, same
cross-pool raw forms); instance levels are pool-opaque (the `Arena` never
inspects `LEVEL_INSTANCE`).

Verdict: the HSDD gains hold on the regular ring at the storage and
semantics level — O(1) module sharing, one edge per instantiation set,
exact cardinality by structural fold, all inside the existing pool API.

## Honest scope (what this spike does NOT show)

- Transduction of events across the module boundary (#387): the
  original spike read and counted only; the stacked brick (section
  "Transductions across the boundary" below) fires the events and
  folds their images.
- Recursive saturation / nested modules (#388): covered by the stacked
  brick (section "Recursive saturation through the ring" below).
- No external witness (#391): compared against lisage's own enumeration
  oracle and the pinned golden series, not against an external DD library.
- No timing publication: this is a correctness + storage-shape spike, not
  a PERF-deposit measurement (no machine fingerprint).

Reproduce: `cargo test -p lisage-bench --test hsdd`.

## Transductions across the boundary (HSDD-5, issue #387)

Stacked brick on this spike (follow-up pull request, branch
`feat/hsdd-transductions`): module-local events fired through the tower
as **data on the arc** - one event node per (event, position) at a new
pool-opaque `LEVEL_EVENT`, hash-consed, chains stack above the tower they
transform; the ring and the module are never duplicated. Three events of
the philosopher: `hungry` (think -> hungry, no guard), `eat` (hungry ->
eat, the guard crosses the fragment boundary: no eater on either side),
`think` (eat -> think).

Semantics by structural fold over the pc words (the `left` fields are
derived pairwise, so a firing that takes a neighbour fork updates it for
free): every event is a partial injective map on the pc of one position,
and each level of the chain must remain a legal ring word (no two
adjacent eaters) - which is also what carries the `eat` guards across
the boundary: firing beside an eater would build an illegal level word.
The circular trace of heterogeneous pair matrices counts the images in
O(n |pcs|^3); the product is never expanded. Fold == enumeration oracle,
asserted for n = 3..7, every position, the three events and chains.

Method note (falsifiability): a first draft folded the cell domain and
was falsified by the oracle (0 images for `eat` where the oracle measured
4 on n = 3): firing `eat` also changes the neighbour's `left` field, a
derived value the cell-domain levels did not propagate. The final fold
works on the pc domain and matches the oracle everywhere.

Measured (fold == oracle, events at position 1 unless noted; chains
innermost first):

| n | R_n | hungry@1 | eat@1 | think@1 | hungry@1;eat@1 | eat@1;think@1 | hungry@0;eat@2 |
|---|---|---|---|---|---|---|---|
| 3 | 20 | 8 | 4 | 4 | 4 | 4 | 2 |
| 4 | 56 | 22 | 12 | 12 | 12 | 12 | 4 |
| 5 | 152 | 60 | 32 | 32 | 32 | 32 | 12 |
| 6 | 416 | 164 | 88 | 88 | 88 | 88 | 32 |
| 7 | 1136 | 448 | 240 | 240 | 240 | 240 | 88 |

Storage: one node per (event, position) transduction, a hit on
retransduction; the ring and the module keep their node ids (measured
in-test). The images are counted by the fold, not stored as new towers -
a stored image hierarchy (closed under the guards, reusable) is the
subject of the #386 design.

Reproduce: `cargo test -p lisage-bench --test hsdd_transduction`.

## Recursive saturation through the ring (HSDD-6, issue #388)

Stacked brick on the transductions (follow-up pull request, branch
`feat/hsdd-saturation`): a **firing set** - one bit per module-local
event - as data on the arc of one new pool-opaque `LEVEL_SATURATION`
node, hash-consed, target the tower it saturates (the ring, or another
saturation node: recursive saturation, chains stack one node per
level). One node per (set, target); the ring and the module are never
duplicated, whatever n.

Semantics by structural fold, to the fixpoint: the per-position pair
matrices of the coupling are seeded at the **initial state** (every
philosopher thinks) and grown **in place** - a sweep fires every event
of the set at every position, position i after position i - 1, reading
its fresh pairs, so information travels the whole ring in one sweep (the
in-place firing of DDD saturation). The `eat` guards stay in the pair
legality: the illegal (eat, eat) pair is never written, on both sides of
the fired cell. The closed count is the circular trace of the product;
the ring is never expanded. Fold == BFS oracle, asserted for n = 3..=7
and all seven non-empty firing sets.

Measured (fold == BFS oracle everywhere; rounds = sweeps through the
ring, confirming sweep included), pins on ring 3:

| firing set (n = 3) | closure | rounds |
|---|---|---|
| hungry | 8 | 2 |
| eat | 1 | 0 |
| think | 1 | 0 |
| hungry + think | 8 | 2 |
| eat + think | 1 | 0 |
| hungry + eat | 20 | 2 |
| all three | 20 | 2 |

| n | golden \|R_n\| | saturated closure (full set) | rounds |
|---|---|---|---|
| 3 | 20 | 20 | 2 |
| 4 | 56 | 56 | 2 |
| 5 | 152 | 152 | 2 |
| 6 | 416 | 416 | 2 |
| 7 | 1136 | 1136 | 2 |

Findings:

- The saturation closure IS the reachability set the spike folded: the
  full set recovers the golden thread exactly, in two sweeps whatever n.
- `hungry` alone closes on the {think, hungry} words (2^3 = 8: every
  position is independently think or hungry, no eater ever appears);
  `eat` or `think` alone never fire from the initial state (closure 1,
  zero sweeps).
- `hungry` + `eat` already saturates to the golden thread: `think` is
  enabled on the eaters of the closure, but every image it produces is
  already in it - the closure, not the run length, is what the count
  measures.
- Recursive saturation is idempotent: nesting the full set over its own
  closure adds no sweep (n = 6: 416, same rounds) - and the read chain
  (`read_saturation`) exposes the nested sets, innermost first, without
  duplicating the ring or the module.
- Storage: one node per (firing set, target), measured in-test; the
  sweeps never intern anything, the closure lives in the fold.

Reproduce: `cargo test -p lisage-bench --test hsdd_saturation`.

## Observed, self-detected and per-fragment saturation (HSDD-6 remainder, issue #388)

The three items the saturation brick left open, now measured - still with
zero `lisage-core` change. The load-bearing finding: the core M1-0 observer
port already exposes what the #388 remainder asked for (`Observer`,
`TraceEvent::Round` with `active_levels`/`modified_levels`, the
`SpanKind::{Op,Round}` span helpers), so the schedulers plug into it from
`lisage-bench`: a `TraceCollector` observes the pool storage events
(`NodeCreated`, `NodeShared`) and the fold emits its own `Round` events
with the structural fingerprint of the chain in `OpStart`.

Three schedulers share the #395 fold (same closure, same rounds
accounting, fold == BFS oracle everywhere):

- **Observed** (`saturate_observed`): the blind ring sweep of #395, one
  core `Round` event per performed sweep, active and modified fragments
  in the event.
- **Self-detected** (`saturate_detected`): the Hamez lift of the core
  `PolicySaturation::AutoDetectee` - a wave fires only the active set,
  the next active set is the neighbourhood of the changed positions
  (the re-arm rule: a position becomes firable again only through a
  neighbour's change), a quiet wave confirms the fixpoint and ends the
  set; `firings` measures the schedule against the blind sweep.
- **Per-fragment** (`saturate_fragments`): n overlapping windows of f
  positions, each window driven to its own local fixpoint, then one
  lift-off pass over the whole ring; one `Round` event per parent pass;
  `local_fixpoints` counts the per-window fixpoints; f == 0 and f > n
  are rejected.

Measured on the full firing set (fold == BFS oracle everywhere):

| n | golden \|R_n\| | all policies | rounds | detected firings |
|---|---|---|---|---|
| 3 | 20 | 20 | 2 | 6 |
| 4 | 56 | 56 | 2 | 8 |
| 5 | 152 | 152 | 2 | 10 |
| 6 | 416 | 416 | 2 | 12 |
| 7 | 1136 | 1136 | 2 | 14 |

Per-fragment sub-fixpoints (full set, 2 parent passes whatever n):

| f | local fixpoints (n = 3..7) | firings |
|---|---|---|
| 1 | 9 / 12 / 15 / 18 / 21 (= 3n) | lf |
| 2 | 8 / 11 / 14 / 17 / 20 (= 3n - 1) | 2 * lf |

Two-set chain [hungry; eat] (nested saturation of #395): the golden
thread is recovered under every policy in 4 rounds (2 per set);
detected firings == 4n; f = 2 local fixpoints 16 / 22 / 28 / 34 / 40.

Findings:

- The honest degeneration: on the regular ring the self-detected
  schedule degenerates to the blind sweep - `firings == n * max(rounds, 1)`
  for every n and every firing set. The detection machinery is in place
  and the trace proves the schedule, but the uniform topology gives it
  nothing to skip.
- The observed trace (n = 5) carries the whole story without growing the
  structure: 6 `NodeCreated` (4 module + ring + saturation), 1
  `NodeShared` on the re-saturation hit, `Round` events [(1, all
  positions, all positions), (2, all positions, empty)] - the
  confirming sweep is visible as the round that modifies nothing -
  balanced `SpanOpen`/`SpanClose`, `OpStart`/`OpEnd` around the
  saturation.
- Per-fragment fixpoints converge to the same closure with the same 2
  parent passes as the blind sweep: the window-local fixpoints (3n for
  f = 1, 3n - 1 for f = 2) do not inflate the parent schedule, and the
  last pass modifies nothing.
- Zero core change held: the #388 items that genuinely need core work
  are the per-round materialized diagrams (real before/image/after
  handles and growth) and memo/memory versions per (round x fragment).

Reproduce: `cargo test -p lisage-bench --test hsdd_policy`.



## Per-round diagrams and per-fragment memory versions (HSDD-6 core, issue #388)

First core change of the stack: the two items #388 lists as core. The
memo key of the runner now carries the versions of the fragments the
visit DECLARES to read (`Transduction::read_fragments`, default: the
visited level only), `MemoryLevel` bumps one version per written
fragment on top of the global one, and `saturate` keeps ONE runner for
the whole fixpoint - each round swaps only the per-round
`RestrictedToLevels` wrapper (whose fingerprint now folds the active
levels) through `set_transduction`. On top, the bench grows
`saturate_diagrammed`: the coupling state materialized as a chain of
sheet nodes (one sheet per position at level 5 + i, the fragment of
the position), saturated through the core saturation itself, the
closure counted by reading the final diagram back.

Measured on the full firing set (fold == BFS oracle everywhere, the
closure read FROM the final diagram):

| n | golden \|R_n\| | diagrammed count | rounds | round-1 growth |
|---|---|---|---|---|
| 3 | 20 | 20 | 2 | 3 |
| 4 | 56 | 56 | 2 | 4 |
| 5 | 152 | 152 | 2 | 5 |
| 6 | 416 | 416 | 2 | 6 |
| 7 | 1136 | 1136 | 2 | 7 |

Storage and memo pins of the materialized saturation (full set, per n):

| n | sheets interned | shares | confirming shares | misses | hits |
|---|---|---|---|---|---|
| 3 | 6 | 8 | 3 | 7 | 22 |
| 4 | 8 | 10 | 4 | 9 | 29 |
| 5 | 10 | 12 | 5 | 11 | 36 |
| 6 | 12 | 14 | 6 | 13 | 43 |
| 7 | 14 | 16 | 7 | 15 | 50 |

Findings:

- The `Round` events now carry the real materialized handles: the
  continuity `after == next before` holds through the whole trace, the
  confirming round reproduces the chain on pure R-share (growth 0,
  nothing modified), and every handle reads back as the coupling chain
  it claims to be - the seed sheets are the think-think singletons,
  the final sheets the 8 legal pairs without (eat, eat).
- The memo survives the rounds, measured: on the uniform ring the seed
  round misses every visit (2n + 1 misses) and ALL 7n + 1 hits answer
  in the confirming round - a per-round runner answers none of them.
  The folded active-level fingerprint is the safety of the reused
  entries under the per-round active-set tracking.
- The rounds of the diagrammed engine are simultaneous-firing rounds:
  2 for every set carrying the hungry bit, 1 confirming-only for the
  sets that cannot leave the all-think seed, 2 per set for the chain
  [hungry; eat] (the golden thread in 4 rounds, the fixpoint state of
  each set handed over as the first before of the next). The closure
  equals the sweeps of the previous bricks everywhere; the round
  counts are NOT those of the in-place sweeps.
- Hash-consing makes the whole diagram one big pool share: re-running
  the engine interns NOTHING (identical handles, same rounds).
- Downstream: the committed offline trace page of lisage-liseuse is
  regenerated (the trace changed with the persistent runner: 196
  events instead of 202, same 4 rounds, more memo hits).

Reproduce: `cargo test -p lisage-core --test memory --test saturate`
and `cargo test -p lisage-bench --test hsdd_core_rounds`.
