# Data Layer and SQL

The data layer is the only place where SQL appears. This keeps database logic
explicit and easy to review.

## Key files

- `src/data/task_repo.rs` - SQL for tasks.
- `src/data/auth_repo.rs` - SQL for users and sessions.
- `src/data/db.rs` - connection pool setup.
- `migrations/` - schema changes.

## Task repository

`src/data/task_repo.rs` defines functions like:
- `insert_task`
- `fetch_task`
- `list_tasks`
- `update_task_status`
- `update_task_details`
- `soft_delete_task`

Each function uses parameterized SQL. This avoids SQL injection and keeps
query shapes explicit.

### Code example: row mapping

From `src/data/task_repo.rs`:

```rust
pub struct TaskRow {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub priority: i16,
    pub row_version: i64,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

## Soft delete

Instead of deleting rows, tasks are marked with:
- `is_deleted = true`
- `deleted_at = now()`

Queries filter out deleted tasks by default. This matches SPEC-02.

## Optimistic concurrency

Updates check `row_version`:
- The command provides an expected version.
- The SQL update only succeeds if it matches.
- Otherwise the command returns a conflict.

This avoids heavy database locking while still preventing lost updates.

### Code example: concurrency guard in SQL

From `src/data/task_repo.rs`:

```rust
let result = sqlx::query!(
    r#"
    UPDATE tasks
    SET status = $2,
        updated_at = now(),
        row_version = row_version + 1
    WHERE id = $1
      AND row_version = $3
      AND is_deleted = FALSE
    "#,
    id,
    status.as_str(),
    expected_row_version
)
.execute(executor)
.await?;
```

## Migrations

Migrations live in `migrations/` and are required for schema changes.
Never change the database manually in production.

Example migrations:
- `0001_create_tasks.sql`
- `0002_task_lifecycle.sql`
- `0003_task_extensions.sql`

### Code example: constraints from a migration

From `migrations/0003_task_extensions.sql`:

```sql
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'tasks_deleted_consistency'
    ) THEN
        ALTER TABLE tasks
            ADD CONSTRAINT tasks_deleted_consistency
            CHECK (
                (is_deleted = FALSE AND deleted_at IS NULL)
                OR
                (is_deleted = TRUE AND deleted_at IS NOT NULL)
            );
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'tasks_priority_range'
    ) THEN
        ALTER TABLE tasks
            ADD CONSTRAINT tasks_priority_range
            CHECK (priority BETWEEN 1 AND 5);
    END IF;
END $$;
```

## SQLx compile-time checking

This project uses SQLx with prepare metadata. The idea is:
- SQL is checked against the database schema.
- Types are verified at compile time.

The workflow is described in `docs/Development-Guide.md`.

You will see SQLx macros like `query!` and `query_as!`. These macros:
- Capture the SQL in code.
- Check column names and types.
- Generate Rust structs that match the result.

## Exercise

- Add a new column `archived_at` and write a migration for it.
- Add a new repo function that lists only archived tasks.

Next: Authentication and Sessions.
