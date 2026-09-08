# Gridstack

Gridstack is an experimental Matrix homeserver and desktop client written in Rust. The backend targets the Matrix Client-Server API v1.18 and is built around explicit service boundaries, typed domain models, and PostgreSQL persistence.

> **Project status:** active development. Matrix compatibility is incomplete, APIs may change, and the project is not ready for production use.

## Highlights

- Async HTTP server built with Axum and Tokio
- PostgreSQL persistence through Diesel
- Matrix-style authentication, rooms, synchronization, and identity services
- Access and refresh tokens with Argon2 password hashing and JWT authentication
- Appservice authentication and namespace ownership support
- OpenAPI export and Redocly documentation workflows
- Separate Rust desktop client built with eframe
- SQLite-backed client session persistence
- Local task registry for repeatable build, test, lint, and documentation workflows

## Repository layout

| Path | Purpose |
| --- | --- |
| `src/` | Homeserver application, services, and infrastructure |
| `client/` | Native Rust desktop client |
| `migrations/` | Diesel database migrations |
| `response_derive/` | Local response-related procedural macro crate |
| `scripts/tasks/` | Scripts used by the project task registry |
| `.zed/tasks.json` | Canonical runnable task definitions |
| `configuration.toml` | Local server, database, authentication, and identity configuration |

## Requirements

- Rust 1.95 or newer
- PostgreSQL
- Diesel CLI with PostgreSQL support
- A C toolchain suitable for the target platform
- Optional: Zed, for running the checked-in task definitions directly

The desktop client has additional native dependencies required by `eframe`. Cross-compiling its Windows release also requires the MinGW-w64 toolchain configured in `client/Cargo.toml`.

## Getting started

1. Clone the repository and switch to its default `develop` branch.
2. Create a PostgreSQL database for Gridstack.
3. Copy or edit `configuration.toml` for your environment.
4. Replace every development secret and identity key in the configuration before using a shared or deployed environment.
5. Apply the migrations from `migrations/` with Diesel.
6. Use the tasks in `.zed/tasks.json` to check, test, and build the project.

The default development configuration binds the server to `127.0.0.1:3000` and expects a PostgreSQL database named `gridstack`.

A dedicated server-run task and database-migration task are not yet registered in `.zed/tasks.json`. Until those workflows are added, consult the Rust and Diesel tooling appropriate to your local environment.

## Development tasks

The task registry is the source of truth for project commands. Useful task labels include:

- `Cargo: Check`
- `Cargo: Test`
- `Cargo: Format`
- `Cargo: Build (Debug)`
- `Cargo: Build (Release)`
- `Cargo: Ultra Clippy (Strict)`
- `Docs: Rebuild OpenAPI + Redocly`
- `Client: Cargo Check`
- `Client: Run`
- `Client: Build (Windows Release)`

When a task produces generated artifacts, its expected outputs are documented in `.zed/task-contracts.md`.

## Architecture

The homeserver is divided into Matrix-oriented service modules:

- **Authorization** — registration, login, token refresh, logout, and appservice access
- **Rooms** — room creation, membership, state, messages, receipts, and read markers
- **Synchronization** — client synchronization data and persistence
- **Identity** — association lookup, invitations, and signing-key management

Cross-cutting HTTP, configuration, persistence, and protocol concerns live under `src/infrastructure/`. Generated artifacts, when present, are treated as read-only and rebuilt through registered tasks.

The desktop client follows a ports-and-adapters structure with application, domain, HTTP infrastructure, and SQLite persistence layers.

## Configuration and security

`configuration.toml` contains development defaults. Do not reuse its database credentials, JWT secret, password pepper, or identity key material in production.

For a real deployment:

- load secrets from a secure deployment mechanism;
- use a strong, unique JWT secret and password pepper;
- generate stable identity keys and rotate them intentionally;
- restrict the database account to the permissions Gridstack needs;
- place the service behind TLS;
- review registration and appservice namespace settings.

## API documentation

Gridstack can export its implemented API as OpenAPI and render it with Redocly. Use the registered `Cargo: Export OpenAPI`, `Redocly: Export OpenAPI to html`, or `Docs: Rebuild OpenAPI + Redocly` tasks.

For Matrix protocol behavior, the project targets the official Matrix Client-Server API v1.18.

## Contributing

Before submitting changes:

1. Follow the nearest applicable `AGENTS.md`.
2. Keep module boundaries explicit and avoid coupling domain logic to infrastructure.
3. Run the relevant format, check, test, and strict Clippy tasks.
4. Regenerate derived artifacts through the task registry instead of editing them manually.
5. Document incomplete protocol behavior and compatibility limitations clearly.
