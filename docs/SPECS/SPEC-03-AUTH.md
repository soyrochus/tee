# SPEC-03-AUTH  
## Authentication, Session Management, and Identity Integration  
**Tee System Reference Extension**

**Status:** Draft  
**Version:** 1.0  
**Depends on:**  
- Tee System Architecture Guidelines (v1.0)  ( docs/Tee-Architecture-Guidelines.md)
- SPEC-01 — Task Registration & Lifecycle Management  
- SPEC-02 — Soft Delete, Scheduling, and Concurrency  (docs/SPECS/SPEC-02.md)

---

## 1. Purpose

This specification introduces **authentication and session management** as a first-class extension to the Tee System reference implementation.

The goals are:

- Provide a **solid, production-grade authentication baseline**
- Remain fully compliant with Tee System constraints (two effective layers)
- Support **both a simple DB-backed authentication mode** and a **clean enterprise AD / OIDC integration seam**
- Define **session and cookie hardening rules** suitable for real deployments
- Keep authorization logic explicitly separate and enforceable at use-case boundaries

This SPEC intentionally focuses on **authentication and session plumbing**, not on complex authorization models or identity governance.

---

## 2. Architectural Context

This specification fully adheres to the **Tee System Architecture Guidelines (v1.0)**.

In particular:

- Authentication is handled in the **Interface Layer**
- Authorization is enforced in the **Application / Domain layers**
- No additional infrastructure tiers are introduced
- PostgreSQL remains the sole durable store
- External identity providers are integrated via **OIDC or trusted headers**, not custom protocols

---

## 3. Scope

### In Scope

- User authentication
- Session management
- Secure cookie handling
- Logout and session revocation
- Identity abstraction for use in domain policies
- Enterprise AD / OIDC integration seam

### Out of Scope

- Fine-grained authorization models
- Role management UI
- User provisioning workflows
- Multi-factor authentication
- Password reset flows
- Account lockout policies

---

## 4. Identity Model

### 4.1 Principal (Normative)

All authenticated requests resolve to a **Principal**, which represents the identity context passed into commands and queries.

Minimum required fields:

| Field | Description |
|------|-------------|
| `subject` | Stable unique identifier for the user |
| `display_name` | Human-readable name |
| `email` | Optional |
| `tenant_id` | Optional, required for multi-tenant scenarios |
| `roles` | Optional, coarse-grained |

The Principal is **immutable** per request and must not depend on HTTP or database types.

---

## 5. Authentication Modes

### 5.1 Mode A — Local DB-Based Authentication (Reference Implementation)

This mode is intended for:

- Development
- Demonstrations
- Self-contained deployments

It provides **realistic plumbing** while remaining simple.

---

## 6. Data Model (Local Auth Mode)

### 6.1 Table: `users`

```sql
users (
  id UUID PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  display_name TEXT NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL
)
````

Constraints:

* Email must be unique
* Passwords must be stored using **Argon2id**
* Disabled users must not authenticate

---

### 6.2 Table: `sessions`

```sql
sessions (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id),
  session_token_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  last_seen_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ NULL
)
```

Rules:

* Only **hashed session tokens** are stored
* A session is valid iff:

  * `revoked_at IS NULL`
  * `now() < expires_at`

Indexes:

* `(session_token_hash)`
* `(user_id)`
* `(expires_at)`

---

## 7. Session Semantics (Normative)

### 7.1 Session Creation

* A new session is created upon successful authentication
* Session token:

  * Cryptographically random (≥ 256 bits)
  * Stored **only as a hash**
* Session lifetime:

  * Absolute expiry (e.g. 8–24 hours)
  * Sliding idle timeout (e.g. 30–60 minutes)

### 7.2 Idle Timeout

On each authenticated request:

* `last_seen_at` is updated
* If idle timeout exceeded:

  * Session is treated as expired
  * Re-authentication required

---

### 7.3 Logout and Revocation

#### Logout (Single Session)

* Marks `revoked_at = now()`
* Clears client cookie
* Idempotent

#### Global Logout (Optional)

* Revokes all active sessions for a user
* Useful for password changes or incident response

---

## 8. Cookie Hardening (Mandatory)

Authentication relies on a **single session cookie** with the following properties:

| Attribute | Requirement                 |
| --------- | --------------------------- |
| Name      | Must use `__Host-` prefix   |
| Secure    | Required                    |
| HttpOnly  | Required                    |
| SameSite  | `Lax` (default) or `Strict` |
| Path      | `/`                         |
| Domain    | Not set                     |

Example:

```
Set-Cookie: __Host-tee_session=…;
  Secure;
  HttpOnly;
  SameSite=Lax;
  Path=/;
```

Rules:

* Cookies must never be accessible via JavaScript
* Cookies must never be scoped to subpaths or subdomains
* Session cookies must be rotated on login

---

## 9. Interface Layer Responsibilities

### 9.1 Endpoints

| Endpoint       | Method | Purpose                                    |
| -------------- | ------ | ------------------------------------------ |
| `/auth/login`  | GET    | Render login page                          |
| `/auth/login`  | POST   | Authenticate user                          |
| `/auth/logout` | POST   | Logout (revoke session)                    |
| `/auth/me`     | GET    | Return current principal (debug / API use) |

---

### 9.2 Login UI (Reference, HTML-First)

The login form reuses the default layout and the Tasks form styling (server-rendered, Tailwind classes, light/dark support):

- **Route:** GET `/auth/login` renders the page; POST `/auth/login` processes credentials and redirects on success (PRG).
- **Structure:** Centered card/dialog with title "Sign in", subtitle "Access your workspace", and a logo/title row consistent with the Tasks pages.
- **Fields:**
  - Email (required, type=email, autofocus)
  - Password (required, type=password, show/hide toggle optional)
  - Remember me (checkbox) — optional; only extends cookie/session lifetime if implemented server-side
- **Controls:** Primary submit button "Sign in"; secondary link to go back to `/tasks`.
- **Validation & errors:** Inline message area above the form for invalid credentials or inactive user; field-level inline errors for missing/invalid input. No information leak on which field failed beyond "Invalid credentials".
- **CSRF:** Hidden input with CSRF token; token bound to session.
- **Accessibility:** Proper labels and `aria-invalid` on error; focus ring on inputs and submit.
- **Theming:** Uses the same light/dark class toggling as the Tasks templates.

---

### 9.2 Request Authentication Flow

For each incoming request:

1. Extract session cookie
2. Hash token
3. Load session from DB
4. Validate expiry and revocation
5. Load user
6. Build `Principal`
7. Attach principal to request context

If any step fails:

* Treat as unauthenticated
* Return 401 or redirect to login (UI routes)

---

## 10. Command / Query Integration

### 10.1 Commands

All commands that mutate state must:

* Require an authenticated Principal
* Pass the Principal explicitly into the handler
* Enforce authorization via domain policy

### 10.2 Queries

Queries may be:

* Public (no Principal)
* Authenticated
* Admin-only

Authorization decisions are made **inside the query handler**, not in the router.

---

## 11. CSRF Protection (Mandatory)

For all authenticated, state-changing endpoints:

* Implement CSRF protection using one of:

  * Synchronizer token pattern
  * Double-submit cookie pattern

Rules:

* CSRF tokens must be tied to the session
* Tokens must be validated server-side
* CSRF checks are enforced in the Interface Layer

---

## 12. Enterprise Identity Seam (AD / OIDC)

### 12.1 OIDC Integration

The Tee Service may act as an OIDC client:

* Authorization Code flow
* Validate:

  * Issuer
  * Audience
  * Signature (JWKS)
  * Nonce and state

Upon successful callback:

* Create a Tee session
* Map ID token claims → Principal
* Optionally create a local user record (“just-in-time”)

### 12.2 Trusted Header Mode (Optional)

In platform-managed environments:

* Identity headers may be accepted:

  * `X-Authenticated-User`
  * `X-Email`
  * `X-Groups`
* Must only be trusted behind a verified gateway (mTLS or equivalent)

---

## 13. Error Semantics

Deterministic errors include:

* `AuthenticationRequired`
* `InvalidCredentials`
* `SessionExpired`
* `SessionRevoked`
* `CsrfViolation`

Errors must not leak sensitive information.

---

## 14. Observability and Audit

### 14.1 Logs

Log the following events:

* Login success / failure
* Logout
* Session expiry
* Session revocation

### 14.2 Audit Table (Optional but Recommended)

```sql
auth_events (
  id UUID PRIMARY KEY,
  event_type TEXT,
  user_id UUID NULL,
  occurred_at TIMESTAMPTZ,
  metadata JSONB
)
```

---

## 15. Testing Requirements

Minimum coverage:

* Password verification
* Session creation and validation
* Idle timeout expiry
* Logout and revocation
* Cookie attribute enforcement
* CSRF enforcement

---

## 16. Non-Goals (Reaffirmed)

This SPEC must not introduce:

* OAuth token issuance by the Tee Service
* External session stores
* Distributed identity caches
* Identity workflows beyond login/logout

---

## 17. Summary

SPEC-AUTH-03 completes the Tee System reference implementation by adding **secure, realistic authentication and session management**, while preserving:

* Architectural simplicity
* Operational clarity
* AI-assisted generation suitability

It defines a **clean identity seam** that scales from local development to enterprise AD / OIDC environments without architectural refactoring.


Any implementation of this SPEC must fully comply with the **Tee System Architecture Guidelines (v1.0), SPEC-01 and SPEC-02**.
