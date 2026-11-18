# Results report — campaign 20261117-campaign-1 (M7-6, issue #116)

Comparative measurements lisage vs OxiDD on the M7-4 real charges.
Protocol: `docs/bench-confrontation-witnesses-and-measures.md`. Every
number below comes from the deposited files of this directory
(`aggregats.json`, `<charge>-<outil>.jsonl`); nothing is unsourced.

- Machine: local developer machine (WSL2), 2026-11-17.
- Repetitions: 5 per charge x tool, one fresh process per measurement
  (fresh Docker container for OxiDD).
- Reproduction: see `README.md` in this directory.
- OxiDD pinned at commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`.
- CUDD/Sylvan: not measured in this campaign (the pre-existing CUDD
  `DagSize` terminal-counting divergence is documented in WORKLOG; out
  of scope here).

## Cross-check (validity of the campaign)

`noeuds` (BDD nodes, terminals included) and `verifie` (satisfying
assignments) are identical across tools for every charge — the
orchestrator's divergence list is empty (`aggregats.json`):

| Charge | noeuds | verifie |
|---|---|---|
| cnf-uf20-01 | 51 | 8 |
| pnml-swimmingpool-1 | 1859 | 1408 |
| pnml-erk-000001 | 9721 | 8672 |

## Time (ms, median [min, max])

| Charge | lisage | OxiDD | ratio lisage/OxiDD |
|---|---|---|---|
| cnf-uf20-01 | 355.4 [352.2, 361.6] | 88.7 [86.4, 90.9] | 4.0x |
| pnml-swimmingpool-1 | 85.6 [85.3, 87.2] | 24.5 [23.6, 26.0] | 3.5x |
| pnml-erk-000001 | 1348.6 [1346.6, 1361.8] | 353.9 [345.2, 370.4] | 3.8x |

## Peak RSS (kio, median [min, max])

| Charge | lisage | OxiDD | ratio OxiDD/lisage |
|---|---|---|---|
| cnf-uf20-01 | 5388 [5388, 5392] | 52692 [52464, 52804] | 9.8x |
| pnml-swimmingpool-1 | 4204 [4204, 4208] | 32060 [31888, 32192] | 7.6x |
| pnml-erk-000001 | 19768 [19724, 19768] | 135720 [135516, 135800] | 6.9x |

## Reading

- **Memory: lisage wins everywhere** (7-10x less peak RSS). Hypothesis:
  the OxiDD witness pre-allocates its unique table and apply cache at
  2^20 entries each (witness sizing constant, `INNER_NODE_CAPACITY` /
  `APPLY_CACHE_CAPACITY` in `docker/oxidd/oxidd_count/src/main.rs`),
  which sets a high fixed baseline (~24-135 Mo observed); lisage's arena
  grows with the actual diagram (51-9 721 nodes). The gap narrows as the
  charge grows (9.8x → 6.9x), consistent with a fixed-cost vs
  proportional-cost difference.
- **Time: OxiDD wins everywhere** (3.5-4.0x faster). Hypothesis: OxiDD
  is a mature BDD engine with an apply cache and tuned node
  representation; lisage builds by bottom-up Shannon expansion over the
  materialized 2^n truth table in a fresh arena, and the table
  materialization itself (2^n booleans) is part of the measured block on
  both sides.
- **Limits of the measurement**: single machine, 5 repetitions, the
  measured block includes reading + materializing the 2^n-entry truth
  table (which dominates for the heavy ERK charge: 2^22 entries), the
  OxiDD witness sizing is a constant not tuned per charge, and only two
  tools are compared (CUDD/Sylvan not measured, see above). The
  cross-check values (node counts) are exact and identical, so the
  comparison is about time/memory, not about correctness of the diagrams.
