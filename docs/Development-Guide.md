# Development Guide

This document describes how to **develop, run, and iterate** on the Tee System reference implementation.

It is intentionally practical. Architectural rules live elsewhere; this guide focuses on **day-to-day developer workflow**.

---

## Overview

The Tee System development workflow is designed to be:

- Simple and predictable
- Close to production behavior
- Friendly to both humans and AI-assisted tooling

The system consists of:
- One Rust service
- One PostgreSQL database

There are no mandatory external services.

---

## Prerequisites

You need the following installed locally:

- **Rust toolchain** (via `rustup`)
- **PostgreSQL** (local or containerized)
- **sqlx-cli** for query checking and migrations
- **Podman or Docker** (recommended for Postgres)
- **bacon** and **cargo-watch** (recommended for development)

---

## Database setup

### Option A: Local PostgreSQL

Ensure PostgreSQL 18+ is running and accessible.

Create a database named `tee`.

Set:

```bash
export DATABASE_URL=postgres://<user>:<password>@localhost:5432/tee
````

---

### Option B: Containerized PostgreSQL (recommended)

Using Podman:

```bash
podman run \
  --name tee-postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=tee \
  -p 5432:5432 \
  -d \
  postgres:18
```

Optional persistent volume:

```bash
-v postgres_data:/var/lib/postgresql
```

Stop and clean:

```bash
podman stop tee-postgres
podman rm tee-postgres
```

---

## SQLx workflow

This project uses **SQLx with compile-time query checking**.

Before building or running:

```bash
cargo sqlx prepare --database-url "$DATABASE_URL"
```

This generates metadata used by the compiler to validate SQL.

### When to re-run `sqlx prepare`

* After changing SQL queries
* After adding or modifying migrations
* After pulling changes that affect database access

---

## Running the service

Basic run:

```bash
cargo run
```

Environment variables:

```bash
export BIND_ADDR=127.0.0.1:8080
export LOG_LEVEL=info
```

In development mode:

* Database migrations are applied automatically at startup
* This is **not** recommended for production environments

---

## Development with file watching

### Recommended setup: bacon + cargo-watch

`bacon` orchestrates dev tasks.
`cargo-watch` handles reliable file watching and recompilation.

Install once:

```bash
cargo install bacon cargo-watch
```

Run:

```bash
bacon
```

This executes:

```bash
cargo watch -x run
```

### If you encounter file change issues

Some editors or container setups require polling:

```bash
cargo watch --poll -x run
```

Or run directly without bacon:

```bash
cargo watch -x run
```

---

## Migrations

Migrations live in:

```
migrations/
```

Apply manually if needed:

```bash
cargo sqlx migrate run
```

List applied migrations:

```bash
cargo sqlx migrate info
```

Never apply schema changes manually in the database.

---

## Pre-PR verification

Before opening a pull request, run:

```bash
bash scripts/verify.sh
```

This typically checks:

* Formatting
* Build
* SQLx metadata consistency
* Tests (if present)

If SQL changed, re-run:

```bash
cargo sqlx prepare --database-url "$DATABASE_URL"
```

---

## Project layout (high-level)

```
src/
  interface/   # HTTP, routing, auth extraction
  app/
    commands/  # state-changing use cases
    queries/   # read-only use cases
  domain/      # domain logic and policies
  data/        # SQL and database access
```

Development work should respect these boundaries.

---

## Common pitfalls

* Forgetting to run `sqlx prepare`
* Writing SQL in route handlers
* Adding implicit state in templates
* Introducing background infrastructure prematurely

If you hit any of these, stop and reassess.

---

## When in doubt

* Prefer explicit SQL over abstractions
* Prefer clarity over cleverness
* Prefer fewer moving parts

This is not accidental; it is the core philosophy of the Tee System.
