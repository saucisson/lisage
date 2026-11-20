# Campaign 20261119-campaign-3 (PERF-4, issue #286)
Machine: M1


Post-PERF confrontation campaign: same charges, same witnesses, same protocol
as `20261118-campaign-2` (BENCH-1, issue #277), after PERF-2 (#282) and
PERF-3 (#284).

## Exact reproduction command

```bash
cargo build --release -p lisage-bench --bin confrontation
docker build -t lisage-oxidd:latest docker/oxidd
docker build -t lisage-cudd:latest docker/cudd
docker build -t lisage-libddd:latest docker/libddd
uv run python -m scripts bench-confrontation --crossover \
    --tools lisage,oxidd,cudd,libddd --repetitions 5 \
    --run-id 20261119-campaign-3
```

(machine: local developer machine, 2026-11-19; one fresh process per
measurement for lisage, one fresh container per measurement for the external
witnesses; OxiDD pinned at commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`,
CUDD at tag `cudd-3.0.0`, libDDD at `b775aed3e8405a8906b10f937f6dbcc4a4b8831c`.
For `alea-lcg-24` the OxiDD witness runs with
`OXIDD_INNER_CAPACITY=OXIDD_APPLY_CACHE_CAPACITY=2^23`, as in campaign-2.)

## Contents

- `<charge>-<outil>.jsonl`: one JSON line per repetition (raw).
- `aggregats.json`: median/min/max of `temps_ms` and `rss_max_ko` per
  charge x tool, plus the cross-check `noeuds`/`verifie` and the cross-tool
  `divergences` list (empty: the campaign is valid).
- `REPORT.md`: the comparative report — the lisage/OxiDD time ratio goes from
  3.7-4.5x (campaign-2) to 1.1-2.0x (campaign-3) on every charge.
