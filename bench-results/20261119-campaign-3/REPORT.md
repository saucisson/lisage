# Campaign 20261119-campaign-3 (PERF-4, issue #286)

Post-PERF comparative measurements lisage vs OxiDD vs CUDD vs libDDD (LIP6) on
the BENCH-1 crossover charges, **same charges, same witnesses, same protocol**
as `20261118-campaign-2` (`docs/bench-confrontation-witnesses-and-measures.md`),
after PERF-2 (#282) and PERF-3 (#284) cut the per-intern cost of lisage.

- Pins: OxiDD `c7a458df8b747a3ee44d0e9e61059a1621865fcd`, CUDD `cudd-3.0.0`,
  libDDD `b775aed3e8405a8906b10f937f6dbcc4a4b8831c` (same docker images as
  campaign-2). `alea-lcg-24` runs OxiDD with
  `OXIDD_INNER_CAPACITY=OXIDD_APPLY_CACHE_CAPACITY=2^23`.
- Machine: local developer machine, 2026-11-19; one fresh process per
  measurement for lisage, one fresh container per measurement for the
  external witnesses; 5 repetitions per charge x tool.

## Cross-check (identical across all four tools)

`noeuds`/`verifie` identical for every charge across lisage/OxiDD/CUDD/libDDD
and `divergences` is empty — the campaign is valid (C1 holds after PERF-2/3).

| charge | noeuds | verifie |
|---|---|---|
| alea-lcg-20 | 106 760 | 523 228 |
| alea-lcg-22 | 325 824 | 2 096 246 |
| alea-lcg-24 | 1 071 698 | 8 388 608 |
| cnf-uf20-01 | (see aggregats) | (see aggregats) |
| pnml-swimmingpool-1 | (see aggregats) | (see aggregats) |

## Time: median ms (5 repetitions), campaign-2 -> campaign-3

| charge | lisage c2 | lisage c3 | OxiDD | CUDD | libDDD | lisage/OxiDD c2 | lisage/OxiDD c3 |
|---|---|---|---|---|---|---|---|
| alea-lcg-20 | 533.9 | 169.8 | 122.3 | 938.1 | 885.8 | 4.37x | **1.39x** |
| alea-lcg-22 | 2146.2 | 975.6 | 489.3 | 4612.9 | 3227.8 | 4.39x | **1.99x** |
| alea-lcg-24 | 8605.1 | 4105.6 | 2013.1 | 21486.9 | 13090.2 | 4.27x | **2.04x** |
| cnf-uf20-01 | 383.3 | 117.4 | 85.5 | 105.6 | 290.4 | 4.48x | **1.37x** |
| pnml-swimmingpool-1 | 90.4 | 27.5 | 24.5 | 30.1 | 77.0 | 3.70x | **1.12x** |

## Verdict

The constant ~4x OxiDD time gap of BENCH-1 is closed to **1.1-2.0x** on every
charge (1.39x on the reference alea-lcg-20). The residual factor grows with n
(1.39x at n=20, 2.04x at n=24), consistent with a remaining per-node cost
(lisage ~158 us/knode at n=24 vs OxiDD ~77 us/knode) rather than a per-intern
constant — next PERF increments would target the per-created-node path
(shadow index, slab growth) rather than the intern path itself. CUDD and
libDDD remain slower than lisage on the truth-table charges, as in campaign-2.

## RSS (median KiB)

| charge | lisage c2 | lisage c3 | OxiDD | CUDD | libDDD |
|---|---|---|---|---|---|
| alea-lcg-20 | 20 524 | 20 224 | 52 420 | 96 168 | 42 836 |

lisage memory is unchanged (within noise) by PERF-2/3 — the wins are time and
transient allocations, not live memory; the MEM-18 thresholds are not
degraded.
