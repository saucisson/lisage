# Deployment channels (staging only, daily)

The merge is the deploy trigger: every push of `daily` deploys the
staging channel automatically, so a merged pull request never waits for
a manual staging step (the template doctrine N3, instantiated here for
ADR-0107). The split follows the branch model (ADR-0009): `daily` is
the agent blast radius, and staging mirrors it.

| Branch  | Channel  | Trigger | Human role |
| ------- | -------- | ------- | ---------- |
| `daily` | staging  | every merge into `daily` (the pull request flow) | tests lisage.linard.fr |
| `main`  | release  | deferred: no release channel exists yet | owns the release (ADR-0013) |

## The workflow contract

`.github/workflows/deploy.yml` implements the trigger:

- it runs on every push of `daily`; the release channel is not
  triggered, because this repository has no release deployment yet -
  the workflow grows the `main` side when one exists;
- it deploys the `staging` GitHub environment, so the channel stays
  separable in protection rules and deployment history;
- it reads the staging secrets and connects to the target host over
  SSH.

The remote command is deliberately a bare `true` today: the
distribution smoke of the template contract, wired now so the
connection, the secrets and the environment are proven before the WEB
milestone lands. The real distribution (the release build of
`lisage-web`, behind the reverse proxy, bound to the loopback
interface - the saucisson VPS convention) is WEB-6 work and replaces
the smoke in place; the per-repo implementation principle is N8's.

## Secrets of the staging channel

| Secret             | Meaning                     | Notes |
| ------------------ | --------------------------- | ----- |
| `STAGING_SSH_KEY` | key the runner connects with | dedicated to staging |
| `STAGING_HOST`    | staging host, the saucisson VPS | serves lisage.linard.fr |
| `STAGING_USER`    | user on the staging host    | a deploy user, not root |

The release channel needs no secrets while it does not exist.

## Staging in the loop

Staging equals `daily` by construction of the automatic deploy: a
staging deployment that drifted behind `daily` shows up immediately
on lisage.linard.fr. The deployment history of the `staging`
environment is the log to consult when it is in doubt.

## Rollback

The deployment history keeps the deployed commit SHAs; rolling back is
a revert on `daily`, then the trigger does the rest. The human merge
of `daily` into `main` stays the release decision (ADR-0009).
