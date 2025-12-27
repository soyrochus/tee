# Authentication and Sessions

The Tasks UI is protected by login. Authentication is handled in the interface
layer, while authorization rules are enforced in commands and queries.

## Where to look

- `docs/Authentication.md` - high level explanation.
- `docs/SPECS/SPEC-03-AUTH.md` - normative rules.
- `src/interface/auth.rs` - implementation.
- `src/data/auth_repo.rs` - SQL for users and sessions.

## The login flow

1. User visits `/auth/login` (GET) and sees a login form.
2. User submits credentials (POST).
3. Server verifies the password.
4. A new session is created and stored in the database.
5. The session token is set as a secure cookie.

This is a classic server-side session setup.

### Code example: session creation on login

From `src/interface/routes_web.rs`:

```rust
let user = user.expect("validated above");
let session_token = auth::generate_session_token();
let session_hash = auth::hash_token(&session_token);
let lifetime = if remember_me {
    st.auth.remember_me_lifetime
} else {
    st.auth.session_lifetime
};
let expires_at = chrono::Duration::from_std(lifetime)
    .map_err(|_| AppError::Internal("invalid session lifetime".to_string()))?;
let expires_at = Utc::now() + expires_at;
let session_id =
    auth_repo::insert_session(&st.pool, user.id, &session_hash, expires_at).await?;
tracing::info!(user_id = %user.id, session_id = %session_id, "login success");

let mut response = Redirect::to("/tasks").into_response();
let session_cookie = auth::session_cookie(&session_token, &st.auth, remember_me);
let login_cookie = auth::clear_login_csrf_cookie();
response
    .headers_mut()
    .append(SET_COOKIE, session_cookie.to_string().parse().unwrap());
response
    .headers_mut()
    .append(SET_COOKIE, login_cookie.to_string().parse().unwrap());
```

## Session security (important)

Session cookies follow strict rules:
- `__Host-` prefix.
- `Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/`.
- No Domain attribute.

Tokens are random, and only a hash is stored in the database.

Why hash the token?
- If the database is leaked, attackers cannot reuse session tokens directly.
- The raw token only exists in the user's browser cookie.

### Code example: session cookie settings

From `src/interface/auth.rs`:

```rust
pub fn session_cookie(token: &str, settings: &AuthSettings, remember_me: bool) -> Cookie<'static> {
    let max_age = if remember_me {
        settings.remember_me_lifetime
    } else {
        settings.session_lifetime
    };
    let max_age = cookie::time::Duration::seconds(max_age.as_secs() as i64);
    Cookie::build((SESSION_COOKIE_NAME, token.to_string()))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(max_age)
        .build()
}
```

## CSRF protection

All state-changing routes require CSRF validation.
The interface layer checks the token before calling commands.

This is required because the UI uses HTML forms (not a SPA).

## Principal and policy

Authenticated requests resolve to a `Principal` object. That principal is passed
into commands and queries, where policy functions decide what is allowed.

This keeps authentication and authorization separate, which is simpler to reason
about and easier to test.

## Exercise

- Follow the login route in `src/interface/routes_web.rs` and write down
  where the CSRF token is created and validated.
- Add a log line for successful login and logout.

Next: Internationalization.
