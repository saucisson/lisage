# Campaign 20261121-campaign-4 (ALIGN-7f, issue #301)
Machine: M1

```json
{"cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "date_utc": "2026-11-21T01:04:43+00:00", "hardware_hash": "febd14ed", "kernel": "6.18.58.1-microsoft-standard-WSL2", "logical_cores": 4, "machine_id": "M1", "page_size": 4096, "physical_cores": 4, "ram_total_kib": 3979828, "sources": {"cpu_model": "/proc/cpuinfo: model name", "kernel": "uname -r (platform.release())", "logical_cores": "/proc/cpuinfo: count of 'processor' entries", "machine_id": "bench-results/machines.md: hardware hash match", "page_size": "os.sysconf(SC_PAGE_SIZE)", "physical_cores": "/proc/cpuinfo: unique (physical id, core id) pairs", "ram_total_kib": "/proc/meminfo: MemTotal", "wsl2_config": "no /mnt/c/Users/*/.wslconfig reachable from inside WSL2"}, "wsl2_config": null}
```

Full confrontation campaign: same charges, same pinned witnesses, same
protocol as `20261119-campaign-3` (BENCH-1/PERF-4, issue #286), now with
the **Sylvan** witness dockerized (issue #308, closes SP-2 #154) as the
4th external tool.

**Machine note**: issue #301 was written as "campaign-4 on M2", but the
machine register was corrected on the evening of 2026-11-20 (switch back
to M1, `bench-results/machines.md`): this campaign ran on **M1**
(hardware hash `febd14ed`), the same machine as campaign-3. The
comparison to campaign-3 is therefore presented **ratio-only** as the
issue requires (and as ALIGN-7c mandates across machines); since both
campaigns are in fact on M1, absolute times are additionally comparable
and are reported as stable (see REPORT.md).

## Exact reproduction command

```bash
cargo build --release -p lisage-bench --bin confrontation
docker build -t lisage-oxidd:latest docker/oxidd
docker build -t lisage-cudd:latest docker/cudd
docker build -t lisage-libddd:latest docker/libddd
docker build -t lisage-sylvan:latest docker/sylvan
uv run python -m scripts bench-confrontation --crossover \
    --tools lisage,oxidd,cudd,libddd,sylvan --repetitions 5 \
    --run-id 20261121-campaign-4
```

(machine: M1, 2026-11-21; one fresh process per measurement for lisage,
one fresh container per measurement for the external witnesses; OxiDD
pinned at commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`, CUDD at tag
`cudd-3.0.0`, libDDD at `b775aed3e8405a8906b10f937f6dbcc4a4b8831c`, Sylvan
at tag `v1.12.8` = `fe8f9463a5f64ea8542534d82dd74a6766de127b`. For
`alea-lcg-24` the OxiDD witness runs with
`OXIDD_INNER_CAPACITY=OXIDD_APPLY_CACHE_CAPACITY=2^23`, as in campaign-2/3.)

## Contents

- `<charge>-<outil>.jsonl`: one JSON line per repetition (raw), 5 charges
  x 5 tools.
- `aggregats.json`: median/min/max of `temps_ms` and `rss_max_ko` per
  charge x tool, plus the cross-check `noeuds`/`verifie` and the cross-tool
  `divergences` list (**empty**: the campaign is valid across all five
  tools, Sylvan included).
- `REPORT.md`: the comparative report — ratio-only comparison to
  campaign-3 and the explicit analysis of the n=24 residue.
