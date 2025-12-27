# SPEC-01  
## Task Registration & Lifecycle Management  
**Tee System Reference Use Case**

**Status:** Draft  
**Version:** 1.0  
**Based on:** Tee System Architecture Guidelines (v1.0) at docs/Tee-Architecture-Guidelines.md

---

## 1. Purpose of This Specification

This document defines a **non-trivial, production-grade Task Registration application** implemented according to the **Tee System Architecture Guidelines (v1.0)**.

The purpose is twofold:

1. **Demonstrate a realistic domain slice** that is sufficiently rich to:
   - Exercise command/query separation
   - Exercise domain rules and state transitions
   - Require meaningful data modeling and constraints
   - Serve as a reference for AI-assisted code generation

2. **Define a canonical extension** of the existing minimal “Task” skeleton into a **full task lifecycle management system**, without introducing additional infrastructure or violating the Tee System constraints.

This SPEC **does not define UI layout or visuals**. It provides **functional UI semantics only**, assuming a responsive, web-based, Material-style interface.

---

## 2. Architectural Context

This specification **inherits and depends on**:

- **Tee System Architecture Guidelines (v1.0)**  
  All constraints, principles, and prohibitions apply unless explicitly stated otherwise.

In particular:

- Two effective layers only:
  - Rust application service (“Tee Service”)
  - PostgreSQL database (“Tee Store”)
- Logical Command / Query split (no full CQRS)
- SQL as a first-class interface
- HTML-first, server-rendered UI (actual UI definition external)
- Postgres used for data integrity, not workflows

---

## 3. Scope

### In Scope

- Task registration
- Task lifecycle management
- Task state transitions
- Task listing and retrieval
- Domain-level validation and invariants
- Auditable, deterministic state changes

### Out of Scope

- User management and authentication (assumed external)
- Authorization beyond placeholder policy hooks
- Notifications (email, push, etc.)
- Collaboration, comments, attachments
- Reporting beyond basic task listing
- UI layout, theming, or component definitions

---

## 4. Domain Overview

### 4.1 Conceptual Model

A **Task** represents a unit of planned work with a clear lifecycle.

A Task:

- Is created with an initial intent
- Moves through a defined set of states
- Cannot skip or violate lifecycle rules
- Is mutable only via explicit Commands

---

## 5. Task Lifecycle

### 5.1 Task Status Enumeration

The Task lifecycle is modeled as a **finite state machine**.

Allowed statuses:

| Status        | Description |
|---------------|-------------|
| `PLANNED`     | Task is defined but not yet started |
| `IN_PROGRESS`| Task is actively being worked on |
| `COMPLETED`   | Task is finished and immutable |

### 5.2 Allowed State Transitions

| From          | To            | Allowed |
|---------------|---------------|---------|
| PLANNED       | IN_PROGRESS   | Yes |
| PLANNED       | COMPLETED     | No |
| IN_PROGRESS   | COMPLETED     | Yes |
| IN_PROGRESS   | PLANNED       | No |
| COMPLETED     | Any           | No |

These rules **must be enforced** at:

- Domain level (pure logic)
- Application command level
- Database level where feasible (constraints)

---

## 6. Data Model (Tee Store)

### 6.1 Table: `tasks`

```sql
tasks (
  id              UUID        PRIMARY KEY,
  title           TEXT        NOT NULL,
  description     TEXT        NOT NULL,
  status          TEXT        NOT NULL,
  created_at      TIMESTAMPTZ NOT NULL,
  updated_at      TIMESTAMPTZ NOT NULL
)
````

### 6.2 Constraints

* `title`

  * Required
  * 1–200 characters
* `description`

  * Required
  * Max length: implementation-defined (recommended ≤ 4000 chars)
* `status`

  * Enum-like constraint: `PLANNED | IN_PROGRESS | COMPLETED`
* `updated_at >= created_at`

Example integrity constraints:

```sql
CHECK (char_length(title) BETWEEN 1 AND 200)
CHECK (status IN ('PLANNED', 'IN_PROGRESS', 'COMPLETED'))
CHECK (updated_at >= created_at)
```

### 6.3 Indexes

* `(status)`
* `(created_at DESC)`
* `(updated_at DESC)`

---

## 7. Domain Layer (Pure Logic)

### 7.1 Domain Types

* `TaskId`
* `TaskTitle`
* `TaskDescription`
* `TaskStatus`
* `Task`

### 7.2 Domain Rules

* Title and description must be validated before persistence
* Status transitions must be validated via a pure function:

  ```text
  can_transition(from, to) -> bool
  ```
* Completed tasks are immutable

Domain logic **must not** depend on:

* HTTP concepts
* SQL or persistence types

---

## 8. Application Layer (Use Cases)

### 8.1 Commands (State-Changing)

All commands:

* Run within explicit transactions
* Validate authorization (placeholder policy)
* Enforce lifecycle rules
* Emit deterministic outcomes

#### 8.1.1 CreateTask

**Intent:** Register a new task

**Inputs:**

* title
* description

**Behavior:**

* Status initialized to `PLANNED`
* `created_at` and `updated_at` set to now

---

#### 8.1.2 StartTask

**Intent:** Move task from `PLANNED` → `IN_PROGRESS`

**Preconditions:**

* Task exists
* Status = `PLANNED`

**Effects:**

* Status updated
* `updated_at` updated

---

#### 8.1.3 CompleteTask

**Intent:** Move task from `IN_PROGRESS` → `COMPLETED`

**Preconditions:**

* Task exists
* Status = `IN_PROGRESS`

**Effects:**

* Status updated
* Task becomes immutable
* `updated_at` updated

---

#### 8.1.4 UpdateTaskDetails

**Intent:** Modify title and/or description

**Preconditions:**

* Task exists
* Status ≠ `COMPLETED`

**Effects:**

* Title and/or description updated
* `updated_at` updated

---

### 8.2 Queries (Read-Only)

Queries must be side-effect free.

#### 8.2.1 ListTasks

* Filterable by:

  * status
  * creation date
* Sorted by:

  * updated_at (default)

#### 8.2.2 GetTaskDetails

* Returns full task representation
* Used for detail view

---

## 9. Interface Layer (Functional UI Semantics)

The UI is **Material Design-inspired**, responsive, supports dark/light modes, and uses Material Symbols icons and Tailwind CSS.

**Reference UI Implementations:** See [docs/SPECS/SPEC-01/](./SPEC-01/) for actual HTML mockups and design screenshots:
- `task_list_screen/`: Task list view with sidebar, filters, and table
- `task_list_create/`: Task creation modal
- `stitch_task_view/`: Task detail screen with timeline and actions

---

### 9.1 Task List Screen

Functional elements:

* Task list with:

  * Title (clickable, navigates to detail screen)
  * Status indicator (color-coded badge: Blue = In Progress, Gray = Planned, Green = Completed)
  * Priority indicator with flag icon
  * Last updated timestamp (relative, e.g., "2h ago")
  * Optional subtitle context (e.g., department, due date)
* Filters:

  * Status filter chips: All | Planned | In Progress | Completed
  * Full-text search input
* Primary actions:

  * "Create Task" button (top-right, primary blue)

---

### 9.2 Task Detail Screen

Functional elements:

* Read-only display:

  * Task ID badge (e.g., "T-4022")
  * Status badge (animated if in progress)
  * Title (large h1 heading)
  * Description (Markdown-rendered prose)
  * Subtasks checklist with progress counter and "Add subtask" option
  * Activity timeline with state changes, comments, and timestamps
* Sidebar metadata:
  * Assignee
  * Due date
  * Created date
  * Updated date
* Contextual actions (based on status):

  * If `PLANNED`: "Start Task" primary button
  * If `IN_PROGRESS`: "Complete Task" primary button
  * If `COMPLETED`: no mutation buttons (read-only)
* Secondary actions: Copy Link, More (⋯)

---

### 9.3 Task Creation Screen

Functional elements:

* Modal / centered card with gradient background
* Form header:

  * Title: "Create New Task"
  * Subtitle: "Fill in the details below to add a new task to your project."
  * Close button (X icon)
* Form inputs:

  * **Task Title** (required, max 200 chars)
    * Autofocus enabled
    * Placeholder: "e.g. Redesign homepage hero section"
    * Edit icon indicator
  * **Priority Level** (optional, default = Medium)
    * Radio chip group: Low | Medium | High
    * Color-coded chips (Green/Orange/Red with dot indicator)
  * **Description** (required)
    * Textarea (min 160px height, user-resizable)
    * Markdown support hint
    * Placeholder: "Add details, context, subtasks, or links relating to this task..."
  * **Optional quick actions** (inline buttons):
    * Attach File (paperclip icon)
    * Set Due Date (calendar icon)
    * Assign to Assignee (person_add icon)
* Form footer:

  * Cancel button (secondary, gray)
  * Create Task button (primary, blue with checkmark icon)

**Design Features:**
* Smooth fade-in animation
* Custom scrollbar styling
* Focus rings: 2px ring in primary color
* Dark mode support with color adjustments

---

## 10. Data Access Layer

* All SQL must be parameterized
* No dynamic SQL
* Explicit queries per use case
* No business logic in SQL beyond constraints

---

## 11. Error Semantics

Errors must be deterministic and classified:

* Validation errors (user input)
* Domain rule violations (invalid state transitions)
* Authorization errors
* Infrastructure errors (DB, timeout)

UI-level error presentation is external to this spec.

---

## 12. Observability Requirements

* Log:

  * Task creation
  * Status transitions
  * Invalid transition attempts
* Metrics:

  * Command execution time
  * Failed command count
* Tracing:

  * Command and query boundaries

---

## 13. Testing Requirements

Minimum coverage:

* Domain tests:

  * Status transition matrix
  * Validation rules
* Command integration tests:

  * Create → Start → Complete happy path
  * Invalid transitions
* Query integration tests:

  * List filters
  * Detail retrieval

---

## 14. Non-Goals and Constraints Recap

This SPEC **must not introduce**:

* Full CQRS
* Event sourcing
* SPA frontend dependency
* Background infrastructure
* Stored-procedure-based workflows

---

## 15. Summary

This specification defines a **realistic, lifecycle-driven Task Registration system** that:

* Fully exercises the Tee System architecture
* Is complex enough to matter
* Remains constrained and predictable
* Is suitable as a **canonical reference for AI-assisted generation**

Any implementation of this SPEC must comply with the **Tee System Architecture Guidelines (v1.0)** in full.
See: docs/Tee-Architecture-Guidelines.md

