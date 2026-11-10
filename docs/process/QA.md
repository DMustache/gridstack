# QA policy

Every task ships with tests. Minimum: **unit tests for every new or changed function with logic**. Additional layers are required by what the task touches.

## Test layers

| Layer | Where | Tools | Needs DB | Runs in gate |
| --- | --- | --- | --- | --- |
| L1 Unit (pure domain: IDs, validation, parsing, token math) | `#[cfg(test)] mod tests` beside the code | `#[test]` | no | yes |
| L2 Service (business rules, typed errors) | `service/tests.rs` in the module | `#[tokio::test]` + in-memory fakes implementing the module's port traits | no | yes |
| L3 Router (HTTP status, JSON shape, Matrix `errcode`, auth) | `handlers/tests.rs` or `tests/` | `tower::ServiceExt::oneshot` against the module router with fake services | no | yes |
| L4 Persistence (queries, migrations, constraints) | `persistence/tests.rs` | real Postgres; `#[ignore]` unless `DATABASE_URL` set | yes | CI only (M1) |
| L5 Scenario (multi-endpoint flows: register → createRoom → send → sync) | `tests/*.rs` | full app router, test DB | yes | CI only (M1) |
| L6 Compliance | Complement CS suite | Docker | yes | M9 |
| P Performance (hot paths: `/sync`, `send`, `messages`) | `benches/` | criterion | optional | on request |

## Required layers by change type ("QA by dependency")

| Task touches | L1 | L2 | L3 | L4 | L5 | P |
| --- | --- | --- | --- | --- | --- | --- |
| Domain type / validation / parsing | ✔ | | | | | |
| `service.rs` business rule | ✔ | ✔ | | | | |
| New or changed endpoint (route/handler) | ✔ | ✔ | ✔ | | | |
| Persistence query / migration | ✔ | | | ✔ | | |
| Cross-module flow (e.g. rooms → sync) | ✔ | ✔ | ✔ | | ✔ | |
| Hot path (`/sync`, send, messages, auth middleware) | ✔ | ✔ | ✔ | | | ✔ (no regression) |
| Bug fix | regression test at the lowest layer that reproduces it, written **first** and shown failing |
| Refactor (no behavior change) | existing tests must pass unchanged; add L2 characterization tests first if the module has none |

## Rules

- Router tests assert exact `errcode` and status for each error the contract lists.
- Fakes live next to the port trait (`#[cfg(test)]`), one per trait, reused across tests. Do not add mocking crates without a task.
- Tests must be deterministic: no sleeps > 50 ms, no wall-clock assertions, no network.
- Test names describe behavior: `idle_syncs_return_no_to_device_events`.
- A PR that lowers the passing test count is rejected.

## Gate

`scripts/agents/qa_gate.sh` runs, in order: `Cargo: Format`, `Cargo: Check`, `Cargo: Ultra Clippy (Strict)`, `Cargo: Test`. Result: `generated/reports/qa-gate.txt`. A task is reviewable only when the last line is `QA GATE PASSED`.
