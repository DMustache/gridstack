# Sprint 01 — M0 Stabilize — 2026-11-10
Goal: Fix the sync write-on-read and long-poll bugs, put startup config (server name, secrets) on a safe footing, give infrastructure failures a proper 500 error, and make identity keys real ed25519.

| # | Task | Depends | Parallel group | Status |
| --- | --- | --- | --- | --- |
| #14 | fix(infrastructure): pre-existing Ultra Clippy errors on develop (added mid-sprint) | — | B0 | done (PR #17) |
| #7 | fix(infrastructure): `DomainError::Unavailable` → 500 M_UNKNOWN; authorization sites | #14 | C1 | done (PR #18) |
| #10 | fix(infrastructure): required `server_name`, no startup panic | #14 | B1 | done (PR #19) |
| #1 | fix(synchronization): sync writes on read; drain to-device queue (migration, opus) | #14 | A1 | done (PR #20, lead pre-review changes applied) |
| #11 | fix(rooms): classify infrastructure errors in rooms persistence (pulled in) | #7 | C2 | done (PR #21) |
| #4 | fix(infrastructure): secrets from env; untrack config; refuse leaked values | #10 | B2 | done (PR #22) |
| #3 | fix(identity): real ed25519 key pairs | #4 | B3 | done (PR #23, 1 review round) |
| #2 | fix(synchronization): long-poll wakes on new events; timeout clamp (opus) | #1, #10 | A2 | done (PR #25, 1 review round; #24 superseded) |

Start coverage: 26/157 (16.6%)  Start test count: 10
End coverage: 26/157 (16.6%)  End test count: 48 (5 `#[ignore]`, need `DATABASE_URL`)
Coverage delta: 0 (expected: M0 fixes existing behaviour, no new operations). Test delta: +38.
`develop` gate at close: **QA GATE PASSED** on `be3e752`.

## Done
- 8/8 planned and added tasks merged. 2 PRs needed one review round (#23: ephemeral seed was discarded; #24/#25: rebase fallout, example config, error convention).
- Audit issues closed: V1 (#1), V2 (#2), V3 (#3), V4 (#4, code part), V7 (#7, core + authorization; rooms in #11), V10 (#10).

## Carried over
- #12 (synchronization error classification) and #9 (single pool): overlap files with in-flight #2/#3, so they were held back on purpose.
- #6 (Postgres sessions; needs the migration slot) and #8 (peer-IP rate limit): not started (depend on #9).

## Blockers
- #14: `develop` itself failed Ultra Clippy (rust 1.99.0 lints). All three wave-1 workers blocked on it (#15 and #16 closed as duplicates). Fixed first, then wave 1 was resumed.

## needs-human
- #13: rotation of the leaked secrets and the git-history policy. Recommendation 1A + 2A: rotate everything (pepper reset for dev accounts), no history rewrite. **Local impact of #4 is posted there:** pulling `develop` deletes the untracked local `configuration.toml` (restore with `git show b26b25f:configuration.toml > configuration.toml`), and startup refuses the leaked JWT secret and pepper.

## Decisions taken
- Process files committed to `develop` (`b26b25f`). The owner's WIP (failing #1 regression tests) was pushed as `wip/1-sync-regression-tests` and merged via #20. The owner's local checkout was never modified.
- #1: to-device rows in Postgres, sharing the `room_sync_stream` sequence; delete-on-ack in one transaction; `next_batch = max(current, since)`. Presence is written only when `set_presence` is given; spec-default `online` presence updates move to M4.
- #2: in-process `tokio::sync::watch` (multi-process deferred to M4). `timeout` is clamped to 30 s by default. Only room event writes publish; to-device (M5), account data, receipts, typing and presence (M4) are follow-ups.
- #10: required `server_name`; the dev config keeps `127.0.0.1:3000`.
- #4: `GRIDSTACK__<SECTION>__<KEY>` env overrides; SHA-256 deny-list of the leaked values. The section name `authenification` (typo) is kept, with a serde alias `authentication`.
- #3: `ed25519-dalek`; seed-only config; mismatched stored pairs are regenerated at bootstrap; ephemeral seeds are persisted. Follow-up: seeds are plaintext at rest.
- #7: `M_INVALID_PARAM` vs `M_BAD_JSON` split moves to M1. Accepted: no L2 fake-port tests for authorization, because the module has no fakes yet (M1 test harness).
- #5 moved to M1 (async DB boundary, option A `spawn_blocking`, module by module).
- Persistence (L4) tests exist but are `#[ignore]`; they first run in CI (M1).

## Process notes
- Delta worktree copies inherit the owner's uncommitted files. Fixed in `.claude/agents/worker.md` (this commit): stash the copies inside the worker's own worktree, never discard, commit or nest worktrees.
- Auto mode gave no verdict on `SendMessage`, so finished workers could not be resumed. Workaround used all sprint: a `## Resume` section at the top of the issue body, then a fresh worker. Cost: each resume re-reads the contract.
- Force-push to task branches is denied, so rebased work landed on new branches. Stale remote branches to clean up (owner or next sprint): `task/7-unavailable-error`, `task/10-impl`, `task/1-sync-to-device-postgres`, `task/2-sync-long-poll-wakeup`, `wip/1-sync-regression-tests`.
- The lead verified the gate itself on the merged result for the two hot-path PRs (#20, #25).

## Next sprint goal (02, M0 Stabilize)
Finish M0: #9 single checked pool (fail fast), #12 sync error classification, #6 durable hashed sessions in Postgres (migration slot), #8 peer-IP rate limiting with bounded buckets. Then check the M0 exit criteria and open M1.
