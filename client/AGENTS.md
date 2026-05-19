# READ PARENT AGENTS

Before using this file:

1. Read all parent `AGENTS.md` files up to the repository root.
2. Treat parent instructions as active context.
3. Apply local rules only as scoped extensions or overrides.
4. Prefer the most specific applicable rule when conflicts exist.

# Client Guidelines

This file applies to all files under `client/`.

## Purpose

`client/` is the Gridtack desktop client workspace.

Primary stack target:

- Tauri desktop shell
- Rust application code
- HTTP integration using Hyper client
- Local persistence with SQLite and `sqlx`
- UI direction informed by `reference/` (structure and UX intent, not direct copy)

## Architecture Direction

Treat the client as layered boundaries:

1. UI layer
- Tauri commands/events and view orchestration.
- No raw SQL and no direct HTTP construction in UI handlers.

2. Application layer
- Use cases and workflow orchestration.
- Depends on ports/traits for outbound concerns.

3. Infrastructure layer
- Hyper HTTP adapter, SQLite/`sqlx` repository adapters, filesystem/config adapters.
- Owns protocol clients and storage implementation details.

## Boundary Rules

- UI code may call application services only.
- Application services may depend on traits/ports only.
- Infrastructure implements those ports.
- Avoid leaking Hyper or `sqlx` concrete types into UI-facing APIs.
- Return typed errors across boundaries and map them at the Tauri boundary.

## Reference Usage Policy

`reference/` is a design and behavior reference for future direction.

Allowed:

- Reuse ideas for layout, interaction model, and feature decomposition.
- Recreate behavior with project-native code and naming.

Not allowed:

- Blind file-level copy of reference code into production client modules.
- Introducing dependencies only because they exist in `reference/`.

## Data and Persistence Rules

- SQLite is the client-local source of persisted state.
- Use `sqlx` query APIs and typed mappings at the persistence boundary.
- Keep migrations and schema ownership explicit; do not embed ad-hoc schema mutation in runtime code.
- Prefer idempotent initialization paths for first-run client setup.

## HTTP Integration Rules

- Hyper client integration belongs to infrastructure adapters.
- Centralize request construction, timeout policy, retry policy, and response decoding.
- Keep Matrix protocol semantics in typed request/response models in application-facing boundaries.

## Task Workflow

Use `.zed/tasks.json` as the executable task registry.

If client-specific workflows are needed (for example `Client: Tauri Dev`, `Client: Check`), propose adding dedicated tasks to `.zed/tasks.json` instead of running ad-hoc long commands.

## Generated and Build Artifacts

- Do not commit transient build output from Tauri/web bundlers.
- Keep generated artifacts out of hand-edited source unless explicitly designated as generated.

