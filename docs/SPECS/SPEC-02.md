# SPEC-02  
## Task Lifecycle Extension: Soft Delete, Scheduling, and Concurrency  
**Tee System Reference Use Case**

**Status:** Draft  
**Version:** 1.0  
**Depends on:**  
- Tee System Architecture Guidelines (v1.0) in docs/Tee-Architecture-Guidelines.md 
- SPEC-TASK-01 — Task Registration & Lifecycle Management  in docs/SPECS/SPEC-01.md

---

## 1. Purpose

This specification extends **SPEC-TASK-01** to evolve the Task Registration application into a **more realistic and operationally credible reference system**, while preserving the core constraints of the **Tee System**.

The extension introduces three carefully bounded enhancements:

1. **Soft delete** (logical deletion, no data loss)
2. **Scheduling metadata** (due date and priority)
3. **Optimistic concurrency control** (row versioning)

These additions significantly improve realism, correctness, and UX relevance **without introducing new infrastructure, architectural layers, or conceptual complexity**.

---

## 2. Architectural Context

This SPEC fully adheres to the **Tee System Architecture Guidelines (v1.0)**. In particular:

- Two effective runtime layers only:
  - Rust Application Service (“Tee Service”)
  - PostgreSQL Database (“Tee Store”)
- Logical Command / Query separation (no full CQRS)
- SQL as a first-class interface
- Postgres enforces data integrity, not workflows
- UI remains server-rendered HTML with optional progressive enhancement
- No additional always-on infrastructure

---

## 3. Scope of Extension

### In Scope

- Logical deletion of tasks
- Scheduling-related metadata
- Concurrency safety for updates
- Query adaptations for filtering and sorting

### Explicitly Out of Scope

- Physical deletion
- Restore/undelete functionality
- User management, ownership, or assignment
- Notifications, reminders, or background schedulers
- Event sourcing or distributed messaging

---

## 4. Data Model Extensions (Tee Store)

### 4.1 Extended `tasks` Table

The following columns are added to the existing `tasks` table:

| Column        | Type         | Required | Description |
|---------------|--------------|----------|-------------|
| `is_deleted`  | BOOLEAN      | Yes      | Logical deletion flag |
| `deleted_at`  | TIMESTAMPTZ  | No       | Timestamp of deletion |
| `due_at`      | TIMESTAMPTZ  | No       | Optional due date |
| `priority`    | SMALLINT     | Yes      | Task priority (1–5) |
| `row_version` | BIGINT       | Yes      | Optimistic concurrency counter |

### 4.2 Defaults

- `is_deleted` → `FALSE`
- `priority` → `3` (neutral priority)
- `row_version` → `1`

---

### 4.3 Integrity Constraints

The following constraints are **mandatory**:

```sql
CHECK (
  (is_deleted = FALSE AND deleted_at IS NULL)
  OR
  (is_deleted = TRUE AND deleted_at IS NOT NULL)
);

CHECK (priority BETWEEN 1 AND 5);

CHECK (updated_at >= created_at);
````

Optimistic concurrency is enforced at the **application + SQL statement level**, not via constraints.

---

### 4.4 Indexing Strategy

To maintain performance under soft delete filtering:

```sql
CREATE INDEX idx_tasks_active_updated_at
ON tasks (updated_at DESC)
WHERE is_deleted = FALSE;

CREATE INDEX idx_tasks_active_status
ON tasks (status)
WHERE is_deleted = FALSE;

CREATE INDEX idx_tasks_due_at
ON tasks (due_at)
WHERE is_deleted = FALSE;
```

---

## 5. Domain Model Extensions

### 5.1 New Domain Concepts

* **Due Date**

  * Optional
  * Represents a scheduling intent, not an SLA
* **Priority**

  * Integer range 1 (highest) to 5 (lowest)
  * Semantically stable, not enum-based
* **Deletion**

  * Orthogonal to task status
  * Treated as a terminal condition

### 5.2 Domain Rules

* A deleted task is:

  * Invisible to default queries
  * Immutable (no updates, no status transitions)
* Priority must always be within range
* Due date may be null
* Optimistic concurrency must be enforced for all mutating commands

All rules must be represented as **pure domain logic** where feasible.

---

## 6. Application Layer Changes

### 6.1 New Command: DeleteTask (Soft Delete)

**Intent:** Logically delete a task while preserving all data.

**Inputs:**

* `task_id`
* `expected_row_version`

**Preconditions:**

* Task exists
* Task is not already deleted
* Authorization allows deletion

**Effects:**

* `is_deleted = TRUE`
* `deleted_at = now()`
* `updated_at = now()`
* `row_version = row_version + 1`

**Idempotency Rule (Normative):**

* If the task is already deleted, the command returns **success with no effect**.

---

### 6.2 Updated Commands (Concurrency-Aware)

The following existing commands must now enforce optimistic concurrency:

* `UpdateTaskDetails`
* `StartTask`
* `CompleteTask`

#### Concurrency Rule

Each command must:

* Accept `expected_row_version`
* Perform updates using:

```sql
WHERE id = $id AND row_version = $expected_row_version
```

If no rows are updated:

* Return a deterministic **ConcurrencyConflict** error

---

### 6.3 Query Semantics (Updated)

All existing queries must:

* Exclude deleted tasks by default:

  ```sql
  WHERE is_deleted = FALSE
  ```

Additional supported query behaviors:

* Filter by:

  * status
  * priority
* Sort by:

  * due date
  * priority
  * updated_at (default)

Queries must remain **side-effect free**.

---

## 7. Interface Layer (Functional Semantics Only)

> UI layout and components are intentionally unspecified.
> The following describes **behavioral intent only**, assuming a responsive Material-style web UI.

### 7.1 Task List

* Displays only non-deleted tasks
* Supports:

  * Status filter
  * Priority filter
  * Due-date-based sorting
* Visual indicators:

  * Priority level
  * Overdue state (derived from `due_at`)

### 7.2 Task Detail

* Displays:

  * Title
  * Description
  * Status
  * Priority
  * Due date
* Contextual actions:

  * Edit details (if not completed or deleted)
  * Start / Complete (based on status)
  * Delete

### 7.3 Deletion UX Semantics

* Delete action requires explicit confirmation
* Deleted tasks:

  * Disappear from default lists
  * Return “Not Found” for normal detail routes

---

## 8. Error Semantics (Extended)

New deterministic errors introduced:

* `TaskDeleted`
* `ConcurrencyConflict`
* `TaskNotFound`

Error classification remains consistent with Tee System rules:

* Validation errors
* Domain rule violations
* Authorization errors
* Infrastructure errors

---

## 9. Migration Requirements

A new migration (e.g., `0002_task_extensions.sql`) must:

* Add new columns
* Backfill defaults safely
* Add constraints and indexes
* Be idempotent

No manual schema changes are permitted.

---

## 10. Testing Requirements (Incremental)

Minimum additional coverage:

### Domain Tests

* Priority bounds
* Deletion immutability
* Transition rejection after deletion

### Integration Tests

* Create → Update → Delete happy path
* Update with stale `row_version`
* Queries exclude deleted tasks
* Sorting and filtering by due date and priority

---

## 11. Non-Goals (Reaffirmed)

This SPEC must not introduce:

* Physical deletes
* Restore/undelete
* Event sourcing
* Background schedulers
* Distributed caches
* Additional databases or services

---

## 12. Summary

SPEC-TASK-02 extends the Task reference implementation to a **credible, production-like example** by adding:

* Data-preserving deletion
* Scheduling semantics
* Concurrency correctness

All enhancements remain strictly within the **Tee System philosophy**:
minimal layers, explicit rules, predictable behavior, and high suitability for AI-assisted generation.

Any implementation of this SPEC must fully comply with the **Tee System Architecture Guidelines (v1.0)** and **SPEC-TASK-01**.
