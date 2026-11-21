# Local OxiDD witness container (ADR-0102 pattern)

Builds the image `lisage-oxidd:latest` with OxiDD pinned by commit SHA and a
`oxidd_count` binary that reads a truth table on stdin (same input format as
`cudd_count`: `n` then `2^n` values `0`/`1`) and prints the OxiDD BDD node
count on stdout.

**Never built or run in CI**: this directory is not referenced by
`Cargo.toml`, the CI workflows, or any build script (ADR-0102). It exists
only for local differential testing on the developer machine.

## Pinned version

- Repository: <https://github.com/oxidd/oxidd>
- Commit SHA: `c7a458df8b747a3ee44d0e9e61059a1621865fcd` (main, 2026-11-16)
- Paper: Husung, Dubslaff, Hermanns, Koehl — *OxiDD: A Safe, Concurrent,
  Modular, and Performant Decision Diagram Framework in Rust*, TACAS 2024.
  <https://doi.org/10.1007/978-3-031-57256-2_13>
- License: MIT OR Apache-2.0

## Build and use

```sh
docker build -t lisage-oxidd:latest docker/oxidd
printf '2\n0 1 1 0\n' | docker run --rm -i lisage-oxidd:latest oxidd_count
```
