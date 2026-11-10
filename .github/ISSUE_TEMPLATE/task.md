---
name: Agent task
about: Contracted unit of work executable by a worker agent
labels: task, stage:backlog
---

## Goal
<!-- One sentence: what changes for a Matrix client after this task. -->

## Spec
- Operation(s): `METHOD /_matrix/client/v3/...`
- Spec link: https://spec.matrix.org/v1.18/client-server-api/#...
- OpenAPI operationId: `...`

## Contract
<!-- The externally observable behavior. Request/response shapes, status codes, errcodes, side effects. -->
- Request: ...
- Response 200: ...
- Errors: `M_...` -> status ...
- Ports/traits touched: `src/services/<module>/...::TraitName::method`

## Context pack (read only these first)
<!-- Lead lists the minimum files and line ranges. Worker expands only if needed. -->
- `src/services/<module>/routes.rs`
- `src/services/<module>/service.rs:LINE-LINE`
- Nearest existing pattern to copy: `...`

## Gaps / known unknowns
- ...

## Out of scope
- ...

## QA (required tests)
<!-- From docs/process/QA.md, by the layers this task touches. -->
- [ ] Unit: ...
- [ ] Service (fake ports): ...
- [ ] Router (status + errcode): ...
- [ ] Persistence (DB, `#[ignore]` unless DATABASE_URL): ...

## Acceptance criteria
- [ ] ...
- [ ] `scripts/agents/qa_gate.sh` passes

## Dependencies
- Depends on: #...
