# ADR-0022: gh-first GitHub client and short CI (ported from project-template)

- **Status**: accepted (2026-11-23, ported from saucisson/project-template ADR-0008)
- **Date**: 2026-11-23
- **Deciders**: template owner

## Context

The project-template introduced two infrastructure improvements that
this repository had not yet adopted:

1. **gh-first GitHub client** (template ADR-0008): every driver calls
   `gh api <endpoint>` first, falling back to exactly one raw REST
   request with `GITHUB_TOKEN` when gh is missing, unsigned, or fails
   for any reason. The gh client owns authentication, so hand-coded
   HTTP in the drivers disappears.
2. **Short merge-path CI** (template ADR-0008): the GitHub CI is one
   merge signal among several gates (review, mutation at EOD), so it
   stays short - only fast gates (lint, types, unit tier, secret
   scan). The dependency audit (pip-audit, cargo audit) moves to a
   separate `dependency-audit.yml` workflow that triggers only when
   a lockfile moves or on a weekly schedule.

## Decision

Port both improvements:

1. `scripts/_github.py` is rewritten gh-first: `try_gh_api()` runs
   `gh api` once, `rest_fallback()` issues one raw REST request with
   `GITHUB_TOKEN`, and `github_request()` chains them. All drivers
   (`ci.py`, `merge.py`, `pulls.py`, `claims.py`, `review.py`,
   `night.py`) call `github_request()` instead of holding an
   `httpx.Client`. The REST fallback remains for unattended sessions
   where gh may be unavailable.
2. `.github/workflows/dependency-audit.yml` runs pip-audit and
   cargo-audit on lockfile changes and weekly. The main `ci.yml`
   drops pip-audit and cargo-audit from its security job, keeping
   only gitleaks, and gains `timeout-minutes` on every job plus a
   shallow checkout where history is not needed.
3. `.vibe/config.toml` allowlists `gh api`, `gh auth status`, `gh
   issue`, `gh label`, `gh milestone`, `gh pr`, `gh repo`.
4. `AGENTS.md` documents the gh-first rule and the short-CI
   principle.

## Consequences

- Drivers no longer manage their own HTTP clients; the gh-first
  chain centralizes transport concerns in `_github.py`.
- The merge-path CI is shorter and cheaper; dependency advisories
  are still caught, just on a different cadence.
- Unattended sessions work whether or not gh is authenticated: the
  REST fallback with `GITHUB_TOKEN` covers the gap.
