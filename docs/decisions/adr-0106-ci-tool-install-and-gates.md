# ADR-0106: CI tool installation via a pinned action, and the CI gate set

- Status: accepted (accepted by the human owner on 2026-11-20)
- Date: 2026-11-20

> Draft written in the CI-speedup PR: a project ADR is *drafted* in a PR
> (numbered from 0100 upward); only the human accepts it. The 0001-0099
> range belongs to the template.

## Context

The `continuous-integration` workflow (`.github/workflows/ci.yml`) spent
most of its wall time compiling the Rust tooling it needs
(`cargo install cargo-nextest --locked`, `cargo-llvm-cov`, `cargo-audit`,
`cargo-deny`) on every run, because each job started from a bare
`ubuntu-latest` image. Measured on the PR runs of 2026-11-20: the
`rust-check` job alone took ~6-8 minutes, most of it `cargo install`.

Two further observations:

- the criterion bench step (`cargo bench --all --bench '*'`) ran in CI
  although the confrontation protocol (ADR-0102) keeps benchmarks local
  only; it is a regression gate of its own kind but not a quality gate,
  and it adds minutes to every PR;
- `cargo audit` ran twice (once in `rust-check`, once in `security`),
  doubling the same install.

`.github/workflows/ci.yml` is a constitutional file (lint/type/coverage
configuration, ADR-0007 quality gates), and `taiki-e/install-action` is
a new third-party dependency: both are on the ADR-0010 escalation list,
hence this ADR.

## Decision

1. **Install the cargo tools through `taiki-e/install-action`, pinned by
   commit SHA** — `taiki-e/install-action@75a7c0f87bd666af3cdb75c5b9e9d03ab6044e6f`
   (tag `v2`). The action uses the prebuilt binaries from the tool's
   releases (or builds from a locked source), which removes the
   `cargo install` compile time. Pinning by SHA (not the mutable `v2`
   tag) keeps the dependency reviewable and tamper-evident.
2. **Add `concurrency` with `cancel-in-progress: true`** so a re-pushed
   branch cancels its obsolete runs instead of queueing them.
3. **Remove the criterion bench step from CI** — benchmarks stay local
   (ADR-0102); the frozen thresholds are enforced by the local
   `make check` / bench runners, not by CI wall time.
4. **Coverage via `cargo llvm-cov nextest`** — the coverage gate measures
   the same test suite the `nextest` gate runs, through the same runner,
   instead of a second `cargo test`-based pass.
5. **Run `cargo audit` once**, in the `security` job only.

## Consequences

- CI wall time per PR drops by several minutes (no more `cargo install`
  of the four tools; no duplicated audit; no criterion bench).
- The new external dependency is a single action, pinned by SHA, used
  only in CI; it needs no repository permissions beyond what the
  workflow already has.
- The quality gates themselves (ruff, mypy, import-linter, pytest with
  the 90% coverage floor, rustfmt, clippy `-D warnings`, nextest,
  coverage, deny, audit, gitleaks, pip-audit) are unchanged in what
  they check; only how the tools are installed and which runner measures
  coverage changes.
