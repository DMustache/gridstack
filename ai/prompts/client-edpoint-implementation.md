Implement endpoint support for the device-side client app with parameterized inputs.

Server/client context:
- `src/` is the Gridstack server.
- `client/` is a standalone client application running on the user device and communicating with the server over HTTP.
- Treat `client/` as an independent app, not as a frontend layer inside `src/`.

## Parameters
Set only when overriding defaults:
- `{{API_BASE_URL}}` (required for runnable examples, example: `http://127.0.0.1:8080`)
- `{{ENDPOINT_SCOPE}}` (`all` | `rooms,media,sync`; default: `all`)

Implicit defaults (do not repeat unless overriding):
- `CLIENT_ROOT=client`
- `SERVER_ROOT=src`
- `REFERENCE_ROOT=reference`
- `REPORT_EXTRA_ENDPOINTS=true`
- `ALLOW_SHARED_MODELS_FROM_SRC=true`
- `STRICT_NO_SERVER_COUPLING=true`

## Task
Implement endpoint support in `CLIENT_ROOT` for endpoints currently available in `SERVER_ROOT`, following applicable `AGENTS.md` instructions.

## Required workflow
1. Discover endpoint inventory in `SERVER_ROOT`:
- routes
- handlers
- request/response models
- auth requirements
- error shapes

2. Implement in `CLIENT_ROOT`:
- typed request/response models
- Hyper-based client methods
- auth/header handling per `AUTH_MODE`
- error mapping
- persistence integration per `PERSISTENCE_MODE` (SQLite + `sqlx` when selected)

3. Reuse models:
- If `ALLOW_SHARED_MODELS_FROM_SRC=true`, reuse only pure domain/data types.
- Never import server-only code (routes, handlers, server persistence/infrastructure).
- Apply `SHARED_MODEL_STRATEGY` when reuse requires refactor.

4. Scope filtering:
- If `ENDPOINT_SCOPE != all`, implement only requested endpoint groups and report skipped ones.

5. Extra endpoints:
- If `REPORT_EXTRA_ENDPOINTS=true`, explicitly list additional server endpoints required for complete client workflows.

## Output format
## Endpoint Inventory
- `METHOD PATH` -> backend file refs

## Implemented in Client
- `METHOD PATH`
- client function signature
- files changed

## Gaps / Not Implemented
- endpoint
- reason
- unblock action

## Extra Endpoints Needed
- proposed `METHOD PATH`
- request/response draft
- reason
- priority (`high|medium|low`)

## Validation
- tasks run from `.zed/tasks.json`
- pass/fail

## Constraints
- Do not invent backend behavior.
- If schema/behavior is unclear, stop and report exact ambiguity with file reference.
- Keep client layered boundaries: UI -> application -> infrastructure.
- Keep Hyper and `sqlx` details inside infrastructure/persistence boundaries.
