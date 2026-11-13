# ADR-0102: Local Docker witnesses for external BDD libraries (CUDD)

- Status: accepted (direct human decision, 2026-11-10)
- Date: 2026-11-10

## Context

M1-8 (issue #14, statement C6) requires a differential test against a real
external BDD library (CUDD) to measure node counts independently. Adding
`cudd-sys` as a Rust dependency would mean compiling C/C++ in CI on all
platforms — the #1 technical risk identified in the roadmap. The human
decided on 2026-11-10: no CUDD in CI, but local tests on this machine,
installed in a Docker container if possible.

## Decision

A new top-level directory **`docker/`** holds Dockerfiles for local
witness containers. These containers are **never built or run in CI**;
they exist only for local differential testing on the developer machine.

- `docker/cudd/` builds the image `lisage-cudd:latest` with CUDD 3.0.0
  (pinned by commit SHA) and a `cudd_count` binary that reads a truth
  table on stdin and prints the CUDD node count.
- The corresponding Rust test (`crates/lisage-bench/tests/cudd_local.rs`)
  is `#[ignore]` by default and excluded from `make check`; it runs only
  manually via `cargo test -- --ignored` with Docker available.

## Consequences

- CI stays pure Rust with zero C/C++ dependencies; the in-CI witness
  remains the internal mini-BDD (degraded mode).
- C6 is verified locally against a real external witness (CUDD).
- Any future external witness (Sylvan, etc.) gets its own subdirectory
  under `docker/` following the same pattern.
- The `docker/` directory is not referenced by `Cargo.toml`, the CI
  workflow, or any build script.
