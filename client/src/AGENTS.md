# READ PARENT AGENTS

Before using this file:

1. Read all parent `AGENTS.md` files up to the repository root.
2. Treat parent instructions as active context.
3. Apply local rules only as scoped extensions or overrides.
4. Prefer the most specific applicable rule when conflicts exist.

# Client Source Guidelines

This file applies to files under `client/src/`.

## Scope

`client/src/` is the client application source entry layer.

Use this area for:

- bootstrap wiring
- application module composition
- high-level command/event registration

Do not place low-level infrastructure implementation details directly in bootstrap files.

## Module Organization

As the client grows, prefer explicit modules:

- `application/` for use cases and orchestration
- `domain/` for typed business concepts
- `infrastructure/` for Hyper/SQLite adapters
- `ui/` for Tauri command handlers and presentation coordination

Introduce modules only when first needed, but keep the layering intent.

## Error and Result Handling

- Do not use `unwrap`/`expect` on runtime paths in production logic.
- Use typed error enums per boundary and convert at the edge.
- Keep user-facing errors stable and implementation-neutral.

## Async and Concurrency

- Prefer async, non-blocking paths for IO.
- Avoid blocking file/database/network operations on UI event paths.
- Centralize runtime and shared client state initialization.

