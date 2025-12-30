API Implementation Guide for Tee (documentation only)

Purpose
-------
This document explains how to implement the API described in `docs/api-openapi.yaml` within the existing Tee codebase. It intentionally avoids producing code; instead it maps the spec to existing modules, types, and commands so a human or code-generation tool has clear guidance.

1. High-level overview
----------------------
- API base path: `/api/v1`
- Authentication: Bearer JWT for write operations (create/update/delete/start/complete). Read/list may be public depending on policy; the spec marks write endpoints as protected.
- Response format: JSON, `application/json`.
- Error model: `{ error: string, details?: object }` with appropriate HTTP status codes.

2. Domain model mapping
------------------------
Use the existing domain types as the canonical data model:
- `crate::domain::task::Task` -> API `Task` schema
  - `id` -> UUID string
  - `title` -> string (use `TaskTitle::parse` for validation)
  - `description` -> string (use `TaskDescription::parse`)
  - `status` -> `TaskStatus` enum values: `PLANNED`, `IN_PROGRESS`, `COMPLETED`
  - `created_at`, `updated_at`, `due_at` -> ISO 8601 date-time strings
  - `priority` -> i16 (1..=5)
  - `row_version` -> i64 (optimistic concurrency)

3. Suggested DTOs (no code, shape only)
--------------------------------------
- NewTask: `{ title: string, description?: string, due_at?: string, priority?: integer }`
- UpdateTask: `{ title?: string, description?: string, status?: string, due_at?: string, priority?: integer }`
- TaskResponse: same as `Task` schema
- ErrorResponse: `{ error: string, details?: object }`

4. Router wiring (where to mount)
---------------------------------
Current web routes are in `src/interface/routes_web.rs` and are merged in `src/interface/http.rs` through `routes_web::router(...)`.

- Create a separate router function for API routes, e.g. `src/interface/routes_api.rs::router(pool, auth_settings, translator)` that returns an `axum::Router` with `.with_state(AppState { ... })` or a tailored `ApiState` if preferred.
- Merge the API router in `src/interface/http.rs::build_router` alongside `routes_web::router`, for example with `.merge(routes_api::router(...))` and prefer prefixing with `/api/v1`.

Note: Keep web and API routers separated to avoid mixing response types. Use `Router::nest` or `.route("/api/v1/...", ...)` as appropriate.

5. Handler responsibilities
---------------------------
Handlers should:
- Validate input using domain parsing helpers (e.g., `TaskTitle::parse`). Return `400` with `ErrorResponse` on validation errors.
- Use app-level commands and queries for business logic:
  - `crate::app::commands::create_task::handle(...)` — create task
  - `crate::app::queries::get_task::handle(...)` — fetch single task
  - `crate::app::queries::list_tasks::handle(...)` — list tasks
  - `crate::app::commands::start_task::handle(...)` — start
  - `crate::app::commands::complete_task::handle(...)` — complete
  - `crate::app::commands::update_task_details::handle(...)` — update
  - `crate::app::commands::delete_task::handle(...)` — delete
- Translate domain errors into HTTP status codes:
  - Not found -> `404`
  - Validation or business rule violation -> `400`
  - Unauthorized -> `401` or `403` depending on auth layer
  - Unexpected failures -> `500`
- Return JSON responses (`axum::Json`), and set status codes accordingly (e.g., `201` for created, `204` for delete no-content).

6. Authentication
-----------------
- Use existing auth utilities in `src/interface/auth.rs`. The web side uses session cookies; for the API we recommend accepting a bearer token and translating it to the same `AuthContext` or principal type used by the web handlers.
- If a new JWT validation helper is required, implement a thin adapter that reads `Authorization: Bearer <token>`, validates signature/claims, and produces a principal object identical to the web login flow.

7. Validation and business rules
--------------------------------
- Reuse `TaskTitle::parse`, `TaskDescription::parse`, and `TaskPriority::parse` for input validation.
- For state transitions, use `can_transition(from, to)` and `can_transition_task(is_deleted, from, to)`.
- Enforce immutability for deleted tasks.

8. Concurrency / row version
----------------------------
- Use `row_version` for optimistic concurrency on updates. Handlers should return a conflict (`409`) if `row_version` mismatches (specify this in API responses' `details`).

9. Pagination, filtering, sorting
--------------------------------
- Implement query parameters `page`, `page_size`, `status`, `search`, `priority`, `sort` as specified in `api-openapi.yaml`.
- For large result sets, return `{ total, items }`.

10. Tests and examples (documentation-only)
------------------------------------------
- Provide integration test scenarios:
  - Create -> Read -> Update -> Start -> Complete -> Delete
  - Validation failures (empty title, invalid priority)
  - Unauthorized access attempts to protected endpoints
- Example `curl` commands (replace host and token):

```sh
# List tasks
curl -H "Authorization: Bearer <token>" "http://localhost:8000/api/v1/tasks"

# Create task
curl -X POST -H "Authorization: Bearer <token>" -H "Content-Type: application/json" \
  -d '{"title":"New","description":"desc","priority":3}' \
  http://localhost:8000/api/v1/tasks
```

11. OpenAPI and code generation guidance
----------------------------------------
- The `docs/api-openapi.yaml` file is intentionally complete for task endpoints. Feed it to OpenAPI codegen tools to generate DTOs, client stubs, or server skeletons.
- When code-generating, prefer: generate DTOs only, then hand-wire handlers to call existing `app::commands` and `app::queries` to avoid duplicating business logic.

12. Error mapping examples
--------------------------
- Validation error example (400):
  - Body: `{ "error": "validation_failed", "details": { "title": "must not be empty" } }`
- Not found (404): `{ "error": "Task not found" }`
- Unauthorized (401): `{ "error": "unauthorized" }`

13. Non-functional considerations
--------------------------------
- Use HTTPS in production and require secure tokens.
- Add logging and observability (request IDs already configured in `src/interface/http.rs`).
- Consider rate-limiting sensitive endpoints.

14. Next steps (human or codegen)
--------------------------------
- If running a code-generation tool:
  - Feed `docs/api-openapi.yaml` to the generator to obtain DTOs and server stubs.
  - Manually implement handler bodies to call `crate::app::...` functions and use domain parsers for validation.
- If implementing by hand, follow the routing and handler guidance above and add tests mirroring the integration scenarios.

