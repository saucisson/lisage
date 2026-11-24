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
- No recursive saturation / nested modules (#388): one tower level only.
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
