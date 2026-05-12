# READ PARENT AGENTS

Before using this file:

1. Read all parent `AGENTS.md` files up to the repository root.
2. Treat parent instructions as active context.
3. Apply local rules only as scoped extensions or overrides.
4. Prefer the most specific applicable rule when conflicts exist.

# Source Tree Guidelines

This file applies to code under `src/`.

## Purpose

`src/` contains the Rust application source tree.

Use this file as a navigation guide. More specific architectural rules live closer to the relevant code.

## Important Files

- `main.rs` — application entrypoint.
- `services/` — bounded application service modules.
- `infrastructure/` — infrastructure implementations and adapters.
- `infrastructure/AGENTS.md` — infrastructure-specific rules.

## Navigation Rules

Before changing code:

1. Identify whether the change belongs to application wiring, a service module, or infrastructure.
2. Read the nearest `AGENTS.md`.
3. Inspect nearby source files to discover the current pattern.
4. Prefer local consistency over introducing a new pattern.

## Traits and Implementations

Shared traits used by services are expected to be discoverable from `services/traits.rs` or nearby service modules.

Infrastructure implementations are expected to be discoverable under `infrastructure/` or service-specific `persistence/` modules.

Do not assume a trait or implementation exists based on this document. Inspect the source tree.

## Cross-Cutting Rule

Do not place service-specific rules here.

If a rule only applies to service modules, put it in `src/services/AGENTS.md`.

If a rule only applies to infrastructure, put it in `src/infrastructure/AGENTS.md`.
