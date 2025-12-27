# Authentication

This document explains **how authentication works** in the Tee System reference implementation and how it is intended to evolve in real deployments.

It focuses on **mechanics and structure**, not formal specifications.

---

## Design goals

Authentication in the Tee System is designed to be:

- Explicit and understandable
- Secure by default
- Compatible with enterprise identity systems
- Cleanly separated from business logic

Authentication is handled at the **interface boundary**.  
Authorization decisions happen deeper in the system.

---

## Identity model

Authenticated requests resolve to a **Principal** object representing the caller.

A Principal typically contains:
- A stable subject identifier
- Display name
- Email (optional)
- Tenant identifier (optional)
- Role or group hints (optional)

The Principal is:
- Immutable per request
- Independent of HTTP or database types
- Passed explicitly into commands and queries

---

## Local authentication mode (reference implementation)

The repository includes a **local, database-backed authentication mode**.

This mode exists to:
- Make the example runnable end-to-end
- Provide realistic plumbing
- Avoid external dependencies

### How it works

- Users are stored in the database
- Passwords are hashed using **Argon2id**
- Successful login creates a server-side session
- Clients authenticate via a secure session cookie

This mode is suitable for:
- Development
- Demonstrations
- Small self-contained deployments

---

## Sessions

Authentication uses **server-side sessions**, not JWTs.

### Session lifecycle

- A random session token is generated on login
- Only a **hash** of the token is stored in the database
- The raw token is stored in a secure cookie
- Each request validates the session and loads the Principal

Sessions have:
- An absolute expiration time
- An idle timeout
- Explicit revocation support

---

## Cookie hardening

Authentication cookies follow strict rules:

- `__Host-` prefix
- `Secure`
- `HttpOnly`
- `SameSite=Lax` (or `Strict` if possible)
- `Path=/`
- No `Domain` attribute

Cookies are:
- Never accessible from JavaScript
- Rotated on login
- Cleared on logout

---

## Logout and revocation

Logout:
- Revokes the current session
- Clears the cookie
- Is idempotent

Revocation:
- Sessions can be invalidated server-side
- Useful for security events or user deactivation

---

## CSRF protection

Because the system is HTML-first, **CSRF protection is mandatory**.

All state-changing requests must:
- Include a CSRF token
- Validate the token server-side

The CSRF mechanism is tied to the session and enforced in the interface layer.

---

## Authorization

Authentication answers **who you are**.  
Authorization answers **what you are allowed to do**.

Authorization logic:
- Lives in domain policy modules
- Is enforced inside command and query handlers
- Never lives in routing code or templates

This keeps policy decisions testable and explicit.

---

## Enterprise identity integration

In real deployments, authentication typically integrates with corporate identity systems.

The Tee System supports this without architectural changes.

### OIDC / AD integration

The service can act as an OIDC client:
- Redirect-based login
- Token validation
- Claim mapping to Principal

After authentication:
- A normal Tee session is created
- The rest of the system remains unchanged

### Trusted proxy mode

In some environments, identity is terminated upstream.

In this case:
- The service accepts identity headers
- Only from trusted network paths
- The same Principal abstraction is used internally

---

## What authentication does *not* do

The reference implementation intentionally avoids:

- Role management UIs
- Password reset flows
- MFA
- Account provisioning pipelines

These are orthogonal concerns and would obscure the core architecture.

---

## Summary

Authentication in the Tee System:

- Is explicit and boring by design
- Keeps identity concerns at the boundary
- Preserves clean command/query semantics
- Scales from local development to enterprise environments

It is intentionally not clever — and that is a feature.

