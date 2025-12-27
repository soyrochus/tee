# Domain Layer

The domain layer defines the rules of the task system. It does not know
about HTTP or SQL. It only knows business concepts.

## Key files

- `src/domain/task.rs` - `Task`, `TaskStatus`, transition rules.
- `src/domain/types.rs` - validation for title, description, priority.
- `src/domain/policy.rs` - authorization hooks.

## Task status and transitions

`TaskStatus` is an enum with three values:
- `PLANNED`
- `IN_PROGRESS`
- `COMPLETED`

Transitions are controlled by a pure function:
```text
PLANNED -> IN_PROGRESS (allowed)
IN_PROGRESS -> COMPLETED (allowed)
Other transitions (not allowed)
```

This logic is implemented in `src/domain/task.rs` and used by commands.

### Code example: status enum and transitions

From `src/domain/task.rs`:

```rust
pub enum TaskStatus {
    Planned,
    InProgress,
    Completed,
}

pub fn can_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Planned, TaskStatus::InProgress)
            | (TaskStatus::InProgress, TaskStatus::Completed)
    )
}
```

## Value types and validation

`src/domain/types.rs` defines types like `TaskTitle` and `TaskPriority`.
Each type has a `parse` function that checks rules and returns an error
if the input is invalid.

Why this matters:
- Validation is consistent everywhere.
- Rules are easy to test without a database.

### Code example: title validation

From `src/domain/types.rs`:

```rust
pub fn parse(raw: &str) -> Result<Self, TaskTitleError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(TaskTitleError::Empty);
    }
    if s.chars().count() > 200 {
        return Err(TaskTitleError::TooLong);
    }
    Ok(Self(s.to_string()))
}
```

## Policy hooks

`src/domain/policy.rs` defines functions like `can_create_task`.
In the reference implementation they are permissive, but they provide
an explicit seam for real authorization.

### Code example: policy seam

From `src/domain/policy.rs`:

```rust
pub fn can_delete_task(_principal: &Principal) -> bool {
    true
}

pub fn can_view_tasks(_principal: &Principal) -> bool {
    true
}
```

## Exercise

- Add a new validation rule for title length and update the error message.
- Add a new policy rule that only allows deletes for admins (fake a role
  check for now).

Next: Data Layer and SQL.
