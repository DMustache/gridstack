# Roadmap

Target: a fast, spec-complete Matrix Client-Server API v1.18 homeserver. Order: **fix → refactor → implement → comply**. Milestone titles must match GitHub milestones exactly (created by `scripts/agents/bootstrap_github.sh`).

Milestone exit criteria are checked by the lead before opening the next milestone.

## M0 Stabilize
Close audit issues #1–#10 (V1–V10).
- Exit: all ten closed with regression tests; `develop` QA gate green; no `/sync` writes on read; no blocking Diesel on the async runtime in request paths (or tracked in M1 with a measured plan); secrets loadable from environment.

## M1 Foundations
Refactor so every later task is small.
- One async DB boundary (single pool, `spawn_blocking` or async driver) with typed `PersistenceError`.
- One Matrix error model: typed service errors → `{errcode, error}` + status, no raw DB text leaks.
- Typed identifiers (`UserId`, `RoomId`, `EventId`, `DeviceId`) validated per spec grammar.
- Test harness: port fakes, router test helper, DB test helper (`DATABASE_URL`), scenario test helper.
- GitHub Actions CI running `scripts/agents/qa_gate.sh` (+ L4/L5 with a Postgres service).
- Tasks registered: server run, DB migrate, QA gate, spec coverage.
- Exit: gate green in CI; every service module has L2 tests; refactor tasks did not change behavior (characterization tests).

## M2 Accounts & Auth
Registration flows + UIA, login types, refresh, logout/all, devices CRUD, account password/deactivate/3pid (subset documented), whoami, capabilities, versions, well-known, profile.

## M3 Rooms & Events
Room versions, create/join/leave/forget/kick/ban/unban, invite, aliases & directory, state & power levels authorization, redactions, relations/threads, context, event by id, timestamp_to_event, reporting, tags, account data, upgrades.

## M4 Sync
Spec-complete `/sync` (rooms join/invite/leave/knock, state, timeline limits, `limited`/`prev_batch`, ephemeral, account data, to-device, device lists, OTK counts), filters, long-poll wake on new events, typing, receipts, presence. Performance budget: idle long-poll ≈ 0 CPU; incremental sync p95 < 20 ms on test dataset.

## M5 E2EE
`keys/upload|query|claim|changes`, to-device `sendToDevice`, device list tracking, cross-signing, key backup (room_keys), dehydrated devices if in v1.18 stable.

## M6 Media
Authenticated media (`/_matrix/client/v1/media/*`): upload, download, thumbnail, config, preview_url (guarded), async upload.

## M7 Push & Notifications
Pushers, push rules evaluation (default ruleset), `/notifications`, unread counts in sync.

## M8 Spec Completion
Everything remaining in `spec-coverage.md` (search, OpenID, third-party, SSO redirect, admin whois, etc.) until coverage = 100%.

## M9 Compliance & Perf
Complement CS-API subset in CI, load tests, profiling hot paths, release hardening, deployment docs.

## Beyond CS API (owner decision)
A fully interoperable homeserver also needs the Server-Server (federation) API. The repository scope (`AGENTS.md`) is Client-Server v1.18, so federation is **not** planned until the owner approves it; the lead opens a `needs-human` issue when M8 completes.
