# ADR-0111: Rebase-first merges - the reviewed commits land one idea at a time

- Status: draft (needs-human)
- Date: 2026-11-25

## Context

ADR-0009 pinned the linear merge method as "squash preferred, a rebase
merge as the fallback": one commit per pull request, the whole review
trace compressed in one place. The saucisson contribution convention
(project-template, engineering-rules I3) is the opposite: rebase-only
merges, squash inside the branch when needed, so every branch carries
one commit per idea and the merge replays them. The owner directed on
2026-11-25 that lisage aligns with the template: this repository is
rebase-first too. This draft supersedes only that one sentence of
ADR-0009; every other clause of the branch model stands unchanged.

## Decision

- The merge driver resolves the linear method in the order (rebase,
  squash): a rebase merge is preferred, a squash merge is the accepted
  fallback when the repository forbids rebase merges. A merge commit
  is still never issued; linear history on `daily` is unchanged.
- Branch hygiene carries the weight the rebase merge puts on it: one
  idea per commit on the feature branch, rebase on `daily` before
  opening the pull request (already the rule in docs/branching.md), so
  the rebase merge replays exactly the commits the review examined.

## Consequences

- `daily` carries commit-granular history: the day report, the
  revert-first protocol and the end-of-day audit reason per commit,
  not per pull request.
- The pull request trace stays whole at the platform level (the review
  comment, the check runs); the squash single-commit summary
  disappears.
- A rebase merge refuses when the branch is not cleanly replayable -
  which the merge criteria already name for what they are.
- This ADR is a draft until the human accepts it; the driver change
  ships with it, and both stay on `daily` under the human end-of-day
  review either way.
