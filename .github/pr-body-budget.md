## Summary

The pull request size budget is raised from ~500 to **~1 000 changed lines**, by direct human decision (2026-11-10: "la règle des 500 lignes semble avoir posé plusieurs fois problème. Augmente la limite à 1.000 lignes."). The old budget repeatedly produced TO-FIX size findings on otherwise clean PRs (e.g. #54 at 541 lines, where the split proposal was pure overhead). All files carrying the old limit are updated in the same change, and the decision is traced in ADR-0101.

## Issue closure

No issue. Direct human instruction.

## Acceptance criteria mapping

| Criterion | Where it is satisfied | Evidence |
| ---------- | --------------------- | -------- |
| Budget ~1 000 lines partout | `AGENTS.md`, `docs/review-checklist.md`, `docs/agents/developer.md` (×2), `docs/agents/orchestration.md`, `docs/branching.md`, ADR-0019, `.vibe/prompts/night-issue.md`, `.vibe/prompts/night-auto.md` | `grep -rn "500"` on those files: no match |
| Décision tracée | `docs/decisions/adr-0101-budget-pr-1000-lignes.md` (status: accepted, décision humaine directe) | — |

## Testing evidence

- `grep -rn "500" AGENTS.md docs/review-checklist.md docs/agents/developer.md docs/agents/orchestration.md docs/decisions/adr-0019-pull-request-body-updates.md docs/branching.md .vibe/prompts/night-issue.md .vibe/prompts/night-auto.md` - no output (plus aucune occurrence).
- `make check` - result: OK (docs-only change, test suite unchanged).

## Judgement calls

- The change touches constitutional files, but the instruction comes directly from the human owner, which outranks `AGENTS.md` in the instruction hierarchy — no `needs-human` escalation needed, the human decision is the authorization. ADR-0101 records it.
- French typography "~1 000" used in prose, matching the user's "1.000".

## Known limitations

None.

## Checklist

- [x] Constitutional files touched — authorized by direct human instruction; ADR-0101 included.
- [x] No new dependency added.
- [x] `Closes #n` — not applicable (no issue).
- [x] Full `make check` run on this branch.
