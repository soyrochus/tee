# Tee System — Runnable Reference Skeleton

*Performance by Rust. Simplicity by design. AI-ready.*

[![Rust](https://img.shields.io/badge/Rust-lang-black?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE) [![FOSS Pluralism](https://img.shields.io/badge/FOSS%20Pluralism-Manifesto-brightgreen?style=flat)](FOSS_PLURALISM_MANIFESTO.md)

![Tee logo](images/tee-logo-small-transparent.png)

## What this is

This repository is a **minimal, runnable reference implementation** of the **Tee System** architecture.

It demonstrates a deliberately constrained way of building business applications:

- **Two effective runtime layers**: a single Rust service + PostgreSQL
- **Server-rendered HTML** by default, with optional progressive enhancement
- **Logical Command / Query split** (no full CQRS infrastructure)
- **SQL as a first-class interface** (explicit, parameterized queries; no ORM magic)
- **PostgreSQL enforces integrity**, Rust enforces workflows and orchestration

The included **Task** implementation is a **deliberately bounded example**. It exists to expose architectural seams, not to showcase features.

For a concrete walkthrough of the example, see
→ [`docs/Tasks-Example-Implementation.md`](./docs/Tasks-Example-Implementataion.md)

For the authoritative rules and constraints, see
→ [`docs/Tee-Architecture-Guidelines.md`](./docs/Tee-Architecture-Guidelines.md)

For the Development Guide, see
→ [`docs/Development-Guide.md`](./docs/Development-Guide.md)

For the details about Authentication, see
→ [`docs/Authentication.md`](./docs/Authentication.md)


For a description on how to add an API (ReST) layer, see
→ [`docs/Add-Rest-API..md`](./docs/Add-Rest-API.md)
---

## Why the Tee System exists

Modern application stacks tend to accumulate **accidental complexity** very early: multiple services, thick clients, opaque data layers, asynchronous pipelines, and infrastructure added “just in case.” This complexity is often justified as preparation for scale, but in practice it makes systems harder to reason about, harder to secure, and harder to evolve — especially when the core workload is still transactional business logic.

The Tee System starts from a different premise:

> **Most business applications do not fail because they are too simple.  
> They fail because meaning is spread across too many layers.**

### Simplicity as a deliberate constraint

Tee deliberately constrains the architectural solution space:

- **Two effective runtime layers only**: application service + database
- **No mandatory third tier** (brokers, projection services, distributed caches)
- **Explicit data access** via SQL instead of hidden ORM abstractions
- **Explicit workflows** in Rust instead of behavior emerging from conventions

This is not minimalism for its own sake. It is a recognition that **coherence beats cleverness** for most business systems.

### Designed for AI-assisted development

AI-assisted code generation amplifies both good and bad architecture.

AI systems work best when:
- Intent is explicit
- Rules are enforced rather than inferred
- Boundaries are clear and stable
- State lives in one place

They struggle when:
- Meaning is distributed across loosely coupled layers
- Behavior emerges from conventions
- State is duplicated across client, service, and middleware

The Tee System is intentionally **AI-friendly**:
- A small number of architectural primitives
- Clear Command / Query boundaries
- SQL that makes data shape and intent explicit
- Predictable lifecycle and state transitions

This allows both humans and AI agents to reason about the system **as a whole**.

### PostgreSQL as a foundation, not a dumping ground

In Tee, PostgreSQL is treated as a **first-class system**:

- It enforces integrity and invariants
- It executes queries efficiently and predictably
- It provides strong indexing and aggregation capabilities

At the same time, Tee explicitly rejects pushing workflows into the database. Business logic, orchestration, and policy decisions live in Rust, where they can be tested and evolved safely.

### Server-rendered first, not anti-frontend

Tee is **HTML-first**, not anti-frontend.

Server-rendered HTML provides:
- A stable, debuggable baseline
- Predictable performance characteristics
- Reduced client-side state complexity

Progressive enhancement is explicitly allowed:
- Lightweight JavaScript
- Partial updates
- Rich interactions where they add clear value

What Tee rejects is the assumption that every application must start as a complex SPA.

### Operational reality over theoretical scalability

Tee optimizes for:
- Predictable latency
- Low idle resource usage
- Clear failure modes
- Straightforward observability

Scaling strategies are **incremental and explicit**:
- Indexes before caches
- Read replicas before service fan-out
- Clear justification before adding infrastructure

This keeps systems cheaper to run, easier to operate, and easier to secure.

### A reference, not a framework

The Tee System is not a framework and not a silver bullet.

It is a **reference architecture and a set of guardrails**:
- For teams that want to move fast without losing control
- For organizations adopting AI-assisted development responsibly
- For systems that value correctness, auditability, and long-term maintainability

If you disagree with these constraints, Tee will feel restrictive.  
If you agree with them, Tee provides a sharp, stable starting point.

---

## Why this repository exists

This repository exists to make the Tee System **concrete and inspectable**:

- To showcase the Tee constraints in a runnable form
- To demonstrate Command / Query separation with explicit SQL
- To provide an HTML-first baseline suitable for real business apps
- To serve as a reference starting point for extensions via migrations, commands, queries, and routes

---

## Default stack

- **Axum (Tokio)** — HTTP server
- **tower-http** — middleware
- **Askama** — server-side HTML templates
- **SQLx** — Postgres access and migrations
- **tracing** — structured observability

---

## Quickstart (happy path)

Prerequisites:
- Rust toolchain
- PostgreSQL (local or containerized)
- `sqlx-cli`

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/tee
export BIND_ADDR=127.0.0.1:8080
export LOG_LEVEL=info

# or run `source db.env`

cargo sqlx prepare --database-url "$DATABASE_URL"
cargo run
````

On startup, migrations are applied automatically in development mode.

Open:

* [http://127.0.0.1:8080/tasks](http://127.0.0.1:8080/tasks)

For full setup instructions, containers, and development workflow, see
→ `docs/Development.md`

---

## Guide (mdBook)

This repository includes a developer Guide built with `mdBook` in the `guide/` folder. The Guide contains walkthroughs, diagrams (Mermaid), and hands-on exercises.

Quick steps to run the Guide locally:

Prerequisites: Rust toolchain (cargo).

Install `mdbook` and the Mermaid preprocessor (global):
```bash
cargo install mdbook 
cargo install mdbook-mermaid 
```

Build and serve the Guide:
```bash
cd guide
mdbook build   # generates static site in guide/book
mdbook serve   # start dev server (default: http://localhost:3000)
# or when using project-local binaries:
./tools/bin/mdbook serve
```


## Architectural conventions (normative)

* Route handlers do **not** call SQL directly
* Every mutation is a **Command** (`src/app/commands/`)
* Every read is a **Query** (`src/app/queries/`)
* SQL lives in `src/data/` and is always parameterized
* Database code is **data-focused**: constraints, indexes, simple reads/writes
* UI is HTML-first; use PRG (Post/Redirect/Get) after form submissions

---

## Authentication

The reference implementation includes a **local, DB-backed authentication mode** and a clean seam for **enterprise OIDC / AD integration**.

See:

* [`docs/Authentication.md`](./docs/Authentication.md) (overview)


---

## Non-goals

* Full CQRS (separate write/read services and projections)
* Mandatory SPA frontend
* Event sourcing as the default persistence model
* Adding always-on infrastructure by default

---

## Participation

Contributions are welcome: issues, pull requests, critique, and discussion.

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md), affirming respect for people, freedom to critique ideas, and space for diverse perspectives.

---

## License

Copyright (c) 2026, Iwan van der Kleijn
Licensed under the MIT License. See [`LICENSE`](./LICENSE) for details.

