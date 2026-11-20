# Campaign 20261117-campaign-1 (M7-5, issue #115)
Machine: M1


Comparative measurements lisage vs OxiDD on the M7-4 real charges,
protocol `docs/bench-confrontation-witnesses-and-measures.md`.

## Exact reproduction command

```bash
cargo build --release -p lisage-bench --bin confrontation
docker build -t lisage-oxidd:latest docker/oxidd
uv run python -m scripts bench-confrontation --repetitions 5 --run-id 20261117-campaign-1
```

(machine: WSL2 local developer machine; OxiDD pinned at commit
`c7a458df8b747a3ee44d0e9e61059a1621865fcd`; one fresh process per
measurement for lisage, one fresh container per measurement for OxiDD).

## Contents

- `<charge>-<outil>.jsonl`: one JSON line per repetition (raw).
- `aggregats.json`: median/min/max of `temps_ms` and `rss_max_ko` per
  charge x tool, plus the constant cross-check values `noeuds`/`verifie`
  and the cross-tool `divergences` list (empty: the campaign is valid).

## Cross-check (must be identical across tools)

| Charge | noeuds | verifie |
|---|---|---|
| cnf-uf20-01 | 51 | 8 |
| pnml-swimmingpool-1 | 1859 | 1408 |
| pnml-erk-000001 | 9721 | 8672 |
