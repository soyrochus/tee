# Interface Layer

The interface layer is the "front door" of the system. It receives HTTP requests,
checks authentication and CSRF, parses user input, and renders responses.

In the Tee system, the interface layer must not talk directly to the database.
It calls commands and queries instead.

## Key files

- `src/interface/http.rs` - builds the main router and middleware.
- `src/interface/routes_web.rs` - HTML routes for the Tasks UI.
- `src/interface/auth.rs` - session auth and CSRF helpers.
- `src/interface/i18n.rs` - locale resolution and translation.
- `src/interface/error.rs` - error mapping for HTTP responses.
- `src/interface/state.rs` - shared state (DB pool, auth settings, translator).

## Router and middleware

`src/interface/http.rs` sets the baseline middleware:
- Request ID generation and propagation.
- Compression.
- Per-request timeout (10 seconds).
- Body size limit (1 MiB).

These are the minimum protections from `docs/Tee-Architecture-Guidelines.md`.

### Code example: router wiring

From `src/interface/routes_web.rs`:

```rust
Router::new()
    .route("/", get(|| async { Redirect::to("/tasks") }))
    .route("/auth/login", get(auth_login_form).post(auth_login_submit))
    .route("/auth/logout", axum::routing::post(auth_logout))
    .route("/auth/me", get(auth_me))
    .route("/i18n/locale", post(set_locale))
    .route("/tasks", get(tasks_list).post(tasks_create))
    .route("/tasks/new", get(task_new))
    .route("/tasks/:id", get(task_detail))
    .route("/tasks/:id/start", axum::routing::post(task_start))
    .route("/tasks/:id/complete", axum::routing::post(task_complete))
    .route(
        "/tasks/:id/update",
        axum::routing::post(task_update_details),
    )
    .route("/tasks/:id/delete", axum::routing::post(task_delete))
    .with_state(state)
```

### Code example: middleware stack

From `src/interface/http.rs`:

```rust
Router::new()
    .merge(routes_web::router(pool, auth, translator))
    .nest_service("/static", ServeDir::new("static"))
    .layer(TraceLayer::new_for_http())
    .layer(CompressionLayer::new())
    .layer(TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(10),
    ))
    .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1 MiB
```

## Route handlers

The web routes live in `src/interface/routes_web.rs`.
Each handler does boundary work only:
- Parse inputs.
- Check auth and CSRF.
- Call commands/queries.
- Render a template or redirect.

Example helper pattern:
```rust
let principal = match require_principal(&auth) {
    Ok(principal) => principal,
    Err(redirect) => return Ok(redirect.into_response()),
};
```

This keeps repetitive auth checks small and consistent.

## Async handlers and extractors (explained)

Routes are `async fn` because they can await I/O:
- Database calls.
- Template rendering.
- Auth lookups.

Axum extractors (like `State`, `Path`, `Query`, and `Form`) pull data from the
request and turn it into typed values. This keeps handlers clean and avoids
manual parsing.

## Errors at the boundary

Handlers return `Result<..., AppError>`. The `AppError` type is mapped to
status codes and basic text responses in `src/interface/error.rs`.

This is also where app-level errors are translated into HTTP responses.

## Why this layer matters

This layer protects the rest of the system from bad input. It is also the best
place to provide user-friendly errors and redirects.

## Exercise

- Add a new route `/tasks/:id/raw` that returns the task as JSON.
  (Hint: you will still call the query layer, but return `Json`.)

Next: Application Layer: Commands and Queries.
