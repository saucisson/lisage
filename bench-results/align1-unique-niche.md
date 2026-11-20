# ALIGN-1 — unique-niche alignment measurement (issue ALIGN-1)

Machine: M2

```json
{"machine_id": "M2", "hardware_hash": "febd14ed", "cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "physical_cores": 4, "logical_cores": 4, "ram_total_kib": 3979828, "kernel": "6.18.58.1-microsoft-standard-WSL2", "wsl2_config": null, "page_size": 4096}
```

Same charge (alea-lcg n=20, bottom-up Shannon expansion, 2^20 - 1 intern
calls), same runner (`profile_intern`), same deterministic allocation
counters as the PERF series.

| line | temps build (ms) | allocs build | allocs/internement | machine |
|---|---|---|---|---|
| PERF-5 (issue #288) | 132.1 | 1 048 802 | 1.000 | M1 |
| ALIGN-1 "before" | 69.8 | 1 048 802 | 1.000 | M2 |

## Verdict (ALIGN-7e)

The 132.1 ms → 69.8 ms gap on the identical charge, runner and allocation
counters is attributed to the **M1 → M2 machine change** of the evening of
2026-11-19 (see `bench-results/machines.md`), not to any code change: the
allocation counters are byte-identical (1 048 802 allocations, 1.000
alloc/intern), so the work performed is the same and only the hardware
clock differs.

**Reformulated nextest note**: the earlier note "nextest ~10x" remains
true in itself for test-suite runs (nextest runs test binaries in parallel
across cores, unlike a single-process runner), but it does **not** explain
this gap — the ALIGN-1 line and the PERF-5 line come from the same
single-process `profile_intern` runner, so the speedup factor between them
is the machine, not the test harness.

**Consequence**: absolute times of the PERF series (M1) and the ALIGN
series (M2) are **not comparable to each other**; only ratios within one
campaign on one machine are. The lisage/OxiDD ratio of campaign-3
(2.04x at n=24) remains exact: the Docker witnesses share the host, so a
machine change applies to all four tools at once and inter-tool ratios
stay valid (ALIGN-7c).
