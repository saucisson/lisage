# Report — 20261121-sylvan-smoke (issue #308)

Machine: M1

```json
{"cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "date_utc": "2026-11-20T23:49:49+00:00", "hardware_hash": "febd14ed", "kernel": "6.18.58.1-microsoft-standard-WSL2", "logical_cores": 4, "machine_id": "M1", "page_size": 4096, "physical_cores": 4, "ram_total_kib": 3979828, "sources": {"cpu_model": "/proc/cpuinfo: model name", "kernel": "uname -r (platform.release())", "logical_cores": "/proc/cpuinfo: count of 'processor' entries", "machine_id": "/home/alban/lisage/bench-results/machines.md: hardware hash match", "page_size": "os.sysconf(SC_PAGE_SIZE)", "physical_cores": "/proc/cpuinfo: unique (physical id, core id) pairs", "ram_total_kib": "/proc/meminfo: MemTotal", "wsl2_config": "no /mnt/c/Users/*/.wslconfig reachable from inside WSL2"}, "wsl2_config": null}
```

## Goal

Validate the dockerized Sylvan witness (SP-2, issue #154) inside the
confrontation orchestrator: same stdin protocol, same JSONL line, and the
cross-check `noeuds`/`verifie` identical across all five tools.

## Result

Charge `cnf-uf20-01` (20 variables), 3 repetitions per tool, medians:

| Tool | temps_ms (median) | rss_max_ko (median) | noeuds | verifie |
|---|---|---|---|---|
| lisage | 135.2 | 6 332 | 51 | 8 |
| oxidd | 93.7 | 52 512 | 51 | 8 |
| cudd | 107.1 | 18 836 | 51 | 8 |
| libddd | 295.6 | 32 280 | 51 | 8 |
| sylvan | 137.5 | 18 780 | 51 | 8 |

**Cross-check: no divergence** (`aggregats.json` → `divergences: []`).
Sylvan's node count uses the lisage convention (distinct (node,
complement-bit) pairs reachable from the root, terminals included) and
matches the other four tools exactly on this charge.

## Notes

- Sylvan is a parallel package (Lace workers on all 4 cores of M1); its
  wall-clock includes that parallelism. On this small charge it lands in
  the same range as CUDD.
- Absolute times are M1-only and not comparable with the M2 campaigns
  (ALIGN-7c); ratios within this run are valid (all witnesses share the
  host).
- This is a smoke run, not a campaign: campaign-4 on M2 stays deferred
  (issue #301, ALIGN-7f).
