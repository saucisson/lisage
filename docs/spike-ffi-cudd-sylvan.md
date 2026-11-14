# Spike — FFI CUDD/Sylvan (M1-8, technical risk no. 1)

Status: **human decision received (2026-11-10)** — CUDD in a local Docker container, outside CI. No dependency added to the project (escalation list in AGENTS.md respected). The structured question was asked in a comment on issue #14.

## Why bindings (roadmap § risks recap)

The reference interpreter only sees the semantics, never the form. "Identical reproduction" (C6) is measured in **number of nodes** against an independent library: a homemade mini-BDD would share the same author's misunderstandings. Sylvan adds a second independent witness (later carrying saturation and LDD).

## What ships without FFI (degraded mode, this PR)

- Controller configuration in **classic BDD** (`lisage-core::bdd`): explicit 0/1 edges, no R-default, R-collapse + R-share only.
- **Independent witness mini-BDD** in pure Rust (`lisage-bench::temoin_bdd`), written specifically for the differential: PBT C6 (identical node count + identical semantics over 2^n assignments, random functions).
- **Bench baseline**: classic BDD n=12 (deterministic LCG table), node count anchored by criterion in CI.

## The real question: the CI build

CUDD and Sylvan are C/C++ libraries: they must be compiled in the CI container (or precompiled images provided), on all targeted platforms. Out-of-process does not remove the risk. In-process, the differential PBT is immediate (random function → two constructions → counts compared on the spot).

## Proposed sequencing (roadmap)

- **M1**: CUDD alone (classic BDD witness, C6).
- **M2**: Sylvan when saturation (M2-4, Hamez-style) requires it.

## Options for the dependency

| Option | Advantage | Disadvantage |
| --- | --- | --- |
| `cudd-sys` (bindgen, build CUDD in CI) | real external witness, strong C6 | heavy CI build, platforms |
| precompiled CI container + out-of-process comparison binary | simple CI | no in-process PBT, comparison via files |
| stay on the internal mini-BDD (degraded mode) | zero dependency | weakened C6 (same author) |

## Human decision (2026-11-10)

> Not in CI, but in local tests on this machine. Installation if possible in a Docker container.

- **CI**: no C/C++ dependency. The internal mini-BDD remains the CI witness (degraded mode, `make check` unchanged).
- **Local**: Docker image `lisage-cudd:latest` (see `docker/cudd/`) embeds compiled CUDD 3.0.0 + the `cudd_count` binary (reads a truth table on stdin, returns the CUDD node count).
- **Local differential test**: `crates/lisage-bench/tests/cudd_local.rs` (`#[ignore]`, outside CI) generates random functions, calls the container via `docker run`, and compares the node count with the internal BDD. Execution:
  ```sh
  docker build -t lisage-cudd:latest docker/cudd
  cargo test -p lisage-bench --test cudd_local -- --ignored
  ```

## Impact

C6 is verified locally against an independent external witness (real CUDD). The published baselines (M4-4, M5-2) can rely on these local measurements. In CI, the differential remains internal (mini-BDD).
