# Observability and Errors

Observability is how you understand what the system is doing in production.
The Tee system keeps this simple and explicit.

## Logging and tracing

- `src/ops/observability.rs` sets up structured logging.
- Requests carry a request ID through the middleware stack.
- Commands log important lifecycle events (create, start, complete, delete).

This makes debugging much easier.

### Code example: logging initialization

From `src/ops/observability.rs`:

```rust
pub fn init(log_level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    fmt().with_env_filter(filter).init();
}
```

## Error mapping

The interface layer converts errors into HTTP responses:
- `403` for forbidden.
- `404` for not found.
- `409` for conflicts.
- `400` for bad requests.
- `500` for unexpected failures.

This mapping lives in `src/interface/error.rs` and uses the `AppErrorSource`
trait from `src/app/error.rs`.

### Code example: error to response mapping

From `src/interface/error.rs`:

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::AuthenticationRequired => {
                (StatusCode::UNAUTHORIZED, "authentication required").into_response()
            }
            AppError::CsrfViolation => (StatusCode::FORBIDDEN, "csrf violation").into_response(),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden").into_response(),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg).into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Template(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
            }
            AppError::Db(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response()
            }
            AppError::Command(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
        }
    }
}
```

## Why explicit errors matter

Users get consistent responses, and developers can quickly identify where
an error came from. It also makes testing easier.

## Exercise

- Add a log line for invalid task transitions.
- Add a counter metric for command failures (even if it is a placeholder).

Next: Development Workflow.
