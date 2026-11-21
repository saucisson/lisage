# CI troubleshooting — stalled CI usually means a merge conflict

Symptom: a pushed PR head shows **no CI run at all** (empty
`statusCheckRollup`, no workflow run for the head SHA) for tens of minutes.

Cause: for `pull_request` events GitHub Actions runs on the PR *merge ref*.
When the branch **conflicts with the base** (`mergeable: CONFLICTING`, e.g.
`WORKLOG.md` or `crates/lisage-liseuse/src/lib.rs` touched on both sides),
no merge ref can be built and **no workflow run is created** — the stall is
not a runner problem, it is a conflict.

Rule: whenever CI looks stalled on a PR, check `gh pr view N --json
mergeable` **first** and rebase the branch onto the current base before
suspecting the runner fleet:

```bash
gh pr view N --json mergeable --jq '.mergeable'   # CONFLICTING → rebase
git fetch origin daily && git rebase origin/daily
```

Conflict resolution conventions on rebase:

- `WORKLOG.md` is append-only: keep **all** entries from both sides (base
  entries first, then the branch's new entries under the
  `<!-- Append new entries directly below this line. -->` marker), then
  verify with `grep -c` that no known entry was dropped.
- `crates/lisage-liseuse/src/lib.rs`: union of the `pub mod` lines.
- Then `git push --force-with-lease`; the `synchronize` event fires CI on
  the fresh, conflict-free head.
