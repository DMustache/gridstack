Implement `{METHOD} {PATH}` in this repository.

Requirements:
1. Follow AGENTS hierarchy strictly:
- repository root `AGENTS.md`
- nearest scoped AGENTS files under `src/` and `src/services/`
2. Use Matrix behavior source ladder:
- `generated/openapi/matrix-openapi.json` first
- Matrix Client-Server spec v1.18 second
- `matrix-construct/tuwunel` only if still ambiguous, and do not copy code
3. Discover the existing local service structure before editing (routes/handlers/service/persistence/entities/errors) and match that pattern.
4. Keep layering clean:
- handlers: HTTP parsing/response mapping only
- service: business logic and typed errors
- persistence: DB access only
5. Do not edit `generated/` manually.
6. Before running validation commands, read `.zed/tasks.json` and use task labels only; evaluate outputs using `.zed/task-contracts.md`.

Implementation expectations:
1. Add route wiring for `{METHOD} {PATH}`.
2. Implement request parsing and validation according to Matrix v1.18.
3. Implement registration business flow (including any required auth/session/token handling per spec and existing project architecture).
4. Map errors to Matrix-compatible error responses.
5. Add/adjust tests for success and important failure paths.

Deliverables:
1. Code changes in appropriate `{SERVICE_MODULE}` modules.
2. Brief notes on any spec ambiguity and which source resolved it.
3. Final report in this format:

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
