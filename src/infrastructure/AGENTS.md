# READ PARENT AGENTS

Before using this file:

1. Read all parent `AGENTS.md` files up to the repository root.
2. Treat parent instructions as active context.
3. Apply local rules only as scoped extensions or overrides.
4. Prefer the most specific applicable rule when conflicts exist.

# Infrastructure Guidelines

This file applies to code under `src/infrastructure/`.

## Purpose

`src/infrastructure/` contains infrastructure implementations, adapters, typed primitives, configuration support, and integration details.

Do not rely on this document as an inventory of available infrastructure.

Always inspect the files in this directory to discover the current infrastructure types and implementations.

## Adapter Boundary

Infrastructure may depend on external crates, system details, configuration formats, cryptographic libraries, database schemas, clocks, generators, and other implementation-specific mechanisms.

Service and business logic should depend on small traits or ports where practical.

Infrastructure implements those ports.

## Discovery Workflow

Before changing infrastructure code:

1. Inspect this directory.
2. Identify the local naming and error-handling pattern.
3. Check whether a service-facing trait already exists in `src/services/traits.rs` or nearby service code.
4. Implement infrastructure behind the existing boundary where possible.

## Typed Values

Prefer typed wrappers for important domain concepts when they already exist in the project.

Do not pass raw strings through multiple layers when a typed value is available.

Do not invent a new typed wrapper without checking the existing files in this directory.

## Configuration

Keep environment-specific configuration parsing and infrastructure setup here or in the existing configuration boundary.

Do not read configuration directly from handlers.

## Schema and Database Integration

Database schema integration belongs to infrastructure or persistence boundaries.

Do not hand-edit generated schema artifacts.

If schema context is needed, prefer generated artifacts under `generated/db/`.

If schema evolution is required, use the project migration workflow and task registry.
