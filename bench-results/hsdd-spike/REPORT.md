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

- No transduction of events across the module boundary (#387): the spike
  reads and counts, it does not fire transitions inside the module.
- No recursive saturation / nested modules (#388): one tower level only.
- No external witness (#391): compared against lisage's own enumeration
  oracle and the pinned golden series, not against an external DD library.
- No timing publication: this is a correctness + storage-shape spike, not
  a PERF-deposit measurement (no machine fingerprint).

Reproduce: `cargo test -p lisage-bench --test hsdd`.
