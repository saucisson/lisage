# Campaign 20261118-campaign-2 (BENCH-1, issue #277)
Machine: M1


Comparative measurements lisage vs OxiDD vs CUDD vs libDDD (LIP6) on the
BENCH-1 crossover charges: `alea-lcg` at n = 20/22/24 (time and RSS
tracked against the node count), plus the real charges `cnf-uf20-01` and
`pnml-swimmingpool-1`. Protocol
`docs/bench-confrontation-witnesses-and-measures.md`.

## Exact reproduction command

```bash
cargo build --release -p lisage-bench --bin confrontation
docker build -t lisage-oxidd:latest docker/oxidd
docker build -t lisage-cudd:latest docker/cudd
docker build -t lisage-libddd:latest docker/libddd
uv run python -m scripts bench-confrontation --crossover \
    --tools lisage,oxidd,cudd,libddd --repetitions 5 \
    --run-id 20261118-campaign-2
```

(machine: local developer machine, 2026-11-18; one fresh process per
measurement for lisage, one fresh container per measurement for the
external witnesses; OxiDD pinned at commit
`c7a458df8b747a3ee44d0e9e61059a1621865fcd`, CUDD at tag `cudd-3.0.0` =
`fc30c18f8db2e7d03d13241038b61eb0056a734e`, libDDD at
`b775aed3e8405a8906b10f937f6dbcc4a4b8831c`. For `alea-lcg-24` the OxiDD
witness runs with `OXIDD_INNER_CAPACITY=OXIDD_APPLY_CACHE_CAPACITY=2^23`
(1 071 698 nodes exceed the default 2^20 unique-table capacity; the
default image stays the reference for the other charges).)

## Contents

- `<charge>-<outil>.jsonl`: one JSON line per repetition (raw).
- `aggregats.json`: median/min/max of `temps_ms` and `rss_max_ko` per
  charge x tool, plus the constant cross-check values `noeuds`/`verifie`
  and the cross-tool `divergences` list (empty: the campaign is valid).
- `REPORT.md`: the comparative report (time/memory tables, the OxiDD ~4x
  gap analysis, the memory crossover).

## Cross-check (identical across all four tools)

| Charge | noeuds | verifie |
|---|---|---|
| alea-lcg-20 | 106 760 | 523 228 |
| alea-lcg-22 | 325 824 | 2 096 246 |
| alea-lcg-24 | 1 071 698 | 8 386 976 |
| cnf-uf20-01 | 51 | 8 |
| pnml-swimmingpool-1 | 1 859 | 1 408 |
