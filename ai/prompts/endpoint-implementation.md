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
7. Optimize for delivery speed with correctness:
- reuse existing DTOs/errors/services where possible
- avoid introducing new abstractions unless reused by at least 2 call sites
- keep first pass minimal, then extend only for missing spec cases
- prefer incremental compile cycles (`Cargo: Check` is complete)

Implementation expectations:
1. Add route wiring for `{METHOD} {PATH}`.
2. Implement request parsing and validation according to Matrix v1.18.
3. Implement business flow for `{METHOD} {PATH}` (including any required auth/session/token handling per spec and existing architecture).
4. Map errors to Matrix-compatible error responses.
5. Keep OpenAPI/export compatibility:
- avoid tuple error variants in mapped handler enums when `response_derive` requires unit-style variant matching
- keep enum-to-matrix mapping explicit in handler response enums

Deliverables:
1. Code changes in appropriate `{SERVICE_MODULE}` modules.
2. Brief notes on any spec ambiguity and which source resolved it.
3. Explicitly list whether implementation is exact match vs intentional subset of Matrix/OpenAPI behavior.
4. Final report in this format:

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
