# READ PARENT AGENTS

Before using this file:

1. Read all parent `AGENTS.md` files up to the repository root.
2. Treat parent instructions as active context.
3. Apply local rules only as scoped extensions or overrides.
4. Prefer the most specific applicable rule when conflicts exist.

# Services Guidelines

This file applies to code under `src/services/`.

## Purpose

`src/services/` contains bounded application service modules.

Each service module should be isolated and should expose behavior through explicit routes, handlers, services, entities, persistence modules, view models, and traits where applicable.

## Discover Local Structure First

Do not assume every service module has the exact same files.

Before editing a service module:

1. Inspect the target service directory.
2. Identify existing `entities`, `handlers`, `routes`, `service`, `persistence` patterns.
3. Follow the local structure already used in that module.

## Typical Service Layout

A service module may contain:

- `entities/` — domain entities.
- `routes.rs` — route registration.
- `handlers.rs` with `handlers/*` — HTTP request/response boundary.
- `service.rs` — business logic.
- `persistence.rs` and `persistence/*` — persistence boundary and database access.
- `errors.rs` — service-local errors where present.

## Layering Rules

Handlers may:

- parse HTTP inputs
- call service methods
- convert service results into HTTP responses
- map service errors into API errors

Handlers must not:

- build database queries
- contain persistence details
- own business rules that belong in `service.rs`

Services may:

- enforce business rules
- coordinate domain operations
- depend on traits/ports
- return typed domain or service errors

Services must not:

- construct HTTP responses
- depend directly on database query syntax
- know raw table-level storage details unless the existing module has not yet been decoupled and the task is intentionally local

Persistence may:

- use Diesel or database-specific APIs
- build queries
- map database rows to domain entities
- implement service-facing traits or repository ports

Persistence must not:

- construct HTTP responses
- own cross-module business policy
- leak database-specific types into business logic unless already established by the local module

## Fault Isolation

Keep service modules isolated.

A failure in one bounded context must not break unrelated contexts.

Avoid panics in request-handling and service paths. Return typed errors and map them at the boundary.

## Ports and Traits

Before adding a new trait or repository abstraction:

1. Check `src/services/traits.rs`.
2. Check the target service module.
3. Check existing persistence modules.
4. Add the trait close to the service boundary that owns the need.

The service defines what it needs. Infrastructure or persistence implements it.

## Matrix API Behavior

The implementation targets Matrix Client-Server API v1.18.

When endpoint behavior is unclear:

1. Check the local service module.
2. Check generated OpenAPI artifacts under `generated/`.
3. Check Matrix generated/reference artifacts under `generated/`.
4. Avoid guessing protocol behavior.

## Matrix Business Logic Source Ladder

When implementing or reviewing Matrix Client-Server API behavior, use this source order:

1. Local generated Matrix OpenAPI:
   - `generated/openapi/matrix-openapi.json`

2. Official Matrix Client-Server API v1.18 specification:
   - `https://spec.matrix.org/v1.18/client-server-api/`

3. Reference implementation research:
   - `matrix-construct/tuwunel`

Rules:

- Start from `generated/openapi/matrix-openapi.json`.
- Use it to identify endpoint path, method, request schema, response schema, auth requirements, and error shapes.
- If endpoint semantics are unclear, enrich with the official Matrix v1.18 Client-Server API specification.
- If business behavior is still unclear, inspect `matrix-construct/tuwunel` as a reference implementation.
- Do not copy Tuwunel code directly.
- Do not let Tuwunel override the official Matrix specification.
- If OpenAPI, spec, and Tuwunel disagree, report the conflict and prefer the official specification unless the user explicitly chooses implementation compatibility.

## Fast Endpoint Path

For faster endpoint delivery with predictable quality:

1. Start with local patterns:
- copy route/handler/service/error structure from nearest endpoint in the same module
- do not create new folders/modules unless existing module already uses that split

2. Implement in this order:
- request/response DTO updates
- handler response enum mapping
- service business flow
- persistence changes (only if required)

3. Keep tests separated by purpose:
- service tests validate business decisions and typed errors
- handler/router tests validate status codes and Matrix `errcode` mapping

4. Minimize churn:
- avoid broad refactors while adding one endpoint
- avoid changing generated artifacts directly
- avoid changing task scripts unless they are objectively broken
