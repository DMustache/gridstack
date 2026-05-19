# Task Contracts

This document defines the contract for tasks in `.zed/tasks.json`.

## Global Rules

- Each Zed task must invoke a script under `scripts/tasks/`.
- Task scripts should keep stdout/stderr hidden on success.
- On failure, scripts must print the related log file content and exit non-zero.
- Logs are written to `generated/logs/*.log`.
- Tasks that generate artifacts must validate output files with `test -s` (or equivalent).

## Task Inventory

1. `Cargo: Update`
- Script: `scripts/tasks/cargo_update.sh`
- Represents: dependency index/lock refresh.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-update.log`.

2. `Cargo: Check`
- Script: `scripts/tasks/cargo_check.sh`
- Represents: compile validation without building release artifacts.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-check.log`.

3. `Cargo: Format`
- Script: `scripts/tasks/cargo_fmt.sh`
- Represents: rustfmt formatting pass for workspace.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-fmt.log`.

4. `Cargo: Test`
- Script: `scripts/tasks/cargo_test.sh`
- Represents: unit/integration test execution.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-test.log`.

5. `Cargo: Build (Debug)`
- Script: `scripts/tasks/cargo_build_debug.sh`
- Represents: debug build validation.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-build-debug.log`.

6. `Cargo: Build (Release)`
- Script: `scripts/tasks/cargo_build_release.sh`
- Represents: release build validation.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-build-release.log`.

7. `Cargo: Ultra Clippy (Strict)`
- Script: `scripts/tasks/cargo_clippy_ultra.sh`
- Represents: strict lint gate.
- Validation: command exit code.
- Failure log: `generated/logs/cargo-clippy-ultra.log`.

8. `Cargo: Export OpenAPI`
- Script: `scripts/tasks/export_openapi.sh`
- Represents: generate project OpenAPI JSON.
- Validation: command exit code; generated file exists.
- Failure log: `generated/logs/openapi-export.log`.

9. `Redocly: Export OpenAPI to html`
- Script: `scripts/tasks/redocly_from_openapi.sh`
- Represents: build HTML docs from generated OpenAPI JSON.
- Validation: command exit code.
- Failure log: `generated/logs/redocly-export.log`.

10. `Docs: Rebuild OpenAPI + Redocly`
- Script: `scripts/tasks/rebuild_openapi_redocly.sh`
- Represents: end-to-end OpenAPI + Redocly regeneration.
- Validation: command exit code.
- Failure log: `generated/logs/openapi-redocly-rebuild.log`.

11. `Matrix: Sync & Generate OpenAPI`
- Script: `scripts/tasks/sync_matrix_openapi_only.sh`
- Represents: sync matrix-spec repo and generate Matrix OpenAPI JSON only.
- Validation: `generated/openapi/matrix-openapi.json` exists and is non-empty.
- Failure log: `generated/logs/matrix-openapi-only.log`.

12. `Matrix: Sync & Generate OpenAPI + Redocly`
- Script: `scripts/tasks/sync_matrix_openapi.sh`
- Represents: sync matrix-spec repo, generate Matrix OpenAPI JSON, and build Redocly HTML.
- Validation: both matrix JSON and HTML outputs exist and are non-empty.
- Failure log: `generated/logs/matrix-openapi.log`.

13. `DB: Dump Schema Only`
- Script: `scripts/tasks/db_dump_schema.sh`
- Represents: PostgreSQL schema-only dump.
- Validation: `generated/db/schema.sql` exists and is non-empty.
- Failure log: `generated/logs/db-schema-dump.log`.

14. `Tuwunel: Sync Repository`
- Script: `scripts/tasks/sync_tuwunel.sh`
- Represents: clone `tuwunel` into `temp/` if missing, otherwise update local checkout.
- Validation: `temp/tuwunel/.git/HEAD` exists and is non-empty.
- Failure log: `generated/logs/tuwunel-sync.log`.

15. `Client: Cargo Check`
- Script: `scripts/tasks/client_cargo_check.sh`
- Represents: compile validation for standalone `client/` crate.
- Validation: command exit code.
- Failure log: `generated/logs/client-cargo-check.log`.

16. `Client: Run`
- Script: `scripts/tasks/client_run.sh`
- Represents: run desktop client application.
- Validation: command exit code.
- Failure log: `generated/logs/client-run.log`.

17. `Client: Build (Windows Release)`
- Script: `scripts/tasks/client_build_windows_release.sh`
- Represents: cross-compile the standalone `client/` crate for Windows 10/11 (`x86_64-pc-windows-gnu`) in release mode.
- Validation: command exit code; `client/target/x86_64-pc-windows-gnu/release/client.exe` exists and is non-empty.
- Failure log: `generated/logs/client-build-windows-release.log`.
