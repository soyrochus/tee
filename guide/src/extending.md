# Extending the System

This chapter shows safe ways to extend the Task example while staying inside
Tee system constraints.

## Add a new field to tasks

Steps:
1. Add a migration in `migrations/`.
2. Update SQL in `src/data/task_repo.rs`.
3. Update domain types if needed.
4. Update commands and queries.
5. Update templates and routes.

Always update SQLx metadata after SQL changes.

## Add a new command

Example: "Archive Task".
- Create a new command file under `src/app/commands/`.
- Add a route in `src/interface/routes_web.rs`.
- Add a button in the template.
- Add SQL in `src/data/task_repo.rs`.

### Code example: register a command module

From `src/app/commands/mod.rs`:

```rust
pub mod complete_task;
pub mod create_task;
pub mod delete_task;
pub(crate) mod shared;
pub mod start_task;
pub mod update_task_details;
```

## Add a REST API

The Tee system allows a separate API router. Start with:
- `docs/API-Implementation-Guide.md`
- `docs/Add-Rest-API.md`

Key rule: keep API routes separate from HTML routes.

## Add a background job

If you need background work, use the same binary and database.
Do not introduce a new always-on service unless you have approval.

### Code example: add a route

From `src/interface/routes_web.rs`:

```rust
Router::new()
    .route("/tasks", get(tasks_list).post(tasks_create))
    .route("/tasks/new", get(task_new))
    .route("/tasks/:id", get(task_detail))
    .route("/tasks/:id/start", axum::routing::post(task_start))
    .route("/tasks/:id/complete", axum::routing::post(task_complete))
    .route(
        "/tasks/:id/update",
        axum::routing::post(task_update_details),
    )
    .route("/tasks/:id/delete", axum::routing::post(task_delete))
    .with_state(state)
```

## Exercise

- Draft a plan for adding "labels" to tasks.
- Identify which layer each change belongs to.

You have reached the end of the guide.
