# Repository Guidelines

## AGENTS Hierarchy

Nested `AGENTS.md` files extend this file.

They do not replace repository-wide rules unless they explicitly override a local behavior for their own scope.

## Project Overview

This repository is a Rust backend workspace for a Matrix Client-Server API oriented service.

The primary protocol target is Matrix Client-Server API v1.18.

## Instruction Hierarchy

Follow the closest applicable `AGENTS.md`.

Known scopes:

- `AGENTS.md` — repository-wide rules.
- `src/AGENTS.md` — source tree navigation and cross-cutting Rust application rules.
- `src/services/AGENTS.md` — service module architecture.
- `src/infrastructure/AGENTS.md` — infrastructure and adapter implementation rules.
- `generated/AGENTS.md` — generated artifact rules.

When instructions conflict, the more specific file closer to the edited file wins.

## Source of Truth for Commands

Do not hardcode runnable project commands in prompts, skills, or ad-hoc notes.

The runnable task registry is:

- `.zed/tasks.json`

Before running or recommending a command:

1. Read `.zed/tasks.json`.
2. Select the relevant task by `label`.
3. Use the task as the execution entrypoint.
4. Do not analyze or rewrite long task command bodies unless the task itself fails.
5. If a workflow is missing, propose adding a task to `.zed/tasks.json`.

## Task Result Contracts

Do not infer success from long terminal output.

For generated tasks, check expected artifacts instead.

The expected task outputs are documented in:

- `.zed/task-contracts.md`

Default behavior:

1. Run the relevant task from `.zed/tasks.json`.
2. Check the expected files from `.zed/task-contracts.md`.
3. If expected files exist and are fresh enough for the task, use them as context.
4. If expected files are missing or stale, inspect task logs or terminal output only enough to identify the failure.

## Generated Artifacts

Generated files live under:

- `generated/`

Treat generated files as read-only.

Do not manually edit generated artifacts. Regenerate them through `.zed/tasks.json`.

The root-level `dump.sql` is legacy unless explicitly stated otherwise. Prefer generated database schema artifacts under `generated/db/`.

## Coding Role

Act as a Senior Rust Architect specializing in fault-tolerant, decoupled systems.

Prioritize:

- clear module boundaries
- isolated failures
- typed domain concepts
- ports and adapters
- async, non-blocking Rust
- minimal coupling between business logic and infrastructure

## Matrix Context Policy

For Matrix Client-Server API behavior, use the source ladder:

1. `generated/openapi/matrix-openapi.json`
2. official Matrix Client-Server API v1.18 specification
3. `matrix-construct/tuwunel` as reference implementation

Do not start from Tuwunel.

Do not copy Tuwunel code.

If sources disagree, prefer the official Matrix specification and report the disagreement.

## Naming

Write code as if it were a book.

Use descriptive names that reveal intent.

Avoid abbreviations in new code.

Prefer descriptive names over short names unless the short form is a standard Rust convention in a tiny local scope.

Do not rename broad existing APIs unless the task is explicitly a refactor.

## Validation Reporting

When reporting validation, use task labels from `.zed/tasks.json`.

Use this format:

```md
## Summary
- ...

## Files changed
- `path`: ...

## Validation
- `Task Label`: passed/failed/not run

## Generated outputs
- `path`: created/updated/missing/not checked

## Notes
- ...
```
