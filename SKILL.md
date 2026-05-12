---
name: gridstack-rust-backend
description: Use this skill when working on this Rust Matrix Client-Server backend, its services, infrastructure, generated OpenAPI/schema artifacts, or Zed task workflows.
---

# Gridstack Rust Backend Skill

This skill does not duplicate repository rules.

Before acting, read the applicable project instructions:

- `AGENTS.md`
- nearest nested `AGENTS.md`
- `.zed/tasks.json`
- `.zed/task-contracts.md`
- `generated/AGENTS.md` when using generated artifacts

Rules:

- Treat `AGENTS.md` files as the source of truth.
- Treat `.zed/tasks.json` as the source of truth for executable tasks.
- Treat `.zed/task-contracts.md` as the source of truth for expected task outputs.
- Treat `generated/**` as read-only context.
- Do not copy commands or architecture rules into this skill.

## Matrix endpoint workflow

When a task concerns Matrix endpoint behavior:

1. Follow nearest `AGENTS.md`.
2. Read `generated/openapi/matrix-openapi.json`.
3. Use the official Matrix Client-Server API v1.18 specification for semantics.
4. Use `matrix-construct/tuwunel` only when more implementation detail is needed.
5. Use `.zed/tasks.json` for generation/validation tasks.
6. Use `.zed/task-contracts.md` for expected outputs.
