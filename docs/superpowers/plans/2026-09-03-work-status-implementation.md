# Work Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new, independent `work_status` field to `Task` (`todo` / `in-progress` / `waiting-for-review` / `complete`) that agents set via the CLI and the human tracks in the web UI.

**Architecture:** A new `WorkStatus` enum in `task.rs`, persisted as a new SQLite column via an `ALTER TABLE` migration, exposed through the existing `task edit` CLI command and the existing `PATCH /api/tasks/:id` server endpoint (both following the field-clearing pattern already used for `due`/`agent`), and surfaced in the web UI as a table badge, an inspector dropdown, and a new Group-by option.

**Tech Stack:** Rust (rusqlite, serde, clap, axum), TypeScript/React.

**Spec:** `docs/superpowers/specs/2026-09-03-work-status-design.md`

## Global Constraints

- `work_status` is fully independent of `status`/`Status::Open|Done`. Do not touch due-window filtering, `Mark done`/`Reopen`, or recurrence logic anywhere in this plan.
- Setting `work_status` to `complete` must NOT change `status`. No auto-completion.
- TUI (`src/tui.rs`, `src/bin/task_tui.rs`) is explicitly out of scope — do not add `work_status` there.
- CLI value strings (exact, lowercase, kebab-case): `todo`, `in-progress`, `waiting-for-review`, `complete`. `FromStr` should also accept the aliases `to-do`, `in_progress`/`inprogress`, `review`, and `done` respectively (see spec).
- Every new `Option<T>`-typed field on the server's `EditTaskRequest` must treat an empty string as "clear the field", matching the existing `due`/`agent`/`project` handling in `edit_task` — never let an empty string round-trip into `Some("")`.
- Web color tokens for the four states (from `web/src/tokens.css`, do not invent new hex values): Todo = `var(--fg-4)`, In Progress = `var(--citrine)`, Waiting for Review = `var(--cyan)`, Complete = `var(--teal)`.

---

### Task 1: Data model and persistence

**Files:**
- Modify: `src/task.rs` (add `WorkStatus` enum near `Effort`, add `work_status` field to `Task`)
- Modify: `src/db.rs` (schema migration, `row_to_task`, `insert_task_row`)
- Test: inline `#[cfg(test)]` modules in `src/db.rs` and `src/task.rs`

**Interfaces:**
- Produces: `pub enum WorkStatus { Todo, InProgress, WaitingForReview, Complete }` implementing `Display`, `FromStr<Err = String>`, and deriving `Debug, Clone, Copy, PartialEq, Serialize` with `#[serde(rename_all = "kebab-case")]`.
- Produces: `Task.work_status: Option<WorkStatus>` with `#[serde(skip_serializing_if = "Option::is_none")]`.
- Consumed by: Task 2 (CLI), Task 3 (server).

- [ ] **Step 1: Add the `WorkStatus` enum to `src/task.rs`**

Add this immediately after the `Effort` enum's `FromStr` impl (after line 32, before the `// -- Recurrence --` comment):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkStatus {
    Todo,
    InProgress,
    WaitingForReview,
    Complete,
}

impl std::fmt::Display for WorkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkStatus::Todo => write!(f, "todo"),
            WorkStatus::InProgress => write!(f, "in-progress"),
            WorkStatus::WaitingForReview => write!(f, "waiting-for-review"),
            WorkStatus::Complete => write!(f, "complete"),
        }
    }
}

impl std::str::FromStr for WorkStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "todo" | "to-do" => Ok(WorkStatus::Todo),
            "in-progress" | "in_progress" | "inprogress" => Ok(WorkStatus::InProgress),
            "waiting-for-review" | "review" => Ok(WorkStatus::WaitingForReview),
            "complete" | "done" => Ok(WorkStatus::Complete),
            _ => Err(format!(
                "Invalid work status: '{}'. Valid values: todo, in-progress, waiting-for-review, complete",
                s
            )),
        }
    }
}
```

- [ ] **Step 2: Add the field to `Task`**

In the `Task` struct (starts at line 285), add this right after the `effort` field:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_status: Option<WorkStatus>,
}
```

(i.e. insert the new field between the existing `effort` field and the struct's closing `}`.)

After this step, the crate will fail to compile everywhere a `Task { ... }` literal is constructed without `work_status` (every test fixture in `db.rs`, `server.rs`, `commands.rs`, `tui.rs`, etc.). That's expected and will be fixed incrementally as each file is touched in later steps of this task and in Tasks 2-3. For now, run:

```bash
cargo build --lib 2>&1 | grep "missing field" | wc -l
```

Just confirm it's a nonzero count of `missing field \`work_status\`` errors (not some other, unrelated error) — do not try to fix them all here.

- [ ] **Step 3: Add the write-a-failing-test for the enum**

In `src/task.rs`'s existing `#[cfg(test)] mod tests` block, add:

```rust
    #[test]
    fn test_work_status_from_str_and_display() {
        assert_eq!(WorkStatus::from_str("todo").unwrap(), WorkStatus::Todo);
        assert_eq!(WorkStatus::from_str("in-progress").unwrap(), WorkStatus::InProgress);
        assert_eq!(WorkStatus::from_str("waiting-for-review").unwrap(), WorkStatus::WaitingForReview);
        assert_eq!(WorkStatus::from_str("complete").unwrap(), WorkStatus::Complete);
        assert_eq!(WorkStatus::from_str("review").unwrap(), WorkStatus::WaitingForReview);
        assert!(WorkStatus::from_str("bogus").is_err());
        assert_eq!(WorkStatus::InProgress.to_string(), "in-progress");
    }

    #[test]
    fn test_task_serialize_work_status_kebab_case() {
        let json = serde_json::to_string(&WorkStatus::WaitingForReview).unwrap();
        assert_eq!(json, "\"waiting-for-review\"");
    }
```

Run `cargo test --lib task::tests::test_work_status` to confirm both pass (this only requires Steps 1-2 above, not the rest of the crate compiling — if the crate doesn't compile yet because of Step 2's fallout, fix every `Task { ... }` literal inside `src/task.rs` itself first, adding `work_status: None,` to each, before running this test).

- [ ] **Step 4: Migrate the SQLite schema in `src/db.rs`**

Add the import: change line 7 from

```rust
use crate::task::{Effort, Priority, Recurrence, Status, Task, TaskFile};
```

to

```rust
use crate::task::{Effort, Priority, Recurrence, Status, Task, TaskFile, WorkStatus};
```

In `open_conn` (around line 51-61), add the migration immediately after the `execute_batch(SCHEMA_SQL)` call:

```rust
fn open_conn(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("Failed to set busy timeout: {}", e))?;
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| format!("Failed to initialize schema: {}", e))?;
    // ALTER TABLE has no "ADD COLUMN IF NOT EXISTS" — this fails with a
    // "duplicate column" error on every database that already has the
    // column, which is the expected, common case. Any other failure here
    // (locked/corrupt file) will surface immediately on the next query
    // against `tasks` anyway, so discarding the error is safe.
    conn.execute("ALTER TABLE tasks ADD COLUMN work_status TEXT", [])
        .ok();
    Ok(conn)
}
```

- [ ] **Step 5: Update `row_to_task` and `insert_task_row`**

In `row_to_task` (around line 74-110), add a 15th column read right after the `effort_s` read (line 88):

```rust
    let effort_s: Option<String> = row.get(13)?;
    let work_status_s: Option<String> = row.get(14)?;
```

And in the `Ok(Task { ... })` construction, add after `effort`:

```rust
        effort: effort_s.and_then(|s| Effort::from_str(&s).ok()),
        work_status: work_status_s.and_then(|s| WorkStatus::from_str(&s).ok()),
    })
```

In `insert_task_row` (around line 112-135), update the SQL to include the new column and parameter:

```rust
fn insert_task_row(conn: &Connection, t: &Task) -> Result<(), String> {
    conn.execute(
        "INSERT INTO tasks (id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort, work_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            t.id,
            t.title,
            t.status.to_string(),
            t.priority.to_string(),
            list_to_text(&t.tags),
            t.created.to_rfc3339(),
            t.updated.map(|u| u.to_rfc3339()),
            t.description,
            t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            t.project,
            t.recurrence.as_ref().map(|r| r.to_string()),
            list_to_text(&t.notes),
            t.agent,
            t.effort.as_ref().map(|e| e.to_string()),
            t.work_status.as_ref().map(|w| w.to_string()),
        ],
    )
    .map_err(|e| format!("Failed to insert task {}: {}", t.id, e))?;
    Ok(())
}
```

Also find the `SELECT` statement that feeds `row_to_task` (around line 170: `"SELECT id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort FROM tasks ORDER BY id"`) and add `, work_status` before `FROM tasks`.

- [ ] **Step 6: Fix every remaining `Task { ... }` literal in `src/db.rs`**

Search this file for `Task {` (there are several in the `#[cfg(test)]` module, e.g. around lines 260-380) and add `work_status: None,` to each one that doesn't already set it. Run:

```bash
cargo build --lib 2>&1 | grep "src/db.rs"
```

Fix until this prints nothing.

- [ ] **Step 7: Write the migration regression test**

Add to `src/db.rs`'s test module — this is the most important test in this task, since it's the one thing with no existing precedent in the codebase:

```rust
    #[test]
    fn test_migration_adds_work_status_column_to_pre_existing_db() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");

        // Simulate a real pre-existing database: create it with the schema
        // as it existed before this change (no work_status column), and
        // seed one row, using raw SQL rather than going through db::save
        // (which would already include the new column).
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE tasks (
                    id          INTEGER PRIMARY KEY,
                    title       TEXT NOT NULL,
                    status      TEXT NOT NULL,
                    priority    TEXT NOT NULL,
                    tags        TEXT NOT NULL DEFAULT '',
                    created     TEXT NOT NULL,
                    updated     TEXT,
                    description TEXT,
                    due_date    TEXT,
                    project     TEXT,
                    recurrence  TEXT,
                    notes       TEXT NOT NULL DEFAULT '',
                    agent       TEXT,
                    effort      TEXT
                );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO tasks (id, title, status, priority, tags, created) VALUES (1, 'Pre-existing', 'open', 'medium', '', '2026-01-01T00:00:00+00:00')",
                [],
            )
            .unwrap();
        }

        // Opening it now (via the normal load path) must not error, must
        // not lose the existing row, and the new column must be usable.
        let task_file = load(&path).unwrap();
        assert_eq!(task_file.tasks.len(), 1);
        assert_eq!(task_file.tasks[0].title, "Pre-existing");
        assert_eq!(task_file.tasks[0].work_status, None);

        // And a subsequent save/load round-trip with work_status set works.
        let mut task_file = task_file;
        task_file.tasks[0].work_status = Some(WorkStatus::InProgress);
        save(&path, &task_file).unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded.tasks[0].work_status, Some(WorkStatus::InProgress));
    }
```

Check this test module's existing imports (top of the `#[cfg(test)]` block) for whether `Connection` and `tempdir` are already imported — they almost certainly are, given the other tests in this file already use them. Add `use super::*;` coverage for `WorkStatus` if it isn't already brought in by a wildcard import.

- [ ] **Step 8: Run the full test suite and commit**

```bash
cargo test --lib
```

All tests must pass except the pre-existing, unrelated `auth::tests::test_write_read_delete_token_public_api` failure (environmental — confirmed failing on unmodified `main` too; do not attempt to fix it as part of this task).

```bash
git add src/task.rs src/db.rs
git commit -m "feat: add work_status field to Task, migrate SQLite schema"
```

---

### Task 2: CLI surface

**Files:**
- Modify: `src/cli.rs` (add `--work-status` flag to `Edit` command)
- Modify: `src/commands.rs` (add `work_status` to `EditArgs`, parse and apply in `edit()`, print it in `show()`)
- Modify: `src/bin/task.rs` (thread the new field through `Command::Edit`)
- Test: inline `#[cfg(test)]` module in `src/commands.rs`

**Interfaces:**
- Consumes: `WorkStatus::from_str`/`Display` from Task 1.
- Produces: `task edit <id> --work-status <value>` CLI surface. No other command changes.

- [ ] **Step 1: Add the CLI flag**

In `src/cli.rs`, in the `Edit` variant (starts at line 103), add after the existing `effort` field (line 129):

```rust
        #[arg(long)]
        effort: Option<String>,

        #[arg(long)]
        work_status: Option<String>,
    },
```

- [ ] **Step 2: Extend `EditArgs` and `edit()` in `src/commands.rs`**

Add to `EditArgs` (line 36-45), after `effort`:

```rust
pub struct EditArgs {
    pub title: Option<String>,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub effort: Option<String>,
    pub work_status: Option<String>,
}
```

In `edit()` (line 188-232), parse it alongside `effort` (after line 192):

```rust
    let effort = args.effort.as_deref().map(Effort::from_str).transpose().map_err(|e| (1, e))?;
    let work_status = args.work_status.as_deref().map(crate::task::WorkStatus::from_str).transpose().map_err(|e| (1, e))?;
```

And apply it alongside the `effort` assignment (after line 226):

```rust
    if let Some(e) = effort {
        t.effort = Some(e);
    }
    if let Some(w) = work_status {
        t.work_status = Some(w);
    }
```

- [ ] **Step 3: Print it in `show()`**

In `show()` (line 144-184), add after the `effort` line (line 173-175):

```rust
    if let Some(ref e) = t.effort {
        out.push_str(&format!("effort: {}\n", e));
    }
    if let Some(ref w) = t.work_status {
        out.push_str(&format!("work_status: {}\n", w));
    }
```

- [ ] **Step 4: Thread it through `src/bin/task.rs`**

Change line 215-220 from:

```rust
        Some(Command::Edit { id, title, priority, due, project, tags, agent, description, effort }) => {
            let msg = task::commands::edit(&path, id, task::commands::EditArgs {
                title, priority, due, project, tags, agent, description, effort,
            })?;
```

to:

```rust
        Some(Command::Edit { id, title, priority, due, project, tags, agent, description, effort, work_status }) => {
            let msg = task::commands::edit(&path, id, task::commands::EditArgs {
                title, priority, due, project, tags, agent, description, effort, work_status,
            })?;
```

- [ ] **Step 5: Fix any remaining `EditArgs { ... }` literals**

```bash
cargo build --lib 2>&1 | grep "missing field"
```

Fix any `EditArgs { ... }` construction sites this reveals (there is likely at least one existing test in `src/commands.rs` that constructs `EditArgs` directly) by adding `work_status: None,`.

- [ ] **Step 6: Write the test**

Add to `src/commands.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn test_edit_sets_work_status_and_show_prints_it() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        add(&path, AddArgs { title: "Track me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();

        edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, effort: None,
            work_status: Some("in-progress".to_string()),
        }).unwrap();

        let output = show(&path, 1).unwrap();
        assert!(output.contains("work_status: in-progress"));
    }

    #[test]
    fn test_edit_invalid_work_status_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        add(&path, AddArgs { title: "Track me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();

        let result = edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, effort: None,
            work_status: Some("bogus".to_string()),
        });
        assert!(result.is_err());
    }
```

Check the existing test module for the exact shape of `AddArgs` used elsewhere in this file (there are several examples, e.g. around line 315 and 392 from earlier in the file) and match it exactly — the fields shown above must match the real `AddArgs` struct definition.

- [ ] **Step 7: Run tests and commit**

```bash
cargo test --lib commands::
cargo build --bin task
```

```bash
git add src/cli.rs src/commands.rs src/bin/task.rs
git commit -m "feat: add --work-status flag to task edit CLI command"
```

---

### Task 3: Server API

**Files:**
- Modify: `src/server.rs` (`EditTaskRequest`, `edit_task`)
- Test: inline `#[cfg(test)] mod tests` in `src/server.rs`

**Interfaces:**
- Consumes: `WorkStatus::from_str` from Task 1.
- Produces: `PATCH /api/tasks/:id` accepts `{"work_status": "in-progress" | "" | ...}` in its JSON body. `""` clears the field (matches the existing `due`/`agent` clearing behavior — do not skip this; it's a Global Constraint).

- [ ] **Step 1: Add the field to `EditTaskRequest`**

Around line 181-190:

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
    pub work_status: Option<String>,
}
```

- [ ] **Step 2: Parse it in `edit_task`, following the `due`/`agent` clearing pattern**

Find where `effort` is parsed in `edit_task` (around line 204-209):

```rust
    let effort = req
        .effort
        .as_deref()
        .map(crate::task::Effort::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
```

Add immediately after it — note this one is `Option<Option<WorkStatus>>`, not `Option<WorkStatus>`, exactly like the `due_date`/`agent` clearing fix (empty string is an explicit clear, distinct from the field being absent):

```rust
    // work_status: "" is an explicit clear, same pattern as due/agent.
    let work_status: Option<Option<crate::task::WorkStatus>> = req
        .work_status
        .as_deref()
        .map(|s| {
            if s.is_empty() {
                Ok(None)
            } else {
                crate::task::WorkStatus::from_str(s).map(Some)
            }
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
```

- [ ] **Step 3: Apply it in the task-mutation block**

Find where `effort` is applied (around line 256-258):

```rust
        if let Some(e) = effort {
            t.effort = Some(e);
        }
```

Add immediately after it:

```rust
        if let Some(w) = work_status {
            t.work_status = w;
        }
```

(Note: unlike the `effort` line above it, this is `t.work_status = w;` — NOT `Some(w)` — because `w` is already `Option<WorkStatus>` here, matching exactly how the `due_date`/`agent` clearing fix works a few lines earlier in this same function.)

- [ ] **Step 4: Fix any remaining `Task { ... }` literals in `src/server.rs`**

```bash
cargo build --lib 2>&1 | grep "src/server.rs"
```

There are many `Task { ... }` literals in this file's test module (the `make_state`/seed helpers). Add `work_status: None,` to each one this reveals.

- [ ] **Step 5: Write the tests**

Add to `src/server.rs`'s test module, next to the existing `test_edit_task_empty_due_clears_it` / `test_edit_task_empty_agent_clears_it`:

```rust
    #[tokio::test]
    async fn test_edit_task_sets_work_status() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "work_status": "in-progress" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["work_status"], "in-progress");
    }

    #[tokio::test]
    async fn test_edit_task_invalid_work_status_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "work_status": "bogus" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_edit_task_empty_work_status_clears_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: Some(crate::task::WorkStatus::Complete),
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "work_status": "" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json["work_status"].is_null());
    }
```

- [ ] **Step 6: Run tests and commit**

```bash
cargo test --lib server::
cargo build --bin task_server
```

```bash
git add src/server.rs
git commit -m "feat: expose work_status through PATCH /api/tasks/:id"
```

---

### Task 4: Web frontend

**Files:**
- Modify: `web/src/types.ts` (add `work_status` to `Task`)
- Modify: `web/src/api.ts` (add `work_status` to `editTask`'s changes type)
- Modify: `web/src/App.tsx` (table badge column, inspector field, Group-by option)

**Interfaces:**
- Consumes: `GET /api/tasks` now returns `work_status?: string` per task (Task 3); `PATCH /api/tasks/:id` accepts `work_status` in its body (Task 3).
- Produces: nothing consumed by later tasks — this is the last task in the plan.

- [ ] **Step 1: Add the type**

In `web/src/types.ts`, add to the `Task` interface, after `effort`:

```typescript
export interface Task {
  // ... existing fields ...
  effort?: Effort;
  work_status?: 'todo' | 'in-progress' | 'waiting-for-review' | 'complete';
}
```

- [ ] **Step 2: Add it to `editTask`'s changes type in `web/src/api.ts`**

Find `editTask`'s signature:

```typescript
export async function editTask(id: number, changes: Partial<{
  title: string; priority: string; due: string; project: string; tags: string; agent: string; description: string; effort: string;
}>): Promise<Task> {
```

Add `work_status: string;` to that inline type:

```typescript
export async function editTask(id: number, changes: Partial<{
  title: string; priority: string; due: string; project: string; tags: string; agent: string; description: string; effort: string; work_status: string;
}>): Promise<Task> {
```

No other change needed here — the function body already just JSON-stringifies `changes` as-is.

- [ ] **Step 3: Add the `WORK_STATUS` constants near the other option constants in `App.tsx`**

Near `PRIORITY_OPTIONS`/`EFFORT_OPTIONS` (around line 23-34), add:

```typescript
const WORK_STATUS_OPTIONS = [
  { value: '', label: 'None' },
  { value: 'todo', label: 'To Do' },
  { value: 'in-progress', label: 'In Progress' },
  { value: 'waiting-for-review', label: 'Waiting for Review' },
  { value: 'complete', label: 'Complete' },
];

const WORK_STATUS_LABEL: Record<string, string> = {
  'todo': 'To Do',
  'in-progress': 'In Progress',
  'waiting-for-review': 'Waiting for Review',
  'complete': 'Complete',
};

const WORK_STATUS_COLOR: Record<string, string> = {
  'todo': 'var(--fg-4)',
  'in-progress': 'var(--citrine)',
  'waiting-for-review': 'var(--cyan)',
  'complete': 'var(--teal)',
};
```

- [ ] **Step 4: Add `'work_status'` to the `GroupBy` type and options**

Find:

```typescript
type GroupBy = 'agent' | 'project' | 'priority' | 'none';

const GROUP_BY_OPTIONS: { value: GroupBy; label: string }[] = [
  { value: 'agent', label: 'Agent' },
  { value: 'project', label: 'Project' },
  { value: 'priority', label: 'Priority' },
  { value: 'none', label: 'None' },
];
```

Change to:

```typescript
type GroupBy = 'agent' | 'project' | 'priority' | 'work_status' | 'none';

const GROUP_BY_OPTIONS: { value: GroupBy; label: string }[] = [
  { value: 'agent', label: 'Agent' },
  { value: 'project', label: 'Project' },
  { value: 'priority', label: 'Priority' },
  { value: 'work_status', label: 'Work status' },
  { value: 'none', label: 'None' },
];
```

Find `groupKey`:

```typescript
function groupKey(task: Task, groupBy: GroupBy): string {
  switch (groupBy) {
    case 'agent':
      return task.agent ?? 'unassigned';
    case 'project':
      return task.project ?? 'no project';
    case 'priority':
      return task.priority;
    case 'none':
      return '';
  }
}
```

Add a case, matching the existing `'unassigned'`/`'no project'` convention for the empty state:

```typescript
    case 'priority':
      return task.priority;
    case 'work_status':
      return task.work_status ?? 'no status';
    case 'none':
      return '';
```

- [ ] **Step 5: Add the table column**

Find the column header row (has `ID`/`Priority`/the icon column/`Task`/`Due`/`Effort` spans) and add a `Status` header after the icon column and before `Task`:

```tsx
          <span style={{ width: 34 }}>ID</span>
          <span style={{ width: 78 }}>Priority</span>
          <span style={{ width: 64 }}></span>
          <span style={{ width: 100 }}>Status</span>
          <span style={{ flex: 1 }}>Task</span>
          <span style={{ width: 96 }}>Due</span>
          <span style={{ width: 44 }}>Effort</span>
```

Find the corresponding row rendering (the icon-cluster `<span style={{ width: 64, ... }}>` block containing `AlertTriangle`/`CalendarOff`/`UserX`/`Repeat`) and add a badge span immediately after it, before the title `<span>`:

```tsx
                  <span style={{ width: 100 }}>
                    {t.work_status && (
                      <span
                        style={{
                          display: 'inline-block', fontSize: 10, fontWeight: 700, textTransform: 'uppercase',
                          letterSpacing: '0.04em', padding: '2px 8px', borderRadius: 999,
                          color: 'var(--ink)', background: WORK_STATUS_COLOR[t.work_status],
                        }}
                      >
                        {WORK_STATUS_LABEL[t.work_status]}
                      </span>
                    )}
                  </span>
```

- [ ] **Step 6: Add the inspector field**

Find the `FieldRow label="Effort"` block in the inspector and add a new `FieldRow` immediately after it:

```tsx
              <FieldRow label="Work status">
                <EditableField
                  value={selected.work_status ?? ''}
                  type="select"
                  options={WORK_STATUS_OPTIONS}
                  onSave={(v) => handleEditField(selected.id, { work_status: v })}
                  display={selected.work_status ? WORK_STATUS_LABEL[selected.work_status] : undefined}
                />
              </FieldRow>
```

- [ ] **Step 7: Build and verify**

```bash
cd web && npm run build
```

Must produce no TypeScript errors. There is no frontend test runner in this project (confirmed: `package.json`'s `scripts` has no `test` entry) — a clean `npm run build` is the acceptance bar for this task, consistent with every other frontend task in this project's history.

- [ ] **Step 8: Commit**

```bash
git add web/src/types.ts web/src/api.ts web/src/App.tsx
git commit -m "feat: add work_status badge, inspector field, and group-by option to web UI"
```

---

## Final integration check (not a subagent task — do this yourself after all 4 tasks are reviewed and merged)

1. `cargo test --lib` — full suite green except the pre-existing `auth` failure.
2. `cd web && npm run build` — clean.
3. Restart the real `task_server` (rebuild in release mode) and confirm via `curl http://127.0.0.1:4287/api/tasks | head` that existing real tasks still load correctly (this is the live check that the `ALTER TABLE` migration works against the actual production database, not just the test fixtures).
4. Smoke-test end-to-end against a **scratch** database (never the real one) — pass a throwaway db path as the server's first CLI arg: create a task, set `work_status` via `task edit <id> --work-status in-progress` using the CLI pointed at that same scratch file, confirm the web UI's badge/inspector/group-by reflect it, confirm clearing it via the UI (or `--work-status ""` — note the CLI doesn't currently support passing an empty string through clap the same way the HTTP API does empty-string-clears, so clearing via CLI may need `task edit <id> --work-status todo` or similar as a manual reset rather than true clearing; this is expected and not a bug to fix in this plan).
