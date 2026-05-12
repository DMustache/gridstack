# Matrix Endpoint Research Prompt

For any Matrix endpoint implementation, use this order:

1. Read `generated/openapi/matrix-openapi.json`.
2. Find the endpoint by method and path.
3. Extract request schema, response schema, auth rules, and documented errors.
4. Read the official Matrix Client-Server API v1.18 page for endpoint semantics.
5. If behavior remains unclear, inspect `matrix-construct/tuwunel`.
6. Treat Tuwunel as reference implementation, not specification.
7. Report conflicts instead of silently choosing.

Output:

## Endpoint
## OpenAPI facts
## Matrix spec behavior
## Tuwunel reference
## Implementation decision
