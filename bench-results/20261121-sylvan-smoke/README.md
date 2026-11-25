# Smoke run 20261121-sylvan-smoke (issue #308, closes SP-2 #154)

Machine: M1

```json
{"cpu_model": "Intel(R) Core(TM) i5-3470 CPU @ 3.20GHz", "date_utc": "2026-11-20T23:49:49+00:00", "hardware_hash": "febd14ed", "kernel": "6.18.58.1-microsoft-standard-WSL2", "logical_cores": 4, "machine_id": "M1", "page_size": 4096, "physical_cores": 4, "ram_total_kib": 3979828, "sources": {"cpu_model": "/proc/cpuinfo: model name", "kernel": "uname -r (platform.release())", "logical_cores": "/proc/cpuinfo: count of 'processor' entries", "machine_id": "/home/alban/lisage/bench-results/machines.md: hardware hash match", "page_size": "os.sysconf(SC_PAGE_SIZE)", "physical_cores": "/proc/cpuinfo: unique (physical id, core id) pairs", "ram_total_kib": "/proc/meminfo: MemTotal", "wsl2_config": "no /mnt/c/Users/*/.wslconfig reachable from inside WSL2"}, "wsl2_config": null}
```

First confrontation run with the **Sylvan** witness dockerized
(`docker/sylvan/`, Sylvan pinned at tag `v1.12.8` =
`fe8f9463a5f64ea8542534d82dd74a6766de127b`): one real charge
(`cnf-uf20-01`), all five tools, 3 repetitions. Protocol
`docs/bench-confrontation-witnesses-and-measures.md`.

## Exact reproduction command

```bash
cargo build --release -p lisage-bench --bin confrontation
docker build -t lisage-oxidd:latest docker/oxidd
docker build -t lisage-cudd:latest docker/cudd
docker build -t lisage-libddd:latest docker/libddd
docker build -t lisage-sylvan:latest docker/sylvan
uv run python -m scripts bench-confrontation --repetitions 3 \
    --run-id 20261121-sylvan-smoke --charges cnf-uf20-01 \
    --tools lisage,oxidd,cudd,libddd,sylvan
```

(machine: M1, 2026-11-20 evening; one fresh process per measurement for
lisage, one fresh container per measurement for the external witnesses;
OxiDD pinned at commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`, CUDD
at tag `cudd-3.0.0` = `fc30c18f8db2e7d03d13241038b61eb0056a734e`, libDDD
at `b775aed3e8405a8906b10f937f6dbcc4a4b8831c`, Sylvan at tag `v1.12.8` =
`fe8f9463a5f64ea8542534d82dd74a6766de127b`.)
