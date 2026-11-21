# Campaign 20261121-campaign-4 (ALIGN-7f, issue #301)
Machine: M1

```json
{"cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "date_utc": "2026-11-21T01:04:43+00:00", "hardware_hash": "febd14ed", "kernel": "6.18.58.1-microsoft-standard-WSL2", "logical_cores": 4, "machine_id": "M1", "page_size": 4096, "physical_cores": 4, "ram_total_kib": 3979828, "sources": {"cpu_model": "/proc/cpuinfo: model name", "kernel": "uname -r (platform.release())", "logical_cores": "/proc/cpuinfo: count of 'processor' entries", "machine_id": "bench-results/machines.md: hardware hash match", "page_size": "os.sysconf(SC_PAGE_SIZE)", "physical_cores": "/proc/cpuinfo: unique (physical id, core id) pairs", "ram_total_kib": "/proc/meminfo: MemTotal", "wsl2_config": "no /mnt/c/Users/*/.wslconfig reachable from inside WSL2"}, "wsl2_config": null}
```

Full confrontation lisage vs OxiDD vs CUDD vs libDDD vs **Sylvan** (new 4th
witness, issue #308) on the BENCH-1 crossover charges — **same charges,
same pinned witnesses, same protocol** as `20261119-campaign-3`
(`docs/bench-confrontation-witnesses-and-measures.md`).

- Pins: OxiDD `c7a458df8b747a3ee44d0e9e61059a1621865fcd`, CUDD `cudd-3.0.0`,
  libDDD `b775aed3e8405a8906b10f937f6dbcc4a4b8831c`, Sylvan `v1.12.8` =
  `fe8f9463a5f64ea8542534d82dd74a6766de127b` (same docker images as
  campaign-3 plus the new `docker/sylvan`). `alea-lcg-24` runs OxiDD with
  `OXIDD_INNER_CAPACITY=OXIDD_APPLY_CACHE_CAPACITY=2^23`.
- Machine: **M1** (hardware hash `febd14ed`), 2026-11-21. Issue #301 was
  titled "campaign-4 on M2", but the register was corrected on
  2026-11-20 evening (switch back to M1): this campaign ran on M1, the
  same machine as campaign-3. Per the issue (and ALIGN-7c), the
  comparison below is **ratio-only**; absolute times are additionally
  comparable here and are noted as stable.
- One fresh process per measurement for lisage, one fresh container per
  measurement for the external witnesses; 5 repetitions per charge x tool.

## Cross-check (identical across all five tools)

`noeuds`/`verifie` identical for every charge across
lisage/OxiDD/CUDD/libDDD/Sylvan and `divergences` is empty — the campaign
is valid, Sylvan included (C1 holds).

| charge | noeuds | verifie |
|---|---|---|
| alea-lcg-20 | 106 760 | 523 228 |
| alea-lcg-22 | 325 824 | 2 096 246 |
| alea-lcg-24 | 1 071 698 | 8 386 976 |
| cnf-uf20-01 | 51 | 8 |
| pnml-swimmingpool-1 | 1 859 | 1 408 |

## Time: median ms (5 repetitions), ratio-only vs campaign-3

| charge | lisage c4 | OxiDD | CUDD | libDDD | Sylvan | lisage/OxiDD c4 | lisage/OxiDD c3 |
|---|---|---|---|---|---|---|---|
| alea-lcg-20 | 172.4 | 126.1 | 973.4 | 896.5 | 336.3 | **1.37x** | 1.39x |
| alea-lcg-22 | 1 013.1 | 492.5 | 4 792.1 | 3 358.8 | 1 355.7 | **2.06x** | 1.99x |
| alea-lcg-24 | 4 218.3 | 2 016.5 | 22 769.7 | 13 555.8 | 5 740.4 | **2.09x** | 2.04x |
| cnf-uf20-01 | 129.9 | 87.4 | 108.9 | 296.7 | 136.1 | **1.49x** | 1.37x |
| pnml-swimmingpool-1 | 25.4 | 24.5 | 30.3 | 77.8 | 49.1 | **1.04x** | 1.12x |

Absolute times, same machine M1 (campaign-3 -> campaign-4, lisage):
169.8 -> 172.4, 975.6 -> 1 013.1, 4 105.6 -> 4 218.3, 117.4 -> 129.9,
27.5 -> 25.4 ms — all within run-to-run noise (no lisage code changed
between the two campaigns; only the Sylvan witness was added).

## n=24 residue analysis (explicit, per issue #301)

- **Did it move?** No: the lisage/OxiDD ratio at n=24 is **2.09x** in
  campaign-4 vs **2.04x** in campaign-3 — unchanged within noise.
- **What share is the unique table?** Per-node cost at n=24: lisage
  4 218.3 ms / 1 071 698 nodes = **3.94 us/node**, OxiDD 2 016.5 ms /
  1 071 698 = **1.88 us/node**; the excess is 2.06 us/node, i.e. ~52% of
  lisage's per-node cost at n=24. At n=20 the same computation gives
  1.61 vs 1.18 us/node (excess 0.43 us/node, ~27%). Both tools' per-node
  cost grows with the table size (cache-miss-bound unique table: 1.07M
  nodes at n=24, load factor 12.8% of the 2^23 OxiDD slots, vs 107k nodes
  at n=20), but lisage's grows faster (1.61 -> 3.94 us/node, 2.45x) than
  OxiDD's (1.18 -> 1.88, 1.59x), which is why the ratio widens from 1.37x
  to 2.09x. The residue is therefore dominated by **unique-table
  behavior** (content hash lookup + node allocation), not by a fixed
  per-intern constant — consistent with the campaign-3 verdict; next
  PERF increments should target the per-created-node path (shadow index,
  slab growth), not the intern path.
- **Sylvan position**: 2.67x OxiDD at n=20, 2.85x at n=24 — between OxiDD
  and CUDD (7.7x OxiDD at n=20), well ahead of CUDD/libDDD on the
  truth-table charges; its cross-check values match exactly.

## RSS (median KiB)

| charge | lisage | OxiDD | CUDD | libDDD | Sylvan |
|---|---|---|---|---|---|
| alea-lcg-20 | 18 300 | 52 444 | 96 268 | 42 848 | 31 536 |
| alea-lcg-22 | 57 256 | 135 432 | 274 840 | 127 252 | 92 528 |
| alea-lcg-24 | 221 272 | 610 596 | 898 600 | 462 368 | 362 816 |
| cnf-uf20-01 | 6 408 | 52 632 | 18 860 | 32 280 | 18 644 |
| pnml-swimmingpool-1 | 4 184 | 31 788 | 16 320 | 10 752 | 13 516 |

lisage keeps the lowest memory footprint on every charge (2.8x less than
OxiDD at n=24); the MEM-18 thresholds are not degraded.

## Verdict

Campaign-4 confirms campaign-3 on the same machine: the lisage/OxiDD gap
stays at **1.0-2.1x** on every charge (1.37x on the reference
alea-lcg-20), and the n=24 residue is stable at ~2.1x, unique-table
dominated. The new Sylvan witness validates (identical `noeuds`/`verifie`)
and slots between OxiDD and CUDD on time, with mid-range memory.
