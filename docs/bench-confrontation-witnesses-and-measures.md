# Confrontation benchmarks: witnesses and measures (M7)

Protocol of the comparative benchmark campaign lisage vs external BDD
libraries (OxiDD today, CUDD/Sylvan when applicable), ADR-0102 pattern:
the witnesses run in local Docker containers, **never in CI**. This
document is the protocol the campaign follows (issue #115); the results
report lives in `bench-results/` (issue #116).

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
