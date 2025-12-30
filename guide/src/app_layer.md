# Application Layer: Commands and Queries

The application layer is where use cases live. It tells the system what to do
and in what order. It does not know about HTTP, templates, or browser forms.

## Two folders

- `src/app/commands/` contains state-changing operations.
- `src/app/queries/` contains read-only operations.

This is the command/query split described in the Tee guidelines.

## Commands

A command:
- Runs in a transaction.
- Checks authorization through domain policy.
- Validates rules (like status transitions).
- Writes to the database.

Example: `StartTask` in `src/app/commands/start_task.rs`:

```rust
let mut tx = pool.begin().await?;
let row = shared::fetch_task_required(&mut *tx, cmd.id, StartTaskError::NotFound).await?;
let current = shared::parse_status(&row.status, StartTaskError::InvalidStatus)?;
if !can_transition_task(row.is_deleted, current, TaskStatus::InProgress) {
    return Err(StartTaskError::InvalidTransition);
}
```

### Why transactions

A transaction ensures all steps succeed or all fail. Without it, you could
read a task, update it, and fail midway, leaving data in a confusing state.

## Queries

A query:
- Is read-only.
- Does not start a transaction unless needed for consistency.
- Does not perform side effects.

Example: `ListTasks` in `src/app/queries/list_tasks.rs`.

## Error mapping

Commands and queries return custom errors. To avoid duplicated mapping logic,
errors implement the `AppErrorSource` trait in `src/app/error.rs`.

This lets the interface layer translate errors into HTTP responses consistently.

Why enums for errors?
- They make each failure case explicit.
- They are easy to match on.
- They encourage deterministic responses (no hidden surprises).

### Code example: error interface

From `src/app/error.rs`:

```rust
pub enum AppErrorKind {
    Forbidden,
    NotFound,
    Conflict,
    BadRequest,
    Internal,
    Db,
}

pub trait AppErrorSource {
    fn error_kind(&self) -> AppErrorKind;
    fn user_message(&self) -> String;
    fn into_db_error(self) -> Option<sqlx::Error>
    where
        Self: Sized,
    {
        None
    }
}
```

## Shared helpers

`src/app/commands/shared.rs` holds small helper functions to reduce repetition:
- Authorization checks.
- Fetching tasks with a consistent "not found" error.
- Conflict classification when `row_version` mismatches.

These helpers keep command handlers short and readable.

### Code example: conflict classification helper

From `src/app/commands/shared.rs`:

```rust
pub async fn classify_update_conflict<'a, E>(
    executor: E,
    id: Uuid,
) -> Result<UpdateConflict, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let current_row = task_repo::fetch_task(executor, id).await?;
    Ok(match current_row {
        None => UpdateConflict::NotFound,
        Some(row) if row.is_deleted => UpdateConflict::Deleted,
        Some(_) => UpdateConflict::Conflict,
    })
}
```

## Exercise

- Pick one command (create, start, or delete) and add a log line when it
  succeeds. Note where logs belong and where they do not.

Next: Domain Layer.
