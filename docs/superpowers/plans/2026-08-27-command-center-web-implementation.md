# Command Center Web Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a browser-based companion to the `task` CLI/TUI — a new `task_server` JSON API binary over the existing SQLite task store, and a React/Vite frontend implementing the "1a: Rail · Table · Inspector" shell from the imported design.

**Architecture:** `task_server` (new binary, `axum` + `tower-http`) talks directly to `db::load`/`db::save` and `Task`/`TaskFile` from the existing crate — no new persistence layer. Route handlers live in a new always-compiled library module `src/server.rs` (mirroring how `commands.rs` holds CLI logic); the binary itself is a thin `main()`. The frontend (`web/`, Vite + React + TypeScript) fetches from `task_server`'s JSON API and renders the table/inspector layout with real data; agent live-status is mocked per the spec's explicit deferral.

**Tech Stack:** Rust (`axum`, `tokio`, `tower-http`) for the backend; React + TypeScript + Vite + `lucide-react` for the frontend.

**Spec:** `docs/superpowers/specs/2026-08-27-command-center-web-design.md`

## Global Constraints

- `task_server` is not gated behind the `tui` feature — it must build and run with `cargo build` alone (no `--features tui`).
- `task_server` does not call into `commands::*` for anything that returns human-formatted `String` output — it returns structured JSON (`Task` values or small DTOs), reusing `db::load`/`db::save` and a shared filter/sort helper extracted from `commands::list`.
- No new persistence: `task_server` reads/writes the same `tasks.db` the CLI/TUI use, via the same `db::resolve_file_path`/`load`/`save` functions.
- No authentication; binds to `127.0.0.1` only; default port `4287`, overridable via `--port` flag or `TASK_SERVER_PORT` env var (same override pattern as `--file`/`TASK_FILE`).
- Agent live-status (running/waiting/idle) and the inspector's "Agent waiting" Q&A block are **mocked with static placeholder data** in the frontend — no backend endpoint for them in this plan. Agent *names* (from `GET /api/agents`) and a task's assigned `agent` field are real data.
- No in-browser note editing, no drag-and-drop, no keyboard shortcuts, no 1b board layout — see spec Non-goals.
- Design tokens (colors, fonts, spacing) are extracted into real CSS custom properties in `web/src/tokens.css`, not copied as literal inline hex/px values.

---

### Task 1: Fix `Task`'s JSON serialization to include `recurrence`

**Files:**
- Modify: `src/task.rs`

**Interfaces:**
- Produces: `Task`'s `Serialize` impl now includes `recurrence` as a string (e.g. `"weekly:fri"`) when present, omitted when `None` — needed by every later task that returns `Task` as JSON.

`Task` currently has `#[serde(skip_serializing)]` on `recurrence` — this was a compile-time necessity (the `Recurrence` enum has no `Serialize` derive, only `Display`/`FromStr`), not a deliberate design choice. Nothing has ever consumed `Task` as JSON over a real API before this plan, so this has never been exercised. Fix it using the same `serialize_with` pattern already used for `due_date`/`updated`.

- [ ] **Step 1: Write the failing test**

Add to `src/task.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn test_task_serialize_includes_recurrence_when_present() {
        let mut task = make_task(1, "Recurring task");
        task.recurrence = Some(Recurrence::from_str("weekly:fri").unwrap());
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"recurrence\":\"weekly:fri\""), "json was: {}", json);
    }

    #[test]
    fn test_task_serialize_omits_recurrence_when_none() {
        let task = make_task(1, "No recurrence");
        let json = serde_json::to_string(&task).unwrap();
        assert!(!json.contains("\"recurrence\""), "json was: {}", json);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib task::tests::test_task_serialize_includes_recurrence_when_present task::tests::test_task_serialize_omits_recurrence_when_none`
Expected: `test_task_serialize_includes_recurrence_when_present` FAILS (recurrence is currently absent even when set); `test_task_serialize_omits_recurrence_when_none` passes trivially (it's already absent) — that's fine, it's a regression guard for after the fix.

- [ ] **Step 3: Implement the fix**

In `src/task.rs`, change the `recurrence` field's attribute from:

```rust
    #[serde(skip_serializing)]
    pub recurrence: Option<Recurrence>,
```

to:

```rust
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_recurrence")]
    pub recurrence: Option<Recurrence>,
```

Add this function near the existing `serialize_option_date`/`serialize_option_datetime` functions:

```rust
fn serialize_option_recurrence<S>(r: &Option<Recurrence>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match r {
        Some(r) => serializer.serialize_str(&r.to_string()),
        None => serializer.serialize_none(),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib task::tests::test_task_serialize_includes_recurrence_when_present task::tests::test_task_serialize_omits_recurrence_when_none`
Expected: PASS.

Run: `cargo test --lib task::`
Expected: all existing `task.rs` tests still pass — this change is additive and shouldn't affect any other serialization test.

- [ ] **Step 5: Commit**

```bash
git add src/task.rs
git commit -m "fix: include recurrence in Task's JSON serialization"
```

---

### Task 2: Extract a shared filter/sort helper from `commands::list`

**Files:**
- Modify: `src/commands.rs`

**Interfaces:**
- Produces: `pub fn filter_and_sort_tasks<'a>(tasks: &'a [Task], status: Option<Status>, agent: Option<&str>, project: Option<&str>, tag: Option<&str>, due_before: Option<chrono::NaiveDate>) -> Vec<&'a Task>` — a pure function with no I/O, usable by both `commands::list` (CLI text output) and the new `task_server` JSON endpoint (Task 3), so the filter/sort logic is defined once.

This is a refactor with no behavior change — `commands::list`'s existing tests must keep passing unmodified.

- [ ] **Step 1: Write the failing test for the new function**

Add to `src/commands.rs`'s test module:

```rust
    #[test]
    fn test_filter_and_sort_tasks_by_status_and_agent() {
        let mut tf = crate::task::TaskFile::new();
        tf.tasks.push(sample_task_for_filter(1, "Open task", Status::Open, "bot"));
        tf.tasks.push(sample_task_for_filter(2, "Other agent", Status::Done, "human"));
        let result = filter_and_sort_tasks(&tf.tasks, Some(Status::Open), Some("bot"), None, None, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_filter_and_sort_tasks_sorts_by_due_then_priority() {
        let mut tf = crate::task::TaskFile::new();
        let mut t1 = sample_task_for_filter(1, "No due, critical", Status::Open, "bot");
        t1.priority = Priority::Critical;
        t1.due_date = None;
        let mut t2 = sample_task_for_filter(2, "Due today, low", Status::Open, "bot");
        t2.priority = Priority::Low;
        t2.due_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1);
        tf.tasks.push(t1);
        tf.tasks.push(t2);
        let result = filter_and_sort_tasks(&tf.tasks, None, None, None, None, None);
        // Due date ascending, None last — task 2 (has a due date) sorts before task 1 (no due date)
        assert_eq!(result[0].id, 2);
        assert_eq!(result[1].id, 1);
    }

    fn sample_task_for_filter(id: u32, title: &str, status: Status, agent: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            status,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            notes: Vec::new(),
            agent: Some(agent.to_string()),
            effort: None,
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test --lib commands::test_filter_and_sort_tasks 2>&1 | head -20`
Expected: FAIL — `cannot find function 'filter_and_sort_tasks'`.

- [ ] **Step 3: Extract the function and rewrite `list` to use it**

In `src/commands.rs`, replace the filtering/sorting block inside `pub fn list` with a call to a new function, and move the logic into that function:

```rust
pub fn filter_and_sort_tasks<'a>(
    tasks: &'a [Task],
    status: Option<Status>,
    agent: Option<&str>,
    project: Option<&str>,
    tag: Option<&str>,
    due_before: Option<chrono::NaiveDate>,
) -> Vec<&'a Task> {
    let mut result: Vec<&Task> = tasks
        .iter()
        .filter(|t| status.is_none_or(|s| t.status == s))
        .filter(|t| agent.is_none_or(|a| t.agent.as_deref() == Some(a)))
        .filter(|t| project.is_none_or(|p| t.project.as_deref() == Some(p)))
        .filter(|t| tag.is_none_or(|tag| t.tags.iter().any(|x| x == tag)))
        .filter(|t| due_before.is_none_or(|d| t.due_date.is_some_and(|td| td <= d)))
        .collect();

    result.sort_by(|a, b| {
        let date_cmp = match (a.due_date, b.due_date) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        };
        date_cmp.then(a.priority.cmp(&b.priority))
    });

    result
}
```

Now rewrite `pub fn list` to call it instead of duplicating the filter/sort logic:

```rust
pub fn list(path: &Path, args: ListArgs) -> Result<String, (i32, String)> {
    let task_file = db::load(path).map_err(|e| (1, e))?;
    let status_filter = args
        .status
        .as_deref()
        .map(Status::from_str)
        .transpose()
        .map_err(|e| (1, e))?;
    let due_before = args
        .due_before
        .as_deref()
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| (1, format!("Invalid date: '{}'. Expected YYYY-MM-DD.", s)))
        })
        .transpose()?;

    let tasks = filter_and_sort_tasks(
        &task_file.tasks,
        status_filter,
        args.agent.as_deref(),
        args.project.as_deref(),
        args.tag.as_deref(),
        due_before,
    );

    if tasks.is_empty() {
        return Ok("No matching tasks.".to_string());
    }
    let mut lines = Vec::new();
    for t in tasks {
        let due = t.due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
        let status = if t.status == Status::Done { "x" } else { " " };
        lines.push(format!("[{}] {:>4}  {:<8} {:<10}  {}", status, t.id, t.priority, due, t.title));
    }
    Ok(lines.join("\n"))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib commands::`
Expected: PASS — both new tests and every pre-existing `commands.rs` test (especially `test_list_filters_by_status_and_agent`, `test_list_no_matches`).

- [ ] **Step 5: Commit**

```bash
git add src/commands.rs
git commit -m "refactor: extract filter_and_sort_tasks helper for reuse by task_server"
```

---

### Task 3: `task_server` scaffold — dependencies, `AppState`, `GET /api/tasks`, `GET /api/tasks/:id`

**Files:**
- Modify: `Cargo.toml`
- Create: `src/server.rs`
- Create: `src/bin/task_server.rs`
- Modify: `src/lib.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/server.rs`

**Interfaces:**
- Produces:
  - `pub struct AppState { pub db_path: std::path::PathBuf }` (in `src/server.rs`)
  - `pub fn router(state: AppState) -> axum::Router` — builds the full route table; used by `src/bin/task_server.rs`'s `main()` and by this task's own tests (via `axum::body::Body`/`tower::ServiceExt::oneshot` or an actual bound test server — implementer's choice, see Step 3's test code for the exact pattern used).
  - `async fn list_tasks(...)`, `async fn get_task(...)` handlers.

- [ ] **Step 1: Add dependencies to `Cargo.toml`**

In `[dependencies]`, add:

```toml
axum = "0.7"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs"] }
```

Add the new binary entry:

```toml
[[bin]]
name = "task_server"
path = "src/bin/task_server.rs"
```

Run: `cargo build` (no `--features tui` — this binary must build without it)
Expected: builds successfully (no `task_server.rs` exists yet, so this just fetches the new dependencies; if `cargo build` complains about the missing bin path, that's expected until Step 4 creates it — run `cargo build --lib` instead to just fetch dependencies at this point).

- [ ] **Step 2: Register the module in `src/lib.rs`**

Add `pub mod server;` to `src/lib.rs` (not feature-gated — alongside `db`, `commands`, etc.):

```rust
pub mod auth;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod note;
pub mod parser;
pub mod server;
pub mod task;

#[cfg(feature = "tui")]
pub mod claude_session;
#[cfg(feature = "tui")]
pub mod todoist;
#[cfg(feature = "tui")]
pub mod tui;
```

- [ ] **Step 3: Write the failing tests**

Create `src/server.rs` with just imports and the test module first:

```rust
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::commands::filter_and_sort_tasks;
use crate::db;
use crate::task::{Status, Task};

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tempfile::tempdir;
    use tower::ServiceExt;

    fn make_state() -> (tempfile::TempDir, AppState) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        (dir, AppState { db_path })
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_list_tasks_returns_seeded_data() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1,
                title: "Seeded task".to_string(),
                status: Status::Open,
                priority: crate::task::Priority::High,
                tags: vec!["x".to_string()],
                created: chrono::Utc::now(),
                updated: None,
                description: None,
                due_date: None,
                project: None,
                recurrence: None,
                notes: Vec::new(),
                agent: None,
                effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json[0]["title"], "Seeded task");
        assert_eq!(json[0]["id"], 1);
    }

    #[tokio::test]
    async fn test_list_tasks_filters_by_status_query_param() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            let mut t1 = crate::task::Task {
                id: 1, title: "Open".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
            };
            let mut t2 = t1.clone();
            t2.id = 2; t2.title = "Done".to_string(); t2.status = Status::Done;
            tf.tasks.push(t1);
            tf.tasks.push(t2);
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks?status=open").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = body_json(response).await;
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["title"], "Open");
    }

    #[tokio::test]
    async fn test_get_task_by_id() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 7, title: "Found me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks/7").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Found me");
    }

    #[tokio::test]
    async fn test_get_task_missing_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks/999").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
```

Note: `crate::task::Task` doesn't derive `Clone`... check — actually `Task` DOES derive `Clone` already (`#[derive(Debug, Clone, PartialEq, Serialize)]`), so `t1.clone()` in the third test works fine.

- [ ] **Step 2: Run tests to verify they fail to compile**

Run: `cargo test --lib server:: 2>&1 | head -30`
Expected: FAIL — `router`, `list_tasks`, `get_task` don't exist yet.

- [ ] **Step 3: Implement `router`, `list_tasks`, `get_task`**

Add above the `#[cfg(test)]` module in `src/server.rs`:

```rust
#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub agent: Option<String>,
    pub project: Option<String>,
    pub tag: Option<String>,
    pub due_before: Option<String>,
}

fn app_error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": message.into() })))
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Task>>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let status = query
        .status
        .as_deref()
        .map(Status::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let due_before = query
        .due_before
        .as_deref()
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date: '{}'. Expected YYYY-MM-DD.", s))
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;

    let tasks: Vec<Task> = filter_and_sort_tasks(
        &task_file.tasks,
        status,
        query.agent.as_deref(),
        query.project.as_deref(),
        query.tag.as_deref(),
        due_before,
    )
    .into_iter()
    .cloned()
    .collect();

    Ok(Json(tasks))
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    task_file
        .find_task(id)
        .cloned()
        .map(Json)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))
}

pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks/:id", get(get_task))
        .with_state(state)
}
```

Add `impl IntoResponse` is not needed since `(StatusCode, Json<Value>)` already implements `IntoResponse` via axum's blanket impls for tuples of `(StatusCode, T: IntoResponse)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib server::`
Expected: PASS — all 5 tests green.

- [ ] **Step 5: Create the thin binary**

Create `src/bin/task_server.rs`:

```rust
use task::server::{router, AppState};

#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);

    println!("task_server: serving {} on http://127.0.0.1:{}", db_path.display(), port);

    let app = router(AppState { db_path });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

(This is a minimal `--file`-equivalent via positional arg for now — Task 7 revisits argument parsing when static file serving is added, to decide whether to bring in `clap` for `task_server` too or keep it minimal. Note this for that task.)

- [ ] **Step 6: Build and smoke-test manually**

Run: `cargo build --bin task_server`
Expected: builds successfully.

Run: `cargo run --bin task_server -- /tmp/smoke-test-tasks.db &` then `curl http://127.0.0.1:4287/api/tasks` — expect `[]`. Kill the background process afterward (`kill %1` or find/kill by port).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/server.rs src/bin/task_server.rs
git commit -m "feat: scaffold task_server with GET /api/tasks and GET /api/tasks/:id"
```

---

### Task 4: `POST /api/tasks` (add) and `GET /api/agents`

**Files:**
- Modify: `src/server.rs`

**Interfaces:**
- Consumes: `crate::commands::{add, AddArgs}` is NOT used here (it returns a formatted string) — this task re-implements the add logic directly against `TaskFile`, mirroring `commands::add`'s body exactly but returning the created `Task` as JSON. `crate::config::list_agent_profiles() -> Vec<(String, String)>` (existing, unchanged).
- Produces: `async fn add_task(...)`, `async fn list_agents(...)`; `#[derive(Deserialize)] pub struct AddTaskRequest`; `#[derive(Serialize)] pub struct AgentProfile { name: String, dir: String }`. Router gains `POST /api/tasks` and `GET /api/agents`.

- [ ] **Step 1: Write the failing tests**

Add to `src/server.rs`'s test module:

```rust
    #[tokio::test]
    async fn test_add_task_creates_and_returns_task() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "New task", "priority": "high" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "New task");
        assert_eq!(json["priority"], "high");
        assert_eq!(json["id"], 1);
    }

    #[tokio::test]
    async fn test_add_task_invalid_priority_returns_400() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "Bad", "priority": "urgent" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_agents_returns_configured_profiles() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "agent-bot: /code/bot\n").unwrap();
        unsafe { std::env::set_var("TASK_CONFIG_FILE", &config_path) };
        let app = router(AppState { db_path });
        let response = app
            .oneshot(Request::builder().uri("/api/agents").body(Body::empty()).unwrap())
            .await
            .unwrap();
        unsafe { std::env::remove_var("TASK_CONFIG_FILE") };
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json[0]["name"], "bot");
        assert_eq!(json[0]["dir"], "/code/bot");
    }
```

- [ ] **Step 2: Run tests to verify they fail to compile**

Run: `cargo test --lib server::test_add_task server::test_list_agents 2>&1 | head -20`
Expected: FAIL — handlers/types don't exist.

- [ ] **Step 3: Implement**

Add to `src/server.rs`:

```rust
#[derive(Deserialize)]
pub struct AddTaskRequest {
    pub title: String,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct AgentProfile {
    pub name: String,
    pub dir: String,
}

fn parse_csv(s: Option<&str>) -> Vec<String> {
    s.unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

async fn add_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let priority_str = req.priority.as_deref().unwrap_or("medium");
    let priority = crate::task::Priority::from_str(priority_str)
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let today = chrono::Local::now().date_naive();
    let due_date = req.due.as_deref().and_then(|d| crate::parser::parse_due_date_input(d, today));

    let id = task_file.next_id;
    task_file.next_id += 1;
    let new_task = Task {
        id,
        title: req.title,
        status: Status::Open,
        priority,
        tags: parse_csv(req.tags.as_deref()),
        created: chrono::Utc::now(),
        updated: None,
        description: req.description,
        due_date,
        project: req.project,
        recurrence: None,
        notes: Vec::new(),
        agent: req.agent,
        effort: None,
    };
    task_file.tasks.push(new_task.clone());
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(new_task))
}

async fn list_agents() -> Json<Vec<AgentProfile>> {
    let profiles = crate::config::list_agent_profiles()
        .into_iter()
        .map(|(name, dir)| AgentProfile { name, dir })
        .collect();
    Json(profiles)
}
```

Update `router` to add the new routes:

```rust
pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", get(get_task))
        .route("/api/agents", get(list_agents))
        .with_state(state)
}
```

Add `use axum::routing::post;` — actually `post` is used as a method on the route builder returned by `get(...)`, not a separate import for this line (axum's `MethodRouter` supports `.post(handler)` chaining directly on the result of `get(handler)`), so no new import is needed for the `/api/tasks` line. `list_agents` still needs `get(list_agents)` which uses the already-imported `get`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib server::`
Expected: PASS — all tests including the 3 new ones and everything from Task 3.

- [ ] **Step 5: Commit**

```bash
git add src/server.rs
git commit -m "feat: add POST /api/tasks and GET /api/agents to task_server"
```

---

### Task 5: `PATCH /api/tasks/:id` (edit)

**Files:**
- Modify: `src/server.rs`

**Interfaces:**
- Produces: `async fn edit_task(...)`, `#[derive(Deserialize)] pub struct EditTaskRequest`. Router's `/api/tasks/:id` route gains `.patch(edit_task)`.

- [ ] **Step 1: Write the failing tests**

Add to `src/server.rs`'s test module:

```rust
    #[tokio::test]
    async fn test_edit_task_updates_only_given_fields() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Original".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "priority": "critical" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Original");
        assert_eq!(json["priority"], "critical");
        assert!(json["updated"].is_string());
    }

    #[tokio::test]
    async fn test_edit_task_missing_id_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "x" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/999")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib server::test_edit_task 2>&1 | head -20`
Expected: FAIL — `edit_task` doesn't exist, route not registered.

- [ ] **Step 3: Implement**

Add to `src/server.rs`:

```rust
#[derive(Deserialize)]
pub struct EditTaskRequest {
    pub title: Option<String>,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub effort: Option<String>,
}

async fn edit_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<EditTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let today = chrono::Local::now().date_naive();
    let priority = req
        .priority
        .as_deref()
        .map(crate::task::Priority::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let effort = req
        .effort
        .as_deref()
        .map(crate::task::Effort::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let due_date = req
        .due
        .as_deref()
        .map(|d| {
            crate::parser::parse_due_date_input(d, today)
                .ok_or_else(|| format!("Invalid due date: '{}'", d))
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let tags = req.tags.as_deref().map(|s| parse_csv(Some(s)));

    let updated_task = {
        let t = task_file
            .find_task_mut(id)
            .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
        if let Some(title) = req.title {
            t.title = title;
        }
        if let Some(p) = priority {
            t.priority = p;
        }
        if let Some(d) = due_date {
            t.due_date = Some(d);
        }
        if let Some(p) = req.project {
            t.project = Some(p);
        }
        if let Some(tg) = tags {
            t.tags = tg;
        }
        if let Some(a) = req.agent {
            t.agent = Some(a);
        }
        if let Some(d) = req.description {
            t.description = Some(d);
        }
        if let Some(e) = effort {
            t.effort = Some(e);
        }
        t.updated = Some(chrono::Utc::now());
        t.clone()
    };

    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task))
}
```

Update the `/api/tasks/:id` route registration in `router`:

```rust
        .route("/api/tasks/:id", get(get_task).patch(edit_task))
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib server::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/server.rs
git commit -m "feat: add PATCH /api/tasks/:id to task_server"
```

---

### Task 6: `POST /api/tasks/:id/done` (with recurrence spawn) and `POST /api/tasks/:id/reopen`

**Files:**
- Modify: `src/server.rs`

**Interfaces:**
- Produces: `async fn done_task(...)`, `async fn reopen_task(...)`, `#[derive(Serialize)] pub struct DoneResponse { completed: Task, spawned: Option<Task> }`. Router gains `POST /api/tasks/:id/done` and `POST /api/tasks/:id/reopen`.

This mirrors `commands::done`'s recurrence-spawn logic exactly (same logic already reviewed and shipped in `commands.rs` — re-implemented here because the return shape is structured JSON, not a formatted string).

- [ ] **Step 1: Write the failing tests**

Add to `src/server.rs`'s test module:

```rust
    #[tokio::test]
    async fn test_done_task_marks_status_done() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Finish me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().method("POST").uri("/api/tasks/1/done").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["completed"]["status"], "done");
        assert!(json["spawned"].is_null());
    }

    #[tokio::test]
    async fn test_done_task_recurring_spawns_next_occurrence() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recurring".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: None, effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().method("POST").uri("/api/tasks/1/done").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["completed"]["status"], "done");
        assert_eq!(json["spawned"]["id"], 2);
        assert_eq!(json["spawned"]["due_date"], "2026-01-12");
    }

    #[tokio::test]
    async fn test_reopen_task_sets_status_open() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Reopen me".to_string(), status: Status::Done, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().method("POST").uri("/api/tasks/1/reopen").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "open");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib server::test_done_task server::test_reopen_task 2>&1 | head -20`
Expected: FAIL — handlers/routes don't exist.

- [ ] **Step 3: Implement**

Add to `src/server.rs`:

```rust
#[derive(Serialize)]
pub struct DoneResponse {
    pub completed: Task,
    pub spawned: Option<Task>,
}

async fn done_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<DoneResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let idx = task_file
        .tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;

    task_file.tasks[idx].status = Status::Done;
    task_file.tasks[idx].updated = Some(chrono::Utc::now());

    let mut spawned = None;
    if let Some(recur) = task_file.tasks[idx].recurrence {
        let parent = task_file.tasks[idx].clone();
        let next_due = crate::task::next_due_date(&recur, parent.due_date);
        let new_id = task_file.next_id;
        task_file.next_id += 1;
        let new_task = Task {
            id: new_id,
            title: parent.title.clone(),
            status: Status::Open,
            priority: parent.priority,
            tags: parent.tags.clone(),
            created: chrono::Utc::now(),
            updated: None,
            description: parent.description.clone(),
            due_date: Some(next_due),
            project: parent.project.clone(),
            recurrence: Some(recur),
            notes: parent.notes.clone(),
            agent: parent.agent.clone(),
            effort: parent.effort,
        };
        task_file.tasks.push(new_task.clone());
        spawned = Some(new_task);
    }

    let completed = task_file.tasks[idx].clone();
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(DoneResponse { completed, spawned }))
}

async fn reopen_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let updated_task = {
        let t = task_file
            .find_task_mut(id)
            .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
        t.status = Status::Open;
        t.updated = Some(chrono::Utc::now());
        t.clone()
    };
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task))
}
```

Update `router`:

```rust
        .route("/api/tasks/:id/done", axum::routing::post(done_task))
        .route("/api/tasks/:id/reopen", axum::routing::post(reopen_task))
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib server::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/server.rs
git commit -m "feat: add POST /api/tasks/:id/done and /reopen to task_server"
```

---

### Task 7: `DELETE /api/tasks/:id` and static file serving

**Files:**
- Modify: `src/server.rs`
- Modify: `src/bin/task_server.rs`

**Interfaces:**
- Produces: `async fn delete_task(...)`. Router gains `DELETE /api/tasks/:id` and a fallback service serving `web/dist` as static files.

- [ ] **Step 1: Write the failing tests**

Add to `src/server.rs`'s test module:

```rust
    #[tokio::test]
    async fn test_delete_task_removes_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Doomed".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let db_path = state.db_path.clone();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().method("DELETE").uri("/api/tasks/1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let tf = db::load(&db_path).unwrap();
        assert!(tf.tasks.is_empty());
    }

    #[tokio::test]
    async fn test_delete_task_missing_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().method("DELETE").uri("/api/tasks/999").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib server::test_delete_task 2>&1 | head -20`
Expected: FAIL — `delete_task` doesn't exist.

- [ ] **Step 3: Implement `delete_task`**

Add to `src/server.rs`:

```rust
async fn delete_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    task_file
        .remove_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}
```

Update the `/api/tasks/:id` route registration:

```rust
        .route("/api/tasks/:id", get(get_task).patch(edit_task).delete(delete_task))
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib server::`
Expected: PASS — all `server.rs` tests green (13 tests total across Tasks 3-7).

- [ ] **Step 5: Add static file serving to the router, parameterized by a frontend dist path**

Change `router`'s signature to accept an optional static directory, so tests don't need a real `web/dist` to exist:

```rust
pub fn router(state: AppState) -> Router {
    router_with_static(state, None)
}

pub fn router_with_static(state: AppState, static_dir: Option<std::path::PathBuf>) -> Router {
    let state = Arc::new(state);
    let mut app = Router::new()
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", get(get_task).patch(edit_task).delete(delete_task))
        .route("/api/tasks/:id/done", axum::routing::post(done_task))
        .route("/api/tasks/:id/reopen", axum::routing::post(reopen_task))
        .route("/api/agents", get(list_agents))
        .with_state(state);

    if let Some(dir) = static_dir {
        app = app.fallback_service(tower_http::services::ServeDir::new(dir));
    }

    app
}
```

- [ ] **Step 6: Wire the static dir into `src/bin/task_server.rs`**

Update `src/bin/task_server.rs`'s `main()` to pass a static directory (resolved relative to the binary's location or a fixed `web/dist` relative to the crate root for now — simplest correct option for local development is resolving relative to the current working directory, since this binary is expected to be run from the repo root):

```rust
use task::server::{router_with_static, AppState};

#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);
    let static_dir = std::path::PathBuf::from("web/dist");
    let static_dir = if static_dir.exists() { Some(static_dir) } else { None };

    println!("task_server: serving {} on http://127.0.0.1:{}", db_path.display(), port);
    if static_dir.is_none() {
        println!("task_server: web/dist not found — API only, no static frontend (run `npm run build` in web/ first)");
    }

    let app = router_with_static(AppState { db_path }, static_dir);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 7: Run the full test suite**

Run: `cargo build && cargo test --lib server:: commands:: task::`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/server.rs src/bin/task_server.rs
git commit -m "feat: add DELETE /api/tasks/:id and static frontend serving to task_server"
```

---

### Task 8: Frontend scaffold — Vite + React + TypeScript, design tokens, static shell layout

**Files:**
- Create: `web/` (Vite scaffold — many generated files, not enumerated individually)
- Create: `web/src/tokens.css`
- Create: `web/src/App.tsx` (replacing the Vite template's default)
- Create: `web/src/components/Button.tsx`
- Modify: `web/src/main.tsx` (Vite-generated, just needs the tokens.css import)
- Create: `web/public/fonts/` (copy `.ttf` files from the design project)

**Interfaces:**
- Produces: a running `npm run dev` frontend rendering a static (no real data yet) version of the 1a layout: left rail, header bar, table column headers + a couple of hardcoded sample rows, empty-state inspector. Real data wiring is Task 9.

- [ ] **Step 1: Scaffold the Vite project**

From the repo root:

```bash
npm create vite@latest web -- --template react-ts
cd web
npm install
npm install lucide-react
```

- [ ] **Step 2: Fetch the design system's font files via DesignSync**

Using the DesignSync MCP tool (`projectId: 22cad86b-ec0d-42a9-9369-75d9fc346cd3`), fetch these two files (the only weights actually used in the mockup — DM Sans for headings, Open Sans for body text) and save them locally:

```
_ds/itential-design-system-82009cd1-16ea-46ac-896f-492c319727d2/fonts/DMSans-VariableFont.ttf
_ds/itential-design-system-82009cd1-16ea-46ac-896f-492c319727d2/fonts/OpenSans-Regular.ttf
```

`get_file` returns content as a string — for binary `.ttf` files, request them and write the returned bytes to `web/public/fonts/DMSans-VariableFont.ttf` and `web/public/fonts/OpenSans-Regular.ttf` respectively (create the `web/public/fonts/` directory first). If `get_file`'s 256 KiB cap or encoding makes a `.ttf` awkward to transfer this way, fall back to `system-ui` / `-apple-system` in the font stack for this task and note it as a concern in your report — font fidelity is cosmetic, not load-bearing for the task.

- [ ] **Step 3: Write `web/src/tokens.css`**

```css
@font-face {
  font-family: 'DM Sans';
  src: url('/fonts/DMSans-VariableFont.ttf') format('truetype');
  font-weight: 100 900;
}

@font-face {
  font-family: 'Open Sans';
  src: url('/fonts/OpenSans-Regular.ttf') format('truetype');
  font-weight: 400;
}

:root {
  /* Surfaces */
  --ink: #00001E;
  --ink-2: #020224;
  --ink-3: #09163B;

  /* Hairlines */
  --hairline: rgba(255, 255, 255, 0.10);
  --hairline-soft: rgba(255, 255, 255, 0.06);

  /* Foreground (descending opacity) */
  --fg-1: #FFFFFF;
  --fg-2: rgba(255, 255, 255, 0.85);
  --fg-3: rgba(255, 255, 255, 0.65);
  --fg-4: rgba(255, 255, 255, 0.5);
  --fg-5: rgba(255, 255, 255, 0.3);

  /* Brand / semantic */
  --magenta: #FF0095;
  --cyan: #0099FF;
  --teal: #00F3DB;
  --citrine: #F5C518;
  --danger: #E5484D;

  /* Type */
  --font-display: 'DM Sans', system-ui, sans-serif;
  --font-body: 'Open Sans', system-ui, sans-serif;
  --font-mono: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;

  /* Spacing (4px base) */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-6: 24px;
  --space-8: 32px;

  /* Radius */
  --radius-xs: 2px;
  --radius-sm: 5px;
  --radius-md: 8px;
  --radius-pill: 999px;
}

* { box-sizing: border-box; }

html, body, #root {
  margin: 0;
  height: 100%;
  background: var(--ink);
  font-family: var(--font-body);
  color: var(--fg-1);
}

a { color: var(--magenta); text-decoration: none; }
a:hover { color: #CC0077; }
```

- [ ] **Step 4: Write `web/src/components/Button.tsx`**

```tsx
import type { ReactNode, CSSProperties } from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'cyan' | 'ghost';
type ButtonSize = 'sm' | 'md' | 'lg';

const sizeStyles: Record<ButtonSize, CSSProperties> = {
  sm: { height: 34, padding: '0 14px', fontSize: 13, gap: 6 },
  md: { height: 40, padding: '0 18px', fontSize: 14, gap: 8 },
  lg: { height: 48, padding: '0 24px', fontSize: 16, gap: 10 },
};

const variantStyles: Record<ButtonVariant, CSSProperties> = {
  primary: { background: 'var(--magenta)', color: 'var(--ink)', border: '1px solid transparent' },
  secondary: { background: 'transparent', color: 'var(--fg-1)', border: '1px solid var(--hairline)' },
  cyan: { background: 'var(--cyan)', color: 'var(--ink)', border: '1px solid transparent' },
  ghost: { background: 'transparent', color: 'var(--magenta)', border: '1px solid transparent' },
};

interface ButtonProps {
  children: ReactNode;
  variant?: ButtonVariant;
  size?: ButtonSize;
  onClick?: () => void;
  disabled?: boolean;
}

export function Button({ children, variant = 'primary', size = 'sm', onClick, disabled }: ButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        borderRadius: 'var(--radius-sm)',
        fontFamily: 'var(--font-display)',
        fontWeight: 600,
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.5 : 1,
        ...sizeStyles[size],
        ...variantStyles[variant],
      }}
    >
      {children}
    </button>
  );
}
```

- [ ] **Step 5: Write a static-layout `web/src/App.tsx`**

This renders the three-region layout (rail / table / inspector) with hardcoded placeholder content, proving the build and CSS work before wiring real data in Task 9:

```tsx
import './tokens.css';
import { Terminal, Sun, CalendarDays, Settings } from 'lucide-react';
import { Button } from './components/Button';

export default function App() {
  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      <aside
        style={{
          width: 252,
          flex: 'none',
          background: 'var(--ink-2)',
          borderRight: '1px solid var(--hairline)',
          display: 'flex',
          flexDirection: 'column',
          padding: '20px 0',
        }}
      >
        <div style={{ padding: '0 20px 22px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div
            style={{
              width: 26,
              height: 26,
              borderRadius: 5,
              background: 'var(--magenta)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 14 }}>
            command center
          </span>
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <Sun size={16} color="var(--magenta)" />
            <span style={{ flex: 1, fontSize: 14 }}>Today</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <CalendarDays size={16} color="var(--fg-4)" />
            <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>This week</span>
          </div>
        </nav>
        <div style={{ marginTop: 'auto', padding: '14px 20px 0', borderTop: '1px solid var(--hairline-soft)', display: 'flex', alignItems: 'center', gap: 10 }}>
          <Settings size={16} color="var(--fg-4)" />
          <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>Settings</span>
        </div>
      </aside>

      <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <header
          style={{
            height: 72,
            flex: 'none',
            padding: '0 24px',
            borderBottom: '1px solid var(--hairline)',
            display: 'flex',
            alignItems: 'center',
            gap: 16,
          }}
        >
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22 }}>Today</span>
          <div style={{ flex: 1 }} />
          <Button>New task</Button>
        </header>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 16,
            height: 34,
            flex: 'none',
            padding: '0 24px',
            borderBottom: '1px solid var(--hairline-soft)',
            fontFamily: 'var(--font-display)',
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: '0.06em',
            textTransform: 'uppercase',
            color: 'var(--fg-5)',
          }}
        >
          <span style={{ width: 34 }}>ID</span>
          <span style={{ width: 78 }}>Priority</span>
          <span style={{ flex: 1 }}>Task</span>
          <span style={{ width: 96 }}>Due</span>
          <span style={{ width: 44 }}>Effort</span>
        </div>
        <div style={{ flex: 1, minHeight: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--fg-5)' }}>
          (tasks load here — Task 9)
        </div>
      </main>

      <aside
        style={{
          width: 352,
          flex: 'none',
          background: 'var(--ink-3)',
          borderLeft: '1px solid var(--hairline)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--fg-5)',
        }}
      >
        Select a task
      </aside>
    </div>
  );
}
```

- [ ] **Step 6: Simplify `web/src/main.tsx`** (remove Vite template boilerplate CSS imports that no longer apply)

Ensure it's just:

```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

Delete the Vite template's default `web/src/App.css` and `web/src/index.css` if present (superseded by `tokens.css`).

- [ ] **Step 7: Verify it builds and runs**

Run: `cd web && npm run build`
Expected: builds successfully, produces `web/dist/`.

Run: `cd web && npm run dev` (manually verify in a browser at the printed localhost URL, then stop the dev server) — expected: dark background, left rail with logo/nav, header with "New task" button, empty center message, "Select a task" on the right. This is a manual visual check; note in your report whether you were able to actually view it in a browser or only confirm the build/dev-server started without errors.

- [ ] **Step 8: Commit**

```bash
git add web/
git commit -m "feat: scaffold Command Center Web frontend with static 1a layout"
```

(The `web/node_modules/` directory must not be committed — confirm the Vite scaffold's generated `.gitignore` inside `web/` already excludes it before running `git add`; if `web/` doesn't have its own `.gitignore` excluding `node_modules` and `dist`, add one before staging.)

---

### Task 9: Wire real data — `api.ts` client, task list, agent grouping, due-window counts

**Files:**
- Create: `web/src/api.ts`
- Create: `web/src/types.ts`
- Modify: `web/src/App.tsx`
- Create: `web/vite.config.ts` modification (dev proxy)

**Interfaces:**
- Produces: `export interface Task { id: number; title: string; status: 'open' | 'done'; priority: 'critical' | 'high' | 'medium' | 'low'; tags: string[]; created: string; updated?: string; description?: string; due_date?: string; project?: string; recurrence?: string; notes: string[]; agent?: string; effort?: 'high' | 'medium' | 'low'; }` (in `types.ts`); `export async function fetchTasks(): Promise<Task[]>`, `export async function fetchAgents(): Promise<{name: string, dir: string}[]>` (in `api.ts`); `App.tsx` now fetches and renders real data grouped by agent.

- [ ] **Step 1: Configure the dev proxy**

Modify `web/vite.config.ts` to add a proxy so `fetch('/api/...')` works identically in dev and production:

```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:4287',
    },
  },
});
```

- [ ] **Step 2: Write `web/src/types.ts`**

```ts
export type Priority = 'critical' | 'high' | 'medium' | 'low';
export type Effort = 'high' | 'medium' | 'low';
export type Status = 'open' | 'done';

export interface Task {
  id: number;
  title: string;
  status: Status;
  priority: Priority;
  tags: string[];
  created: string;
  updated?: string;
  description?: string;
  due_date?: string;
  project?: string;
  recurrence?: string;
  notes: string[];
  agent?: string;
  effort?: Effort;
}

export interface AgentProfile {
  name: string;
  dir: string;
}
```

- [ ] **Step 3: Write `web/src/api.ts`**

```ts
import type { Task, AgentProfile } from './types';

async function jsonOrThrow<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
  return response.json();
}

export async function fetchTasks(): Promise<Task[]> {
  return jsonOrThrow(await fetch('/api/tasks'));
}

export async function fetchAgents(): Promise<AgentProfile[]> {
  return jsonOrThrow(await fetch('/api/agents'));
}

export async function addTask(input: {
  title: string;
  priority?: string;
  due?: string;
  project?: string;
  tags?: string;
  agent?: string;
  description?: string;
}): Promise<Task> {
  return jsonOrThrow(
    await fetch('/api/tasks', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    }),
  );
}

export async function editTask(id: number, changes: Partial<{
  title: string; priority: string; due: string; project: string; tags: string; agent: string; description: string; effort: string;
}>): Promise<Task> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${id}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
}

export async function markDone(id: number): Promise<{ completed: Task; spawned: Task | null }> {
  return jsonOrThrow(await fetch(`/api/tasks/${id}/done`, { method: 'POST' }));
}

export async function reopenTask(id: number): Promise<Task> {
  return jsonOrThrow(await fetch(`/api/tasks/${id}/reopen`, { method: 'POST' }));
}

export async function deleteTask(id: number): Promise<void> {
  const response = await fetch(`/api/tasks/${id}`, { method: 'DELETE' });
  if (!response.ok && response.status !== 204) {
    throw new Error(`Failed to delete task ${id}`);
  }
}
```

- [ ] **Step 4: Write a due-window count helper**

Create `web/src/dueWindow.ts`. v1's sidebar shows a single "All open" count (matching the simplified single-nav-item sidebar built in Step 5) — this is the only bucket actually wired up in this task; per-window buckets (Today/This week/This month/This year) are a follow-up, not built here:

```ts
import type { Task } from './types';

export function countAllOpen(tasks: Task[]): number {
  return tasks.filter((t) => t.status !== 'done').length;
}
```

Since this plan didn't set up a frontend test runner, this helper is exercised only manually for now (see Step 6) — do not add a testing framework as part of this task; that's a reasonable follow-up, not required for v1. Note this explicitly as a concern in your report rather than silently skipping test coverage.

- [ ] **Step 5: Wire real data into `App.tsx`**

Rewrite `web/src/App.tsx` to fetch on mount and render tasks grouped by agent:

```tsx
import { useEffect, useMemo, useState } from 'react';
import './tokens.css';
import { Terminal, Sun, CalendarDays, Settings } from 'lucide-react';
import { Button } from './components/Button';
import { fetchTasks, fetchAgents } from './api';
import type { Task, AgentProfile } from './types';
import { countAllOpen } from './dueWindow';

function groupByAgent(tasks: Task[]): Map<string, Task[]> {
  const groups = new Map<string, Task[]>();
  for (const t of tasks) {
    const key = t.agent ?? 'unassigned';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(t);
  }
  return groups;
}

const priorityColor: Record<string, string> = {
  critical: 'var(--danger)',
  high: 'var(--citrine)',
  medium: 'var(--fg-3)',
  low: 'var(--fg-5)',
};

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [selected, setSelected] = useState<Task | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([fetchTasks(), fetchAgents()])
      .then(([t, a]) => {
        setTasks(t);
        setAgents(a);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const grouped = useMemo(() => groupByAgent(tasks), [tasks]);
  const todayCount = useMemo(() => countAllOpen(tasks), [tasks]);

  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      <aside
        style={{
          width: 252, flex: 'none', background: 'var(--ink-2)',
          borderRight: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column', padding: '20px 0',
        }}
      >
        <div style={{ padding: '0 20px 22px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{ width: 26, height: 26, borderRadius: 5, background: 'var(--magenta)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 14 }}>command center</span>
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <Sun size={16} color="var(--magenta)" />
            <span style={{ flex: 1, fontSize: 14 }}>All open</span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--magenta)' }}>{todayCount}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <CalendarDays size={16} color="var(--fg-4)" />
            <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>Agents</span>
          </div>
        </nav>
        <div style={{ padding: '8px 20px', display: 'flex', flexDirection: 'column', gap: 4 }}>
          {agents.map((a) => (
            <div key={a.name} style={{ display: 'flex', alignItems: 'center', gap: 10, height: 28 }}>
              <span style={{ width: 7, height: 7, borderRadius: 999, background: 'var(--fg-5)' }} />
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)' }}>{a.name}</span>
            </div>
          ))}
        </div>
        <div style={{ marginTop: 'auto', padding: '14px 20px 0', borderTop: '1px solid var(--hairline-soft)', display: 'flex', alignItems: 'center', gap: 10 }}>
          <Settings size={16} color="var(--fg-4)" />
          <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>Settings</span>
        </div>
      </aside>

      <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <header style={{ height: 72, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline)', display: 'flex', alignItems: 'center', gap: 16 }}>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22 }}>All tasks</span>
          <div style={{ flex: 1 }} />
          <Button>New task</Button>
        </header>
        {error && <div style={{ padding: 16, color: 'var(--danger)' }}>{error}</div>}
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, height: 34, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline-soft)', fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--fg-5)' }}>
          <span style={{ width: 34 }}>ID</span>
          <span style={{ width: 78 }}>Priority</span>
          <span style={{ flex: 1 }}>Task</span>
          <span style={{ width: 96 }}>Due</span>
          <span style={{ width: 44 }}>Effort</span>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
          {[...grouped.entries()].map(([agentName, agentTasks]) => (
            <div key={agentName}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 38, padding: '0 24px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, fontWeight: 600 }}>{agentName}</span>
                <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>{agentTasks.length} tasks</span>
              </div>
              {agentTasks.map((t) => (
                <div
                  key={t.id}
                  onClick={() => setSelected(t)}
                  style={{
                    display: 'flex', alignItems: 'center', gap: 16, height: 46, padding: '0 24px',
                    borderBottom: '1px solid var(--hairline-soft)', cursor: 'pointer',
                    background: selected?.id === t.id ? 'rgba(255,0,149,0.08)' : 'transparent',
                  }}
                >
                  <span style={{ width: 34, fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-5)' }}>{t.id}</span>
                  <span style={{ width: 78, fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, textTransform: 'uppercase', color: priorityColor[t.priority] }}>
                    {t.priority}
                  </span>
                  <span style={{ flex: 1, minWidth: 0, fontSize: 14.5, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    {t.title}
                  </span>
                  <span style={{ width: 96, fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)' }}>
                    {t.due_date ?? '—'}
                  </span>
                  <span style={{ width: 44, fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-3)' }}>
                    {t.effort ?? '—'}
                  </span>
                </div>
              ))}
            </div>
          ))}
        </div>
      </main>

      <aside style={{ width: 352, flex: 'none', background: 'var(--ink-3)', borderLeft: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column', padding: selected ? '20px 24px' : 0, alignItems: selected ? 'stretch' : 'center', justifyContent: selected ? 'flex-start' : 'center', color: 'var(--fg-5)' }}>
        {selected ? (
          <>
            <div style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 20, color: 'var(--fg-1)', marginBottom: 12 }}>
              {selected.title}
            </div>
            <div style={{ fontSize: 13, color: 'var(--fg-3)' }}>#{selected.id} · {selected.priority}</div>
          </>
        ) : (
          'Select a task'
        )}
      </aside>
    </div>
  );
}
```

- [ ] **Step 6: Manual verification**

Run: `cargo run --bin task_server -- /tmp/web-dev-tasks.db &` (background it), then `cd web && npm run dev`. Open the printed URL in a browser. Add a task via `curl -X POST http://127.0.0.1:4287/api/tasks -H 'content-type: application/json' -d '{"title":"Manual test task","priority":"high"}'`, refresh the page, confirm it appears grouped under "unassigned" with the right priority color. Click it, confirm the inspector shows its title. Stop both processes when done.

- [ ] **Step 7: Commit**

```bash
git add web/
git commit -m "feat: wire real task/agent data into Command Center Web frontend"
```

---

### Task 10: Mocked agent-status sidebar/strip and inspector "Agent waiting" block

**Files:**
- Create: `web/src/mockAgentStatus.ts`
- Modify: `web/src/App.tsx`

**Interfaces:**
- Produces: `export interface MockAgentStatus { name: string; state: 'running' | 'waiting' | 'idle'; detail?: string; }`, `export const MOCK_AGENT_STATUSES: MockAgentStatus[]`. Sidebar agent list now shows a status dot (colored per state) using this mock data keyed by agent name (falling back to `idle` for any real agent name not present in the mock list). The inspector shows a static "Agent waiting" block when the selected task's agent matches a mocked `waiting` entry.

Per the spec, this is explicitly static/placeholder — it must NOT call any API endpoint or claim to be live. Any label near this UI should not mislead the user into thinking it's real-time.

- [ ] **Step 1: Write `web/src/mockAgentStatus.ts`**

```ts
export interface MockAgentStatus {
  name: string;
  state: 'running' | 'waiting' | 'idle';
  detail?: string;
}

// Placeholder data — not backed by a real status source yet (see spec Non-goals:
// live agent status requires a cross-process mechanism not built in this change).
export const MOCK_AGENT_STATUSES: MockAgentStatus[] = [
  { name: 'obsidian-sync', state: 'running', detail: 'rewrote 41 of 128 notes' },
  { name: 'command-center', state: 'waiting', detail: 'asked a question' },
];

export function statusFor(agentName: string): MockAgentStatus {
  return (
    MOCK_AGENT_STATUSES.find((s) => s.name === agentName) ?? { name: agentName, state: 'idle' }
  );
}

const STATE_COLOR: Record<MockAgentStatus['state'], string> = {
  running: 'var(--teal)',
  waiting: 'var(--citrine)',
  idle: 'var(--fg-5)',
};

export function statusColor(state: MockAgentStatus['state']): string {
  return STATE_COLOR[state];
}
```

- [ ] **Step 2: Update the sidebar agent list in `App.tsx`** to show a colored status dot per agent (mocked):

Replace the sidebar agent-list rendering block (`{agents.map((a) => ( ... status dot fixed to var(--fg-5) ... ))}`) with:

```tsx
{agents.map((a) => {
  const status = statusFor(a.name);
  return (
    <div key={a.name} style={{ display: 'flex', alignItems: 'center', gap: 10, height: 28 }}>
      <span style={{ width: 7, height: 7, borderRadius: 999, background: statusColor(status.state) }} />
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)' }}>{a.name}</span>
    </div>
  );
})}
```

Add the import: `import { statusFor, statusColor } from './mockAgentStatus';`

- [ ] **Step 3: Add the mocked "Agent waiting" block to the inspector**

In the inspector `<aside>` block (inside the `selected ? (...) : ...` branch), add this after the priority/id line, only rendered when the selected task's agent is in a `waiting` mock state:

```tsx
{selected.agent && statusFor(selected.agent).state === 'waiting' && (
  <div style={{ marginTop: 'auto', paddingTop: 20, borderTop: '1px solid var(--hairline)' }}>
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
      <span style={{ width: 7, height: 7, borderRadius: 999, background: 'var(--citrine)' }} />
      <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 11, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--citrine)' }}>
        Agent waiting (preview — not yet live)
      </span>
    </div>
    <div style={{ padding: 14, borderRadius: 8, background: 'var(--ink-2)', border: '1px solid var(--hairline)', fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-2)' }}>
      {statusFor(selected.agent).detail ?? 'Waiting for input.'}
    </div>
  </div>
)}
```

The "(preview — not yet live)" label is deliberate — it must be visually honest that this isn't a working feature yet, per the spec's mocked-not-real requirement.

- [ ] **Step 4: Manual verification**

Run `npm run dev` (with `task_server` running as in Task 9), add a task with `"agent": "command-center"` via curl, select it in the UI, confirm the "Agent waiting (preview — not yet live)" block appears with the mocked detail text. Select a task with a different or no agent and confirm it does not appear.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat: add mocked agent-status display to Command Center Web"
```

---

### Task 11: New task form, inline field edits, Mark Done / Reopen / Delete actions

**Files:**
- Create: `web/src/components/NewTaskForm.tsx`
- Modify: `web/src/App.tsx`

**Interfaces:**
- Produces: a working "New task" flow (click button → inline form → `POST /api/tasks` → list updates), inspector actions for Mark Done / Reopen / Delete wired to their API calls with local state updates (no full page reload).

- [ ] **Step 1: Write `web/src/components/NewTaskForm.tsx`**

```tsx
import { useState } from 'react';
import { Button } from './Button';

interface NewTaskFormProps {
  onSubmit: (title: string) => void;
  onCancel: () => void;
}

export function NewTaskForm({ onSubmit, onCancel }: NewTaskFormProps) {
  const [title, setTitle] = useState('');

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (title.trim()) onSubmit(title.trim());
      }}
      style={{ display: 'flex', gap: 8, alignItems: 'center' }}
    >
      <input
        autoFocus
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        placeholder="Task title"
        style={{
          height: 34,
          padding: '0 12px',
          borderRadius: 5,
          border: '1px solid var(--hairline)',
          background: 'var(--ink-2)',
          color: 'var(--fg-1)',
          fontSize: 14,
          width: 280,
        }}
      />
      <Button size="sm">Add</Button>
      <Button size="sm" variant="secondary" onClick={onCancel}>Cancel</Button>
    </form>
  );
}
```

- [ ] **Step 2: Wire it into `App.tsx`**

Add state and handlers:

```tsx
const [showNewTaskForm, setShowNewTaskForm] = useState(false);

async function handleAddTask(title: string) {
  try {
    const created = await addTask({ title });
    setTasks((prev) => [...prev, created]);
    setShowNewTaskForm(false);
  } catch (e) {
    setError(String(e));
  }
}

async function handleMarkDone(task: Task) {
  try {
    const { completed, spawned } = await markDone(task.id);
    setTasks((prev) => {
      const next = prev.map((t) => (t.id === completed.id ? completed : t));
      return spawned ? [...next, spawned] : next;
    });
    setSelected(completed);
  } catch (e) {
    setError(String(e));
  }
}

async function handleReopen(task: Task) {
  try {
    const updated = await reopenTask(task.id);
    setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    setSelected(updated);
  } catch (e) {
    setError(String(e));
  }
}

async function handleDelete(task: Task) {
  try {
    await deleteTask(task.id);
    setTasks((prev) => prev.filter((t) => t.id !== task.id));
    setSelected(null);
  } catch (e) {
    setError(String(e));
  }
}
```

Update the imports at the top: `import { fetchTasks, fetchAgents, addTask, markDone, reopenTask, deleteTask } from './api';` and `import { NewTaskForm } from './components/NewTaskForm';`.

Replace the header's `<Button>New task</Button>` with:

```tsx
{showNewTaskForm ? (
  <NewTaskForm onSubmit={handleAddTask} onCancel={() => setShowNewTaskForm(false)} />
) : (
  <Button onClick={() => setShowNewTaskForm(true)}>New task</Button>
)}
```

In the inspector's selected-task branch, add action buttons below the title/id line:

```tsx
<div style={{ display: 'flex', gap: 8, marginTop: 12, marginBottom: 20 }}>
  {selected.status === 'open' ? (
    <Button size="sm" onClick={() => handleMarkDone(selected)}>Mark done</Button>
  ) : (
    <Button size="sm" variant="secondary" onClick={() => handleReopen(selected)}>Reopen</Button>
  )}
  <Button size="sm" variant="secondary" onClick={() => handleDelete(selected)}>Delete</Button>
</div>
```

- [ ] **Step 3: Manual verification**

Run the dev server + `task_server` as before. Click "New task", type a title, submit — confirm it appears in the "unassigned" group. Select it, click "Mark done" — confirm it disappears from view or shows as done (status text/strike-through is a reasonable minimal indicator; exact done-state styling is this task's implementer's call, note what you did in your report). Click "Reopen" on a done task — confirm it flips back. Click "Delete" — confirm it's removed from the list and the inspector clears.

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat: wire New Task, Mark Done, Reopen, and Delete actions"
```

---

### Task 12: Documentation and final end-to-end verification

**Files:**
- Modify: `README.md` (add a "Command Center Web" section — read the existing file first to match its structure/tone; if no `README.md` exists at the repo root, create one covering just this section)

**Interfaces:** none (documentation + verification only).

- [ ] **Step 1: Document how to run it**

Add a section (wherever `README.md`'s existing structure best fits — read it first) covering:

```markdown
## Command Center Web

A browser-based companion to the `task` CLI/TUI.

**Development** (hot-reload frontend):
```bash
cargo run --bin task_server        # starts the API on http://127.0.0.1:4287
cd web && npm install && npm run dev   # starts Vite dev server, proxies /api to task_server
```

**Regular use** (single binary, built frontend):
```bash
cd web && npm install && npm run build   # produces web/dist/
cd .. && cargo run --release --bin task_server
# open http://127.0.0.1:4287
```

`task_server` resolves the task database the same way the CLI does (`--file`/`TASK_FILE`/`default-dir` config/`./tasks.db`). Override the port with `TASK_SERVER_PORT`.

**Current scope:** table + inspector view of real tasks (1a shell only). Agent status shown in the sidebar and the inspector's "Agent waiting" panel is placeholder/mock data — not yet backed by real agent session state. No keyboard shortcuts, drag-and-drop, or in-browser note editing yet.
```

- [ ] **Step 2: Full end-to-end smoke test**

```bash
cargo build --bin task_server
cd web && npm run build && cd ..
cargo run --release --bin task_server -- /tmp/e2e-smoke-tasks.db
```

In a browser, open `http://127.0.0.1:4287`. Confirm: the built (not dev-server) frontend loads and looks like the dev version did; add a task; select it; mark it done; reopen it; delete it. Confirm each action persists (refresh the page after each and confirm state survived). Clean up: stop the server, `rm /tmp/e2e-smoke-tasks.db*`.

- [ ] **Step 3: Run the full Rust test suite one more time**

Run: `cargo test --features tui`
Expected: PASS (all backend tests from Tasks 1-7, plus every pre-existing test — this change must not have broken the TUI build/tests).

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: document how to run Command Center Web"
```

---

## Self-Review Notes

- **Spec coverage:** backend API (all 7 endpoints, Tasks 3-7), recurrence JSON fix (Task 1), DRY filter/sort reuse (Task 2), frontend scaffold + tokens (Task 8), real data wiring (Task 9), mocked agent status clearly labeled as non-live (Task 10), CRUD actions (Task 11), docs + e2e verification (Task 12). All spec sections covered; all Non-goals respected (no 1b, no drag-drop, no live Q&A, no shortcuts, no note editing, no auth).
- **Placeholder scan:** no TBD/TODO left unresolved; the one intentionally-flexible item (font file transfer via DesignSync possibly needing a fallback to system fonts) has a concrete fallback specified, not an open placeholder.
- **Type consistency:** `Task` TypeScript interface (Task 9) matches the Rust `Task` struct's actual JSON shape after Task 1's recurrence fix (including which fields are optional based on `skip_serializing_if`). `AddTaskRequest`/`EditTaskRequest` (Tasks 4-5) field names match `commands::AddArgs`/`EditArgs` field names, which the frontend's `addTask`/`editTask` (Task 9) parameter shapes mirror exactly.
