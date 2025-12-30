# Adding a REST API to Tee

This document provides a step-by-step guide for developers of all experience levels to add a RESTful API to the Tee project, while keeping the existing web application fully functional. The goal is to allow both browser-based users and programmatic clients (like mobile apps or other services) to interact with Tee.

## 1. What is a REST API?
A REST API (Representational State Transfer Application Programming Interface) allows clients to interact with your application by sending HTTP requests (like GET, POST, PUT, DELETE) and receiving data, usually in JSON format. This is different from serving HTML pages, which are meant for browsers.

## 2. Why Add a REST API?
- Enable integration with other systems, mobile apps, or JavaScript frontends (like React, Vue, etc.)
- Allow automation and scripting
- Support modern web development best practices

## 3. How Tee’s Structure Supports This
Tee is already organized into clear modules:
- **Domain logic** (business rules, data types)
- **Data access** (database queries)
- **Interface** (web routes, handlers)

This separation makes it easy to add new interfaces, like a REST API, without duplicating business logic.

## 4. Routing: Keeping Web and API Separate
You should keep web and API routes clearly separated. This avoids confusion and lets you return the right kind of response for each client.

- **Web app routes:** Serve HTML templates for browsers (e.g., `/tasks`, `/task/123`)
- **API routes:** Serve JSON for programmatic clients (e.g., `/api/tasks`, `/api/task/123`)

### Example (using `axum`):
```rust
use axum::{Router, routing::get};

let app = Router::new()
    .route("/tasks", get(web_tasks_handler)) // Web: HTML
    .route("/api/tasks", get(api_tasks_handler)); // API: JSON
```

## 5. Handlers: Web vs. API
Handlers are functions that process requests. You should write separate handlers for web and API endpoints, even if they use the same business logic.

- **Web handler:** Renders HTML using Askama templates.
- **API handler:** Returns JSON using `serde` serialization.

### Example:
```rust
// Web handler (for browsers)
async fn web_tasks_handler() -> impl IntoResponse {
    let tasks = get_tasks_from_db();
    HtmlTemplate(tasks) // Renders HTML
}

// API handler (for scripts, apps)
async fn api_tasks_handler() -> impl IntoResponse {
    let tasks = get_tasks_from_db();
    Json(tasks) // Returns JSON
}
```

## 6. Sharing Business Logic
Do not duplicate code! Place your core logic in shared modules (like `src/domain/task.rs`). Both web and API handlers should call these shared functions.

### Example:
```rust
// src/domain/task.rs
pub fn list_tasks() -> Vec<Task> {
    // ...fetch from DB...
}
```

## 7. Authentication: Web vs. API
- **Web:** Use session cookies (handled by the browser).
- **API:** Use tokens (like JWT) sent in HTTP headers.

### Example (API request):
```http
GET /api/tasks HTTP/1.1
Authorization: Bearer <token>
```

## 8. Error Handling: User-Friendly vs. Machine-Friendly
- **Web:** Show user-friendly error pages (HTML).
- **API:** Return structured error messages (JSON) that clients can parse.

### Example (API error response):
```json
{
  "error": "Task not found"
}
```

## 9. Step-by-Step: Adding a New API Endpoint
Suppose you want to add an endpoint to list all tasks at `GET /api/tasks`.

### Step 1: Define the API Handler
Create a new function that fetches tasks and returns them as JSON.
```rust
use axum::{extract::State, Json, response::IntoResponse};

async fn api_tasks_handler(State(state): State<AppState>) -> impl IntoResponse {
    let tasks = state.task_repo.list_tasks();
    Json(tasks)
}
```

### Step 2: Register the Route
Add the new route to your router setup:
```rust
Router::new().route("/api/tasks", get(api_tasks_handler));
```

### Step 3: Test the API
You can use `curl`, Postman, or any HTTP client:
```sh
curl -H "Authorization: Bearer <token>" http://localhost:8000/api/tasks
```
You should see a JSON array of tasks.

## 10. Documentation
Always document your API endpoints, expected inputs, and outputs. Create or update a file like `docs/API.md` with:
- Endpoint paths and methods
- Required headers (like Authorization)
- Example requests and responses
- Error formats

## 11. Security Considerations
- Always validate and sanitize input for API endpoints.
- Use HTTPS in production.
- Scope tokens to minimal privileges where possible.

## 12. Versioning and Stability
- Prefix API paths with a version when you expect future changes (e.g., `/api/v1/tasks`).
- Maintain backwards compatibility where possible; document breaking changes.

## 13. Pagination, Filtering, Sorting
For list endpoints, provide pagination, filtering, and sorting options to avoid returning huge payloads.

Example query parameters:
```
GET /api/tasks?page=2&page_size=50&sort=created_at:desc&status=open
```

## 14. Rate Limiting and Monitoring
- Consider rate-limiting API endpoints to protect from abuse.
- Add logging and monitoring for API usage.

## 15. Summary and Best Practices
- Keep web and API routes/handlers separate
- Share business logic between handlers
- Use appropriate authentication for each interface
- Return the right response type (HTML for web, JSON for API)
- Document your API for other developers

---

By following these steps, you can confidently extend Tee to support a robust REST API, making it accessible to both users and other software systems. If you want, I can also:

- Add a sample `docs/API.md` with endpoint references
- Create a minimal example of an authenticated API handler in `src/interface`

Tell me which of these you'd like next.

Additional machine-friendly artifacts:

- `docs/api-openapi.yaml` — OpenAPI spec for the task API (machine-readable contract; useful for codegen).
- `docs/API-Implementation-Guide.md` — Mapping of the OpenAPI spec to the existing project structure, domain types, validation rules, and test suggestions (documentation-only).

Use these files if you plan to run an OpenAPI code generator or to hand-implement the API with minimal ambiguity.
