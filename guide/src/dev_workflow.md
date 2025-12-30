# Development Workflow

This chapter summarizes the day-to-day workflow, based on
`docs/Development-Guide.md`.

## Prerequisites

You will need:
- Rust toolchain (via rustup).
- PostgreSQL (local or containerized).
- sqlx-cli for SQL checks and migrations.

## Environment variables

Required:
- `DATABASE_URL`

Optional:
- `BIND_ADDR`
- `LOG_LEVEL`

## Typical workflow

1. Start Postgres.
2. Set `DATABASE_URL`.
3. Run SQLx prepare:
   ```bash
   cargo sqlx prepare --database-url "$DATABASE_URL"
   ```
4. Run the service:
   ```bash
   cargo run
   ```

## Migrations

Migrations live in `migrations/`. Use:
```bash
cargo sqlx migrate run
```

Never change schema manually in production.

## Pre-PR check

Use the existing script:
```bash
bash scripts/verify.sh
```

This verifies formatting, builds, SQLx metadata, and tests.

### Code example: verify script

From `scripts/verify.sh`:

```bash
export SQLX_OFFLINE=true

echo "==> rustfmt"
cargo fmt --all -- --check

echo "==> build"
cargo build --locked

echo "==> clippy"
cargo clippy --all-targets --all-features --locked -- -D warnings

echo "==> test"
cargo test --all --locked
```

## Exercise

- Add a new migration and run `cargo sqlx prepare`.
- Run `scripts/verify.sh` and fix any failures.

Next: Extending the System.
