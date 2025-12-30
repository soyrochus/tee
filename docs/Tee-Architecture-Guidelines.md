# Tee System Architecture Guidelines (v1.0)

## Purpose and Scope

The **Tee System** is a constrained application architecture optimized for high-throughput delivery, operational simplicity, predictable performance, and AI-assisted code generation. It standardizes a **two-tier physical deployment model** (Rust service + PostgreSQL) while permitting multiple logical layers within the Rust codebase.

The architecture explicitly rejects:

- **Full CQRS** (separate read/write services and projection infrastructure)
- **Mandatory thick-client architectures**

It explicitly allows:

- **Progressive enhancement** with lightweight JavaScript patterns

The Tee System is intended for **internal and client-facing business applications** where the dominant workload consists of transactional CRUD combined with domain workflows, with **web UI and API exposure from the same service boundary**.

---

## Architecture Definition

### Physical Topology

#### A. Rust Application Service (“Tee Service”)

A single deployable unit responsible for:

- Serving server-rendered web pages (HTML) and associated static assets
- Serving HTTP APIs (JSON and optionally other media types)
- Enforcing business workflows, orchestration, and authorization decisions
- Owning transaction boundaries for state changes
- Providing observability signals (logs, metrics, traces) and health endpoints

#### B. PostgreSQL Database (“Tee Store”)

A single primary PostgreSQL instance responsible for:

- Durable state and relational data modeling
- Data integrity enforcement (constraints, foreign keys, unique constraints, check constraints)
- Query execution, indexing, and query-level optimization
- Optional: Row Level Security (RLS) where it materially strengthens authorization invariants
- Optional: views and materialized views for read performance and stable read shapes

---

### Optional Adjuncts (Allowed but Not Required)

- PostgreSQL read replica for read scaling
- External object storage for blobs (documents, images), referenced by database metadata
- Reverse proxy / ingress and TLS termination
- Background job runner within the **same Rust deployable** (same repository and binary) for scheduled tasks  
  Separate infrastructure is permitted **only by exception**

---

## Runtime Configuration (Normative)

- Configuration is environment-driven; missing required variables must fail fast with clear errors.
- Minimum variables: `DATABASE_URL` (required), `BIND_ADDR` (default allowed), `LOG_LEVEL` (default allowed).
- Secrets and credentials are never committed; source from environment or a secrets manager.
- Log configuration values at startup for operability (excluding secret values).

---

## Core Principle: Two Effective Layers

The architecture must remain understandable as:

```

Service + Database

```

Any addition that introduces a **third always-on operational tier** (message broker, distributed cache, separate projection services) requires **explicit justification and approval**.

---

## Key Principles (Normative)

### 1. Single Authoritative Service Boundary

All domain behavior is mediated by the **Tee Service**.  
External consumers interact **only** through the Tee Service.  
Direct database access by external systems is prohibited.

---

### 2. SQL Is a First-Class Interface

- Database access uses **explicit SQL** (preferred) or a constrained, type-safe mapping that compiles SQL and validates types
- ORM behavior that generates opaque SQL or hides query shape is prohibited for performance-critical paths

---

### 3. Command / Query Split (Logical CQRS)

The Tee Service separates:

- **Commands**: state-changing operations
- **Queries**: read-only operations

This split is **conceptual and structural**, not infrastructural.

---

### 4. Postgres Is Data-Focused

Business workflows and orchestration **do not live in the database**.

Postgres may enforce:
- Invariants
- Access policies

Postgres must not implement:
- Multi-step domain processes
- External side effects
- Integration orchestration

---

### 5. Progressive Enhancement by Default

- User interfaces are **server-rendered HTML** by default
- JavaScript is optional, minimal, and used as an enhancement layer
- Durable state is server-side

---

### 6. Explicit Invariants and Explicit Boundaries

- All state changes occur in **explicit transactions**
- Invariants are enforced via:
  - Application-level checks
  - Database constraints
- Timeouts, retries, and backpressure are explicit

---

### 7. Secure by Construction

- Default-deny access
- Least privilege
- Narrowly scoped authorization checks
- Secrets are never committed to source control
- Input validation and output encoding are enforced at boundaries

---

## Logical Architecture (Inside the Tee Service)

The Tee Service is organized into the following logical modules.  
These modules are required even if packaged into a single binary.

---

### A. Interface Layer

#### Responsibilities

- HTTP routing (web + API)
- Content negotiation and rendering
- Authentication integration (OIDC / SAML / headers)
- Request parsing and validation at the boundary
- Response shaping, caching directives, and error mapping

#### Rules

- No direct database access from route handlers
- No business rules in templates
- Authorization checks must be performed in:
  - The relevant Command/Query handler, or
  - A dedicated policy module called from it

#### HTTP Middleware Baseline

- Request/trace correlation: generate and propagate a unique request ID.
- Compression: enable response compression by default.
- Timeouts: enforce per-request timeouts with meaningful timeout responses.
- Body limits: enforce a reasonable maximum request body size.
- Apply these defaults to all routers unless explicitly overridden.

---

### B. Application Layer (Use Cases)

Split into:

- **Command handlers**  
  Mutate state, run in transactions, publish outbox entries where needed

- **Query handlers**  
  Read-only, may use optimized SQL, views/materialized views, and read replicas

#### Rules

- Commands must be idempotent where practical
- Queries must be side-effect free (no writes, no “last seen” updates)
- Command handlers define transaction boundaries
- Query handlers define performance envelopes and explain query plans for hot paths

---

### C. Domain Layer

#### Responsibilities

- Domain model and business rules (pure functions where feasible)
- Policy definitions and validation logic testable without I/O
- Domain events as internal structs (not necessarily event sourcing)

#### Rules

- Domain logic must not depend on HTTP or SQL types
- Integrity-critical invariants must be backed by database constraints

---

### D. Data Access Layer

#### Responsibilities

- SQL queries and prepared statements
- Mapping to domain DTOs
- Schema migrations and versioning
- Transaction primitives and connection management

#### Rules

- All SQL must be parameterized
- No dynamic SQL from untrusted input
- Migrations are mandatory
- Schema changes are never applied manually in production

---

## Database Guidelines (Tee Store)

### 1. Schema and Integrity

- Use normalized schema by default
- Enforce integrity using:
  - NOT NULL
  - CHECK
  - UNIQUE
  - Foreign keys
- Use surrogate keys unless natural keys are clearly stable
- Use timestamps with time zones; store UTC

---

### 2. Data-Side Logic Boundaries

#### Allowed

- Constraints and indexes
- Views and materialized views
- Small deterministic validation/predicate functions
- RLS policies when they materially reduce authorization risk

#### Discouraged or Prohibited by Default

- Large stored procedures implementing workflows
- Triggers with hidden side effects
- In-database network calls or external integrations

If used, they must be isolated, documented, and covered by tests.

---

### 3. Performance

- Require indexes for high-frequency access patterns
- Require `EXPLAIN (ANALYZE, BUFFERS)` for hot queries
- Prefer set-based operations over per-row loops
- Use materialized views for expensive aggregates and dashboards

---

## Command / Query Definitions (Normative)

### Command

An operation that changes durable state or produces an external side effect.

**Examples**

- Create / update / delete
- Approvals and state transitions
- Notifications (recorded via outbox)
- Audit entries
- Counters, “last seen”

#### Requirements

- Must run in an explicit transaction
- Must validate authorization
- Must enforce invariants
- Must produce deterministic error semantics
- Must be observable (logs, metrics)

---

### Query

An operation that returns information and does not change durable state.

**Examples**

- Page view models
- Resource listings
- Detail retrieval

#### Requirements

- Must not perform writes
- Must define performance expectations (p95 targets)
- Must return stable, versioned API shapes
- Must apply authorization filters consistently

---

## Web UI and Progressive Enhancement Guidelines

### Default: Server-Rendered HTML

- Page routes return HTML from Query handlers
- Forms submit to Command endpoints
- Use PRG (Post-Redirect-Get)

### Enhancement: Minimal JavaScript

Allowed patterns:

- HTMX-style partial updates
- Small JS “islands” for widgets
- Non-SPA navigation with async fetch where needed

#### Rules

- Core workflows must function without JavaScript
- Client-side state must be transient

---

## API Design Guidelines

- Resource-oriented endpoints
- Explicit versioning
- Consistent error envelopes
- ETags and conditional requests where appropriate
- Rate limiting and request size limits enforced

---

## Security Guidelines (Minimum Baseline)

- Authentication via standard protocols (OIDC recommended)
- Authorization enforced at use-case boundaries
- Strict input validation
- HTML output encoding by default
- Parameterized SQL only
- Secrets via environment or secret manager
- Audit logging for security-relevant commands
- RLS optional, not a convenience layer

## Authentication & I18N Guardrails (Implementation compliance)

The following guardrails codify the specific rules required to remain compatible with the current reference implementation and with the authentication and i18n specifications in `docs/SPECS/SPEC-03-AUTH.md` and `docs/SPECS/SPEC-04-I18N.md`.

These items are normative for any service claiming compliance with the Tee System reference implementation.

### Authentication & Session Management

- Principal: every authenticated request must resolve to a `Principal` object carrying at minimum `subject`, `display_name`, and optional `email`, `tenant_id`, and `roles`. The `Principal` is passed explicitly into Command handlers and Query handlers that require identity. (See the design in `docs/SPECS/SPEC-03-AUTH.md`.)
- Session cookie: use a host-scoped cookie that follows hardening rules: name must use the `__Host-` prefix, `Secure`, `HttpOnly`, `SameSite=Lax` (or `Strict`), `Path=/`, and **no Domain attribute**. Example: `Set-Cookie: __Host-tee_session=...; Secure; HttpOnly; SameSite=Lax; Path=/`. See `docs/SPECS/SPEC-03-AUTH.md` for details.
- Session tokens: tokens MUST be cryptographically random (≥ 256 bits) and stored only as hashed values in the `sessions` table. Rotation on login and revocation semantics (single-session logout and optional global logout) are required.
- Storage and migrations: migrations creating `users` and `sessions` must live in `migrations/` and be reviewed with schema changes. The `sessions` table MUST include `session_token_hash`, `expires_at`, `last_seen_at`, and `revoked_at` columns as specified in the SPEC.
- Idle timeout and absolute expiry: enforce both an absolute expiry and a sliding idle timeout on the server side. Update `last_seen_at` on authenticated requests and treat exceeded idle timeout as expired.
- CSRF: all state-changing HTTP endpoints reachable from browser pages MUST enforce CSRF protection (synchronizer token or double-submit cookie). CSRF tokens must be bound to the session and validated in the Interface layer before invoking Commands.
- Password storage: if using the local DB auth mode, store password hashes using Argon2id (or an equivalent modern KDF) per the SPEC.
- API tokens / JWTs: for API clients, accept `Authorization: Bearer <token>` for protected endpoints. Map validated tokens to the same `Principal` type used by session-based auth so application logic remains uniform.
- Auth adapter location: authentication plumbing and token/session validation must live in the Interface layer (see `src/interface/auth.rs` and `src/interface/state.rs`). Handlers should not re-implement token parsing or low-level cookie handling.
- Error semantics: translate authentication and authorization failures to clear HTTP responses: `401` for unauthenticated, `403` for forbidden, `404` for resources not visible to the caller, and use a consistent JSON or HTML error envelope depending on content negotiation.

### I18N / Locale Handling

- Locale resolution precedence (Interface layer): 1) explicit user preference (Principal/profile), 2) `locale` cookie, 3) `Accept-Language` header, 4) application default (e.g., `en`). Implement as middleware that attaches a `Locale` value to the request state.
- Translator loading: repository-first catalogs under `locales/<locale>/main.ftl` are normative. Load bundles at startup into an in-memory `Translator` and provide thread-safe access for handlers and template rendering. Hot-reload in development is allowed.
- Template integration: resolve translations in Query/handler code and pass localized view models (strings only) to Askama templates. Templates must not perform I/O. If template helpers are used, they must be pure in-memory lookups against the loaded bundles.
- Fallback semantics: Translator lookup must attempt the requested locale, fall back to the default locale, and finally return a stable placeholder and emit a metric/log if a key is missing.
- Required locales: implementations MUST provide `en` plus any locales configured for the deployment. CI should enforce key consistency across required locales (missing keys either fail the build or produce actionable warnings per project policy). See `docs/SPECS/SPEC-04-I18N.md` for the required list of locales.
- Number/date formatting: perform locale-aware formatting in the Interface layer using a small formatting helper (ICU-based crates if needed). Do not leave raw domain datetimes to templates for ad-hoc formatting.
- DB-backed translations: allowed only as an opt-in extension. If used, require explicit migrations, caching, and a clear invalidation strategy. Repository-based catalogs are preferred for UI copy.

### Routing, Middlewares, and Error Handling (Practical Guardrails)

- Separate web and API routers (or clearly prefix API routes with `/api`) so that content negotiation and response shaping remain trivial and deterministic. The Interface layer router should mount the web routes (HTML-first) and the API routes (`/api/v{n}`) separately and merge them into the main router in `src/interface/http.rs`.
- Authentication middleware must produce a `Principal` or a clear unauthenticated context and attach it to request state before handlers run. The same middleware should update `last_seen_at` and enforce idle timeout policies.
- CSRF checks and cookie rotation must happen before invoking Commands. Commands must assume that CSRF and auth checks have already run when they receive a `Principal`.
- Error envelopes: choose one canonical error envelope per interface type and document it (HTML error pages for browser routes; JSON envelopes for API routes). Keep the mapping consistent across the codebase.

### Testing and CI Guardrails

- Add integration tests covering:
  - Session creation, validation, idle timeout, and revocation
  - CSRF enforcement on state-changing routes
  - Locale resolution and template outputs for at least two locales
  - Key consistency checks for translation catalogs (CI lint)
- Add linting/CI jobs to:
  - Validate Fluent file syntax and run key-consistency checks across `locales/`
  - Verify `migrations/` include required `users`/`sessions` schema when Auth mode A is used
  - Run SQL compile-time checks (e.g., `sqlx` prepare) to ensure SQL shapes are stable

### Notes on Implementation Boundaries

- Interface layer: responsible for cookie parsing, token validation, CSRF, locale detection, and translation lookup. It must not contain domain rules beyond authentication plumbing.
- Application/Domain layers: responsible for authorization, business invariant enforcement, and use-case logic. Commands and Queries must accept a `Principal` and apply policy rules deterministically.
- Data layer: responsible for storing only hashed session tokens and other secrets; no plaintext tokens in DB or logs.

Refer to `docs/SPECS/SPEC-03-AUTH.md` and `docs/SPECS/SPEC-04-I18N.md` for the full normative text. Implementations that deviate from these guardrails must produce an ADR explaining the rationale, security implications, and rollback plan.

---

## Reliability and Observability Guidelines

- Liveness and readiness health endpoints
- Metrics:
  - Request duration
  - Error rates
  - DB pool saturation
  - Background queue depth
  - Slow query counts
- Distributed tracing with propagated trace IDs
- Explicit timeouts
- Bounded concurrency and graceful degradation

### Logging and Tracing Baseline

- Log level configured via environment; defaults permitted but must be overridable.
- Structured logging with request IDs included on every request log entry.
- Emit traces/spans for HTTP requests and critical command/query paths.

---

## Sustainability Guidelines (Operational Efficiency)

- One deployable + one database per environment
- Avoid always-on auxiliary infrastructure
- Measure CPU-seconds per request
- Minimize dependencies and image size
- Prefer caching before adding distributed systems

---

## Performance Guidelines

- Optimize p95/p99 latency
- Bounded connection pools
- Avoid N+1 queries
- Use pagination and server-side filtering
- Stream large exports

---

## Background Tasks

### Default Approach: In-Binary Worker

- Scheduled and async work runs as a runtime mode of the same binary
- Database used for coordination (job tables)

---

## Outbox Pattern (Recommended)

- Commands write outbox records in the same transaction
- Background worker delivers events with retries

### Command and Query Behavior (Normative)

- Commands run inside explicit transactions and should be idempotent where practical.
- Prefer optimistic concurrency tokens to avoid lost updates; conflicts must return deterministic error responses.
- Queries are side-effect free and must not emit state changes.

### Database Access and Pooling

- Use bounded connection pools with acquisition timeouts; document defaults per service.
- Apply compile-time SQL validation in CI/build (e.g., sqlx prepare/offline metadata) to keep SQL and types in sync without a live database.

### Migrations and Run Modes

- Every schema change requires a migration; manual production changes are prohibited.
- Auto-running migrations is acceptable in dev/test; production should run migrations as an explicit step before service startup.

---

## Prohibited by Default (Requires Explicit Exception)

- Full CQRS infrastructure
- Mandatory SPA frontend
- Event sourcing as default
- Distributed caches as first resort
- Stored-procedure-heavy workflows
- Multiple databases per bounded domain without justification

---

## Standard Deliverables per Tee System Service

### Repository Structure

- One Rust workspace/repository
- Modules:
  - `interface/`
  - `app/commands/`
  - `app/queries/`
  - `domain/`
  - `data/`
- `migrations/`
- `docs/`

---

### Testing Requirements

- Domain unit tests
- Integration tests for Commands and Queries
- API contract tests
- Performance smoke tests

---

### Documentation Requirements

- Endpoint catalog
- Data model overview
- Operational runbook
- Security notes

---

## Code Generation Constraints (For Agent Use)

A code generation agent targeting the Tee System must:

- Generate server-rendered HTML by default
- Implement every mutation as a Command
- Implement every read as a Query
- Place all SQL in the data layer
- Enforce authorization at use-case boundaries
- Add database constraints for integrity
- Avoid new infrastructure by default
- Produce migrations for schema changes
- Include both unit and integration tests

---

## Glossary

- **Tee Service**: Rust application serving web and APIs  
- **Tee Store**: PostgreSQL database  
- **Command**: State-changing operation  
- **Query**: Read-only operation  
- **Logical CQRS**: Command/query separation within one service and DB  
- **Progressive Enhancement**: HTML-first UI with optional minimal JS

---

## Versioning and Governance

This document defines the baseline.

Any deviation requires an **Architecture Decision Record (ADR)** describing:

- Motivation
- Alternatives considered
- Operational impact
- Rollback plan
