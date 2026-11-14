# ADR-0101: Pull request size budget raised to ~1 000 lines

- Status: accepted (direct human decision, 2026-11-10)
- Date: 2026-11-10

## Context

The pull request size budget was ~500 changed lines (AGENTS.md,
`docs/review-checklist.md`, `docs/agents/developer.md`,
`docs/agents/orchestration.md`, `docs/branching.md`, ADR-0019,
`.vibe/prompts/night-*.md`). In practice the budget repeatedly produced
TO-FIX findings on otherwise clean pull requests (e.g. PR #54 at 541
lines, where the sanctioned split proposal was pure overhead: the
"split" would have separated a self-checking bench from the code it
checks).

## Decision

The budget is raised to **~1 000 changed lines**. A pull request over
~1 000 changed lines still requires a split proposal in the pull
request body via `make update-pr` (ADR-0019). All references to the old
budget are updated in the same change: `AGENTS.md`,
`docs/review-checklist.md`, `docs/agents/developer.md`,
`docs/agents/orchestration.md`, `docs/branching.md`, ADR-0019,
`.vibe/prompts/night-issue.md`, `.vibe/prompts/night-auto.md`.

## Consequences

- The reviewer's size-budget finding now triggers over ~1 000 lines
  instead of ~500.
- The split discipline (one issue per PR, split earlier rather than
  larger) is unchanged; only the numeric threshold moves.
