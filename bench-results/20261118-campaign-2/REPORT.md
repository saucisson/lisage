# Results report — campaign 20261118-campaign-2 (BENCH-1, issue #277)
Machine: M1


Comparative measurements lisage vs OxiDD vs CUDD vs libDDD (LIP6) on the
BENCH-1 crossover charges. Protocol:
`docs/bench-confrontation-witnesses-and-measures.md`. Every number below
comes from the deposited files of this directory (`aggregats.json`,
`<charge>-<outil>.jsonl`); nothing is unsourced.

- Machine: local developer machine, 2026-11-18.
- Repetitions: 5 per charge x tool, one fresh process per measurement
  (fresh Docker container for the external witnesses).
- Reproduction: see `README.md` in this directory.
- Pins: OxiDD `c7a458df8b747a3ee44d0e9e61059a1621865fcd`, CUDD
  `cudd-3.0.0` (`fc30c18f8db2e7d03d13241038b61eb0056a734e`), libDDD
  `b775aed3e8405a8906b10f937f6dbcc4a4b8831c`. `alea-lcg-24` runs the OxiDD
  witness with 2^23 unique-table/apply-cache capacities (1 071 698 nodes
  exceed the default 2^20); the other charges use the default image.

## Cross-check (validity of the campaign)

`noeuds` and `verifie` are identical across all four tools for every
charge — the orchestrator's divergence list is empty (`aggregats.json`):

| Charge | noeuds | verifie |
|---|---|---|
| alea-lcg-20 | 106 760 | 523 228 |
| alea-lcg-22 | 325 824 | 2 096 246 |
| alea-lcg-24 | 1 071 698 | 8 386 976 |
| cnf-uf20-01 | 51 | 8 |
| pnml-swimmingpool-1 | 1 859 | 1 408 |

## Time (ms, median [min, max])

| Charge | lisage | OxiDD | CUDD | libDDD | lisage/OxiDD |
|---|---|---|---|---|---|
| alea-lcg-20 | 533.9 [524.8, 537.4] | 122.3 [121.4, 123.2] | 911.6 [901.0, 992.3] | 861.4 [853.7, 872.3] | 4.4x |
| alea-lcg-22 | 2 146.2 [2 097.4, 2 153.4] | 496.0 [492.1, 535.7] | 4 552.0 [4 428.8, 4 678.7] | 3 211.1 [3 190.5, 3 264.5] | 4.3x |
| alea-lcg-24 | 8 605.1 [8 476.5, 8 627.9] | 2 069.2 [2 021.9, 2 077.3] | 21 439.8 [21 282.9, 21 481.8] | 13 083.6 [12 969.7, 13 102.1] | 4.2x |
| cnf-uf20-01 | 383.3 [381.5, 394.7] | 86.9 [84.9, 94.4] | 105.9 [105.1, 113.4] | 289.8 [288.3, 290.8] | 4.4x |
| pnml-swimmingpool-1 | 90.4 [90.3, 91.4] | 24.6 [22.9, 26.7] | 30.3 [29.4, 32.5] | 76.7 [75.7, 78.3] | 3.7x |

## Peak RSS (kio, median [min, max])

| Charge | lisage | OxiDD | CUDD | libDDD | OxiDD/lisage |
|---|---|---|---|---|---|
| alea-lcg-20 | 20 524 [20 516, 20 568] | 50 420 [50 364, 50 520] | 96 256 [95 956, 96 328] | 42 896 [42 844, 42 952] | 2.5x |
| alea-lcg-22 | 64 432 [64 428, 64 440] | 133 136 [133 124, 133 264] | 274 928 [274 676, 274 944] | 127 292 [127 276, 127 396] | 2.1x |
| alea-lcg-24 | 234 228 [234 196, 234 312] | 608 404 [608 108, 608 628] | 898 544 [898 108, 898 732] | 462 432 [462 308, 462 488] | 2.6x |
| cnf-uf20-01 | 6 304 [6 284, 6 388] | 50 404 [50 272, 52 376] | 18 804 [18 772, 18 904] | 32 348 [32 308, 32 464] | 8.0x |
| pnml-swimmingpool-1 | 4 312 [4 260, 4 356] | 31 772 [29 516, 31 920] | 16 404 [16 248, 16 476] | 10 788 [10 640, 10 956] | 7.4x |

## Reading

### Time: OxiDD ~4x faster than lisage, a constant factor; CUDD/libDDD slower than lisage on the table charges

- The lisage/OxiDD time ratio is **3.7-4.4x on every charge**, and the
  per-node time on the alea-lcg family is 5.0/6.6/8.0 us/node for lisage
  vs 1.15/1.52/1.93 us/node for OxiDD (n = 20/22/24) — the factor is
  constant in the node count, not an artifact of one charge.
- CUDD and libDDD are **1.7-2.5x slower than lisage** on the alea-lcg
  charges (both parse the 2^n-entry table from text stdin inside the
  measured block, then run a generic apply), but faster than lisage on
  the two small real charges (fixed per-process costs dominate there:
  lisage pays pool/arena setup inside the block). lisage sits between
  OxiDD and the classic engines.

### Why OxiDD is ~4x faster (hypotheses, quantified)

- Both sides materialize the 2^n truth table inside the measured block,
  so the table term cannot explain the gap; OxiDD additionally parses it
  from text stdin (~2 B/entry), which works against OxiDD — the diagram
  construction itself is therefore more than 4x in OxiDD's favor.
- lisage pays, per interned node: hash-consing lookup, observer dispatch,
  `ArcRun` construction, and **9.17 allocations per internement**
  (measured by the MEM-18 counting allocator, frozen CI gate at 10.0,
  issue #134). OxiDD's bottom-up ITE loop is a cache lookup per pair plus
  an inline-arcs node in a pre-sized unique table: no observer, no
  per-node heap allocation, no run wrapper.
- Conclusion: the gap is dominated by per-intern overhead (allocations +
  observer + boxed runs), not by the hash-consing itself. This is the
  target of a future perf milestone (see WORKLOG).

### Memory: lisage wins everywhere on these charges; the crossover is workload-dependent

- lisage has the lowest peak RSS on every charge (2.1-8.0x less than
  OxiDD, 1.9-4.7x less than CUDD, 1.4-2.1x less than libDDD).
- Fitting the alea-lcg family (median RSS): lisage ~ **4.3 MB fixed +
  11.1 B x 2^n + 45.8 B x nodes**; OxiDD (default 2^20 capacities) ~
  **50.4 MB fixed + 19.7 B x 2^n** (its unique table and apply cache are
  pre-allocated, so nodes below 2^20 cost nothing extra).
- **Crossover answer**: on truth-table charges (alea-lcg, cnf, pnml) OxiDD
  *never* passes below lisage — both pay the 2^n table, OxiDD pays more
  per entry (19.7 vs 11.1 B) on top of its ~50 MB fixed baseline, so the
  ratio stays 2.1-2.5x in lisage's favor (2.6x at n = 24 where the OxiDD
  witness needed 2^23 capacities). On diagrams built by operations (no
  2^n table materialized), lisage's ~46 B/node crosses OxiDD's ~50 MB
  fixed baseline at **~1.0M nodes** — past that, OxiDD's pre-allocation
  wins. The 2^20 witness sizing is a constant of the witness, not of the
  library: a production OxiDD manager sized to the diagram would not pay
  the 50 MB baseline on small charges.

### Limits of the measurement

Single machine, 5 repetitions; the measured block includes reading +
materializing the 2^n table (dominant for alea-lcg-24: 2^24 entries);
witness sizing constants (OxiDD capacities, CUDD unique/cache slots) are
not tuned per charge; four tools compared, Sylvan still absent (spike
SP-2, issue #154). Cross-check values are exact and identical across
tools, so the comparison is about time/memory, not diagram correctness.
