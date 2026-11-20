# Confrontation benchmarks: witnesses and measures (M7)

Protocol of the comparative benchmark campaign lisage vs external BDD
libraries (OxiDD today, CUDD/Sylvan when applicable), ADR-0102 pattern:
the witnesses run in local Docker containers, **never in CI**. This
document is the protocol the campaign follows (issue #115); the results
report lives in `bench-results/` (issue #116).

## Machine (ALIGN-7c)

Every publication in `bench-results/` opens with a mandatory **Machine**
header: the short machine identifier from the register
`bench-results/machines.md` (ALIGN-7b), followed by the normalized
fingerprint JSON emitted by `scripts/machine_fingerprint.py` (ALIGN-7a)
in a fenced code block:

```
Machine: M2

```json
{"machine_id": "M2", "hardware_hash": "febd14ed", ...}
```
```

Rules:

- **No absolute-time comparison across machines.** When the machine
  changes, absolute wall-clock times of different campaigns are not
  comparable; comparisons are made in **ratios** (e.g. lisage/OxiDD of
  the same charge) only.
- **Docker witnesses share the host.** A machine change applies to all
  four tools at once, so inter-tool ratios of one campaign remain valid
  across machines.
- Deterministic counters (allocations, bytes/node, node counts) are
  hardware-insensitive and stay comparable in absolute value across
  machines.

## Charges (M7-4)

Real, versioned inputs in `crates/lisage-bench/models/` (sources in its
README), each with a pinned cross-check value in
`crates/lisage-bench/tests/confrontation.rs`:

| Label | Input | Charge built |
|---|---|---|
| `cnf-uf20-01` | SATLIB `uf20-01.cnf` (20 vars, 91 clauses) | BDD of the CNF function |
| `pnml-swimmingpool-1` | MCC `SwimmingPool-PT-01` (9 places) | BDD of the transition relation over 2p = 18 variables |
| `pnml-erk-000001` | MCC `ERK-PT-000001` (11 places) | BDD of the transition relation over 2p = 22 variables (heavy) |

All charges are Boolean functions serialized as truth tables in the
standard stdin protocol (`n`, then `2^n` values `0`/`1`), produced by
`confrontation dump-charge <workload> <path>` — the witnesses receive
the exact input lisage measured.

## Witnesses

- **lisage**: the `confrontation` binary (`cargo build --release -p
  lisage-bench --bin confrontation`), one JSONL line per run.
- **OxiDD**: the `lisage-oxidd:latest` container
  (`docker build -t lisage-oxidd:latest docker/oxidd`, OxiDD pinned at
  commit `c7a458df8b747a3ee44d0e9e61059a1621865fcd`), `oxidd_count`
  reading the charge on stdin, one fresh container per measurement. The
  unique-table / apply-cache capacities default to 2^20 entries each and
  can be raised per charge through the `OXIDD_INNER_CAPACITY` /
  `OXIDD_APPLY_CACHE_CAPACITY` environment variables (BENCH-1 crossover
  charges).
- **CUDD**: the `lisage-cudd:latest` container
  (`docker build -t lisage-cudd:latest docker/cudd`, CUDD pinned at tag
  `cudd-3.0.0` = commit `fc30c18f8db2e7d03d13241038b61eb0056a734e`),
  `cudd_measure` reading the charge on stdin, one fresh container per
  measurement. Node count in the lisage convention: distinct
  (node, complement-bit) pairs reachable from the root, terminals
  included (the raw `Cudd_DagSize` is not comparable — complement-edge
  sharing, documented in WORKLOG).
- **libDDD (LIP6)**: the `lisage-libddd:latest` container
  (`docker build -t lisage-libddd:latest docker/libddd`, pinned at commit
  `b775aed3e8405a8906b10f937f6dbcc4a4b8831c`), `libddd_measure` reading
  the charge on stdin, one fresh container per measurement. libDDD has no
  complement edges, so `GDDD::size()` (distinct reachable nodes, terminals
  included) is directly the lisage convention.
- **Sylvan**: not yet dockerized (spike SP-2, issue #154).

## Fairness rules

- **Same input**: the witnesses read the truth table serialized by
  lisage itself (`dump-charge`), not a re-implementation.
- **Same object built**: the BDD of the charge, by Shannon expansion
  (bit ℓ of the index = variable ℓ, variable 0 at the root).
- **Same cross-check value**: `noeuds` (BDD node count, terminals
  included) and `verifie` (satisfying assignments) must be identical
  across tools for the same charge. Any divergence invalidates the
  campaign (the orchestrator exits 1 and records the divergence).

## Measured block

Identical across all tools: **input reading + diagram construction +
verification counting**. Library initialization (manager, workers,
pre-declared variables) is outside the block at the witnesses: it varies
from one tool to another and is not the compared workload. (lisage
creates its pool inside the block, right after the parse that yields the
variable count — noted in the code; the witness pre-declares its
variables upfront.)

## Time and memory

- **Time**: wall-clock of the measured block, in milliseconds
  (`temps_ms`).
- **Memory**: peak RSS of the whole process (`VmHWM` of
  `/proc/self/status`, in kibibytes, `rss_max_ko`). One **fresh process
  per measurement** — the peak RSS does not divide into in-process
  repetitions. For the witnesses, one fresh Docker container per
  measurement.

## Repetitions and aggregation

The orchestrator `python -m scripts bench-confrontation`
(`make bench-confrontation ARGS="..."`) runs each charge x tool
`--repetitions` times (default 5), deposits the raw JSONL lines in
`bench-results/<run-id>/<charge>-<outil>.jsonl` and the aggregates
(median, min, max of `temps_ms` and `rss_max_ko`, plus the constant
`noeuds`/`verifie`) in `bench-results/<run-id>/aggregats.json`.

## Reproduction

```bash
cargo build --release -p lisage-bench --bin confrontation
docker build -t lisage-oxidd:latest docker/oxidd
make bench-confrontation ARGS="--repetitions 5 --run-id <run-id>"
```

The exact command of each deposited run is recorded in
`bench-results/<run-id>/README.md`.

## Deposited results

- `bench-results/20261117-campaign-1/` — first campaign (M7-5): raw
  JSONL, aggregates, reproduction command, and the comparative report
  `REPORT.md` (M7-6).
- `bench-results/20261118-campaign-2/` — BENCH-1 campaign (issue #277):
  lisage vs OxiDD vs CUDD vs libDDD on the crossover charges
  (`alea-lcg` n = 20/22/24, `cnf-uf20-01`, `pnml-swimmingpool-1`), with
  the OxiDD ~4x gap analysis and the memory crossover in `REPORT.md`.
