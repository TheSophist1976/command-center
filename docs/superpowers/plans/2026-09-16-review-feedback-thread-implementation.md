# Review Feedback Thread Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a human give an agent round-by-round feedback on a `waiting-for-review` task through a single rendered markdown thread, without a separate reassignment step.

**Architecture:** A deterministic per-task note (`task-<id>-review-thread`) holds the whole thread as append-only markdown. A new `note::append_to_note` primitive backs both a new CLI command (`task note append`) and two new server endpoints (`GET`/`POST /api/tasks/:id/review`). A new `WorkStatus::ChangesRequested` value signals "human left feedback, agent's turn again." The web UI renders the thread via `react-markdown` in a new inspector panel with a feedback box that posts a round and flips the status in one call.

**Tech Stack:** Rust (rusqlite, serde, clap, axum), TypeScript/React, react-markdown.

**Spec:** `docs/superpowers/specs/2026-09-16-review-feedback-thread-design.md`

## Global Constraints

- No new SQLite column or migration — the review thread is identified purely by its deterministic slug `task-<id>-review-thread`, computed wherever needed, never stored.
- `server.rs`'s new endpoints must follow the file's existing dominant pattern (`db::load` → mutate → `db::save`, guarded by `state.write_lock`) — **not** the newer atomic single-row functions (`db::get_task`/`db::update_task`/`db::append_note`) that a separate, unrelated, already-in-progress refactor added to `db.rs`/`bin/task.rs`/`bin/task_tui.rs`. Do not touch that refactor.
- `WorkStatus::ChangesRequested`'s string form is `changes-requested`; aliases `changes_requested`, `changesrequested`, `changes` — none of these may collide with the existing `"review"` alias for `WaitingForReview`.
- `task note append` requires the note to already exist (same convention as `Edit`/`Rm`/`Show`) — it never creates one.
- Any new `NoteCommand` variant added to `src/cli.rs` must get a matching match arm in **both** `src/bin/task.rs` and `src/bin/task_tui.rs` — these two binaries independently duplicate the entire `NoteCommand` match, and a prior feature in this project shipped a real build break by only updating one of them.
- Do not fix the pre-existing `task.rs`/`task_tui.rs` notes-directory inconsistency (one joins `Notes/`, the other doesn't) — out of scope, flagged in the spec, not this plan's job.
- Do not add path-traversal validation to the new review endpoints — the slug is always server-computed from a `u32` task id via `review_thread_slug`, never taken from request input, so the existing `is_valid_slug` pattern used elsewhere doesn't apply here.

---

### Task 1: `append_to_note` primitive and the `ChangesRequested` work status

**Files:**
- Modify: `src/note.rs` (new `append_to_note` function + tests)
- Modify: `src/task.rs` (`WorkStatus` enum, `Display`, `FromStr`, existing tests)
- Test: inline `#[cfg(test)]` modules in both files

**Interfaces:**
- Produces: `pub fn append_to_note(dir: &Path, slug: &str, section: &str) -> Result<PathBuf, String>` in `src/note.rs` — appends `section` to the note's existing body, separated by a blank line if the body is non-empty; errors if the note doesn't exist (via `read_note`'s existing error).
- Produces: `WorkStatus::ChangesRequested` variant, `Display` → `"changes-requested"`, `FromStr` accepting `"changes-requested"`, `"changes_requested"`, `"changesrequested"`, `"changes"`.
- Consumed by: Task 2 (CLI), Task 3 (server).

- [ ] **Step 1: Add `append_to_note` to `src/note.rs`**

Add this function immediately after `write_note` (which ends around line 112, right before `pub fn discover_notes`):

```rust
/// Appends a markdown section to an existing note's body, separated from
/// whatever came before by a blank line. The note must already exist —
/// this never creates one (mirrors read_note's "must exist" behavior).
pub fn append_to_note(dir: &Path, slug: &str, section: &str) -> Result<PathBuf, String> {
    let path = dir.join(format!("{}.md", slug));
    let mut n = read_note(&path)?;
    if n.body.trim().is_empty() {
        n.body = section.to_string();
    } else {
        n.body = format!("{}\n\n{}", n.body.trim_end(), section);
    }
    write_note(dir, &n)
}
```

- [ ] **Step 2: Write tests for `append_to_note`**

Add to `src/note.rs`'s existing `#[cfg(test)] mod tests` block (it already has `use super::*;` and `use tempfile::tempdir;` — reuse those, don't re-import):

```rust
    #[test]
    fn test_append_to_note_on_empty_body() {
        let dir = tempdir().unwrap();
        let note = Note { slug: "thread".to_string(), title: "Thread".to_string(), body: String::new() };
        write_note(dir.path(), &note).unwrap();

        append_to_note(dir.path(), "thread", "## First section\n\nHello.").unwrap();

        let read = read_note(&dir.path().join("thread.md")).unwrap();
        assert_eq!(read.body, "## First section\n\nHello.");
    }

    #[test]
    fn test_append_to_note_on_non_empty_body_adds_blank_line_separator() {
        let dir = tempdir().unwrap();
        let note = Note { slug: "thread".to_string(), title: "Thread".to_string(), body: "## First section\n\nHello.".to_string() };
        write_note(dir.path(), &note).unwrap();

        append_to_note(dir.path(), "thread", "## Second section\n\nMore text.").unwrap();

        let read = read_note(&dir.path().join("thread.md")).unwrap();
        assert_eq!(read.body, "## First section\n\nHello.\n\n## Second section\n\nMore text.");
    }

    #[test]
    fn test_append_to_note_errors_when_note_does_not_exist() {
        let dir = tempdir().unwrap();
        let result = append_to_note(dir.path(), "nope", "## Section");
        assert!(result.is_err());
    }
```

- [ ] **Step 3: Run the new tests**

```bash
cargo test --lib note::tests::test_append_to_note
```

Expected: 3 passed.

- [ ] **Step 4: Add `WorkStatus::ChangesRequested` to `src/task.rs`**

Change the enum (around line 36-41):

```rust
pub enum WorkStatus {
    Todo,
    InProgress,
    WaitingForReview,
    ChangesRequested,
    Complete,
}
```

Update the `Display` impl (around line 43-52) to add a match arm — insert after the `WaitingForReview` arm:

```rust
            WorkStatus::WaitingForReview => write!(f, "waiting-for-review"),
            WorkStatus::ChangesRequested => write!(f, "changes-requested"),
            WorkStatus::Complete => write!(f, "complete"),
```

Update `FromStr` (around line 54-68) — insert after the `waiting-for-review` arm, and update the error message's valid-values list:

```rust
            "waiting-for-review" | "review" => Ok(WorkStatus::WaitingForReview),
            "changes-requested" | "changes_requested" | "changesrequested" | "changes" => Ok(WorkStatus::ChangesRequested),
            "complete" | "done" => Ok(WorkStatus::Complete),
            _ => Err(format!(
                "Invalid work status: '{}'. Valid values: todo, in-progress, waiting-for-review, changes-requested, complete",
                s
            )),
```

- [ ] **Step 5: Extend the existing `WorkStatus` tests**

Find `test_work_status_from_str_and_display` in `src/task.rs`'s test module and add these assertions inside it (don't create a new test — extend the existing one, matching how it already checks each variant):

```rust
        assert_eq!(WorkStatus::from_str("changes-requested").unwrap(), WorkStatus::ChangesRequested);
        assert_eq!(WorkStatus::from_str("changes").unwrap(), WorkStatus::ChangesRequested);
        assert_eq!(WorkStatus::ChangesRequested.to_string(), "changes-requested");
```

Find `test_task_serialize_work_status_kebab_case` and add a second assertion for the new variant (or add a new test named `test_task_serialize_changes_requested_kebab_case` if the existing one only checks one variant and you'd rather not overload it — check the existing test body first and follow its shape):

```rust
    #[test]
    fn test_task_serialize_changes_requested_kebab_case() {
        let json = serde_json::to_string(&WorkStatus::ChangesRequested).unwrap();
        assert_eq!(json, "\"changes-requested\"");
    }
```

- [ ] **Step 6: Fix any `WorkStatus`-exhaustive `match` that the compiler now flags**

```bash
cargo build --lib --bins --tests --features tui 2>&1 | grep -B2 "non-exhaustive\|missing match arm"
```

`WorkStatus` is only matched exhaustively in its own `Display` impl (just updated) and nowhere else in the codebase as of this plan's writing — every other usage goes through `Option<WorkStatus>` with `if let`/`.map()`, which doesn't require exhaustiveness. If this command finds anything else, add the missing arm following the pattern of its neighbors.

- [ ] **Step 7: Run the full test suite in both feature configurations**

```bash
cargo test --lib
cargo test --lib --features tui
```

Both must be green except the pre-existing, unrelated `auth::tests::test_write_read_delete_token_public_api` failure (environmental — confirmed failing on unmodified `main` too under this sandbox; passes when run with `dangerouslyDisableSandbox: true`. Not yours to fix.)

- [ ] **Step 8: Commit**

```bash
git add src/note.rs src/task.rs
git commit -m "feat: add append_to_note primitive and WorkStatus::ChangesRequested"
```

---

### Task 2: CLI — `task note append`

**Files:**
- Modify: `src/cli.rs` (`NoteCommand::Append` variant)
- Modify: `src/bin/task.rs` (match arm)
- Modify: `src/bin/task_tui.rs` (match arm — same as task.rs, do not skip)

**Interfaces:**
- Consumes: `note::append_to_note` from Task 1.
- Produces: `task note append <slug> --body "<text>"` CLI surface, identical in both binaries.

- [ ] **Step 1: Add the `NoteCommand::Append` variant to `src/cli.rs`**

In the `NoteCommand` enum (starts around line 247), add this variant after `Rm` and before `Link`:

```rust
    /// Append a section to a note's existing body
    Append {
        /// Note slug
        slug: String,

        /// Markdown text to append (including its own header, if any)
        #[arg(long)]
        body: String,
    },
```

- [ ] **Step 2: Add the match arm to `src/bin/task.rs`**

Find the `NoteCommand::Rm { slug } => { ... }` arm (around line 146-150) and insert this immediately after its closing `}`, before `NoteCommand::Link`:

```rust
                NoteCommand::Append { slug, body } => {
                    let file_path = task::note::append_to_note(&dir, &slug, &body).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());
                    Ok(())
                }
```

- [ ] **Step 3: Add the identical match arm to `src/bin/task_tui.rs`**

Same insertion, same code, in the equivalent spot in `src/bin/task_tui.rs`'s own copy of the `NoteCommand` match (around line 145-150, right after its own `NoteCommand::Rm` arm):

```rust
                NoteCommand::Append { slug, body } => {
                    let file_path = task::note::append_to_note(&dir, &slug, &body).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());
                    Ok(())
                }
```

- [ ] **Step 4: Build both binaries**

```bash
cargo build --bin task
cargo build --bin task-tui --features tui
```

Both must succeed with zero errors.

- [ ] **Step 5: Manual smoke test against a scratch database**

Do not use the real database at `~/Documents/Mark-main/Tasks/tasks.db` for this. Use a throwaway path:

```bash
rm -f /tmp/append-smoke.db
cargo run --bin task -- --file /tmp/append-smoke.db note add "Test Thread"
# Note the printed file path's slug (it will be "test-thread")
cargo run --bin task -- --file /tmp/append-smoke.db note append test-thread --body "## Round 1"
cargo run --bin task -- --file /tmp/append-smoke.db note show test-thread
# Expected output: "# Test Thread" followed by a blank line then "## Round 1"
cargo run --bin task -- --file /tmp/append-smoke.db note append test-thread --body "## Round 2"
cargo run --bin task -- --file /tmp/append-smoke.db note show test-thread
# Expected: both "## Round 1" and "## Round 2" present, separated by a blank line
cargo run --bin task -- --file /tmp/append-smoke.db note append missing-slug --body "x"
# Expected: a non-zero exit and an error message, not a panic
rm -f /tmp/append-smoke.db
rm -rf /tmp/Notes  # note.rs's discover/write logic may create this alongside the scratch db depending on dir resolution — clean up if present
```

Paste the real terminal output of each command into your report — do not describe what you expect to happen instead of running it.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/bin/task.rs src/bin/task_tui.rs
git commit -m "feat: add task note append CLI command"
```

---

### Task 3: Server API — `GET`/`POST /api/tasks/:id/review`

**Files:**
- Modify: `src/server.rs` (`review_thread_slug` helper, `FeedbackRequest`, `get_task_review`, `add_task_review_feedback`, router)
- Test: inline `#[cfg(test)] mod tests` in `src/server.rs`

**Interfaces:**
- Consumes: `note::append_to_note` and `WorkStatus::ChangesRequested` from Task 1. Reuses the existing `NoteResponse` struct and `notes_dir` helper already in `server.rs` — do not create a new response type.
- Produces: `GET /api/tasks/:id/review` → `Option<NoteResponse>` JSON (`null` if no thread yet). `POST /api/tasks/:id/review` with `{"text": "..."}` → `NoteResponse` JSON; creates the thread if absent, appends a `## Feedback — <date>` section, links the note to the task, sets `work_status` to `changes-requested`.

- [ ] **Step 1: Add the slug helper**

Add immediately after the existing `fn notes_dir(state: &AppState) -> PathBuf { ... }` function (around line 420-426):

```rust
fn review_thread_slug(task_id: u32) -> String {
    format!("task-{}-review-thread", task_id)
}
```

- [ ] **Step 2: Add the `GET` handler**

Add this near the other note-related handlers (e.g. right after `create_task_note`, which ends around line 506):

```rust
async fn get_task_review(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Option<NoteResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if task_file.find_task(id).is_none() {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)));
    }
    let dir = notes_dir(&state);
    let slug = review_thread_slug(id);
    let note = crate::note::read_note(&dir.join(format!("{}.md", slug)))
        .ok()
        .map(NoteResponse::from);
    Ok(Json(note))
}
```

- [ ] **Step 3: Add the `POST` handler and its request type**

Add immediately after `get_task_review`:

```rust
#[derive(Deserialize)]
pub struct FeedbackRequest {
    pub text: String,
}

async fn add_task_review_feedback(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = req.text.trim();
    if text.is_empty() {
        return Err(app_error(StatusCode::BAD_REQUEST, "feedback text must not be empty"));
    }

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let dir = notes_dir(&state);
    let slug = review_thread_slug(id);
    let note_path = dir.join(format!("{}.md", slug));

    let task_title = task_file
        .find_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?
        .title
        .clone();

    if !note_path.exists() {
        crate::note::write_note(
            &dir,
            &crate::note::Note { slug: slug.clone(), title: format!("Review — {}", task_title), body: String::new() },
        )
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    let today = chrono::Local::now().date_naive().format("%Y-%m-%d");
    let section = format!("## Feedback — {}\n\n{}", today, text);
    crate::note::append_to_note(&dir, &slug, &section)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    {
        let t = task_file.find_task_mut(id).expect("checked above");
        if !t.notes.contains(&slug) {
            t.notes.push(slug.clone());
        }
        t.work_status = Some(crate::task::WorkStatus::ChangesRequested);
        t.updated = Some(chrono::Utc::now());
    }
    db::save(&state.db_path, &task_file)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let note = crate::note::read_note(&note_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(NoteResponse::from(note)))
}
```

- [ ] **Step 4: Add the route**

In `router_with_static` (or wherever the `.route("/api/tasks/:id/notes", ...)` line lives, around line 585), add a new route line:

```rust
        .route("/api/tasks/:id/review", get(get_task_review).post(add_task_review_feedback))
```

- [ ] **Step 5: Write the tests**

Add to `src/server.rs`'s `#[cfg(test)] mod tests`, near the other notes-related tests:

```rust
    #[tokio::test]
    async fn test_get_task_review_returns_null_when_no_thread_exists() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has no thread yet".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json.is_null());
    }

    #[tokio::test]
    async fn test_post_task_review_creates_thread_and_sets_changes_requested() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Waiting on review".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: Some("bot".to_string()),
                effort: None, work_status: Some(crate::task::WorkStatus::WaitingForReview),
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        let body = serde_json::json!({ "text": "Please fix the error handling." });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["slug"], "task-1-review-thread");
        assert!(json["body"].as_str().unwrap().contains("## Feedback —"));
        assert!(json["body"].as_str().unwrap().contains("Please fix the error handling."));

        let tf = db::load(&state.db_path).unwrap();
        let task = tf.find_task(1).unwrap();
        assert_eq!(task.work_status, Some(crate::task::WorkStatus::ChangesRequested));
    }

    #[tokio::test]
    async fn test_post_task_review_appends_to_existing_thread() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Multi-round".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        let body1 = serde_json::json!({ "text": "First round of feedback." });
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body1).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body2 = serde_json::json!({ "text": "Second round of feedback." });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body2).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(response).await;
        let full_body = json["body"].as_str().unwrap();
        assert!(full_body.contains("First round of feedback."));
        assert!(full_body.contains("Second round of feedback."));
        // First round's text must appear before the second's.
        let first_pos = full_body.find("First round of feedback.").unwrap();
        let second_pos = full_body.find("Second round of feedback.").unwrap();
        assert!(first_pos < second_pos);
    }

    #[tokio::test]
    async fn test_post_task_review_empty_text_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "T".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "text": "   " });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
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
    async fn test_post_task_review_links_note_to_task_without_duplicating() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "T".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        for text in ["Round one.", "Round two."] {
            let body = serde_json::json!({ "text": text });
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/tasks/1/review")
                        .header("host", "127.0.0.1")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        let tf = db::load(&state.db_path).unwrap();
        let task = tf.find_task(1).unwrap();
        let occurrences = task.notes.iter().filter(|s| s.as_str() == "task-1-review-thread").count();
        assert_eq!(occurrences, 1, "the review-thread slug must be linked exactly once, not duplicated on the second post");
    }
```

Check the exact field list your `Task { ... }` literals need against `src/task.rs`'s current `Task` struct before running — this plan was written against the struct as it exists after Task 1 lands (which added no new `Task` fields, only a new `WorkStatus` variant, so no literal shape changes from that task).

- [ ] **Step 6: Run the tests**

```bash
cargo test --lib server::
cargo build --bin task_server
```

- [ ] **Step 7: Run the full suite**

```bash
cargo test --lib
```

Green except the known pre-existing `auth` failure.

- [ ] **Step 8: Commit**

```bash
git add src/server.rs
git commit -m "feat: add GET/POST /api/tasks/:id/review endpoints"
```

---

### Task 4: Web frontend — Review panel

**Files:**
- Modify: `web/package.json` (add `react-markdown`)
- Modify: `web/src/api.ts` (`fetchTaskReview`, `postTaskFeedback`)
- Create: `web/src/components/ReviewPanel.tsx`
- Modify: `web/src/App.tsx` (state, effect, handler, `WORK_STATUS_*` constants, inspector layout, Notes-list filtering)

**Interfaces:**
- Consumes: `GET`/`POST /api/tasks/:id/review` from Task 3.
- Produces: nothing consumed by later tasks — this is the last task in the plan.

- [ ] **Step 1: Add the dependency**

```bash
cd web && npm install react-markdown
```

- [ ] **Step 2: Add the API functions**

Append to `web/src/api.ts`:

```typescript
export async function fetchTaskReview(taskId: number): Promise<Note | null> {
  return jsonOrThrow(await fetch(`/api/tasks/${taskId}/review`));
}

export async function postTaskFeedback(taskId: number, text: string): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${taskId}/review`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ text }),
    }),
  );
}
```

(`Note` is already imported in `api.ts` — check the top of the file; it's used by `fetchTaskNotes`/`createTaskNote` already, so no new import needed.)

- [ ] **Step 3: Create `web/src/components/ReviewPanel.tsx`**

```tsx
import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import type { Note } from '../types';
import { Button } from './Button';

interface ReviewPanelProps {
  review: Note | null;
  onSendFeedback: (text: string) => void;
}

export function ReviewPanel({ review, onSendFeedback }: ReviewPanelProps) {
  const [text, setText] = useState('');

  function submit() {
    const trimmed = text.trim();
    if (!trimmed) return;
    onSendFeedback(trimmed);
    setText('');
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8 }}>
      <div
        style={{
          fontFamily: 'var(--font-display)',
          fontWeight: 700,
          fontSize: 11,
          letterSpacing: '0.08em',
          textTransform: 'uppercase',
          color: 'var(--fg-5)',
        }}
      >
        Review
      </div>

      {review ? (
        <div
          style={{
            border: '1px solid var(--hairline)',
            borderRadius: 6,
            padding: '10px 12px',
            fontSize: 13,
            color: 'var(--fg-2)',
            maxHeight: 320,
            overflowY: 'auto',
          }}
        >
          <ReactMarkdown>{review.body}</ReactMarkdown>
        </div>
      ) : (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>No review thread yet.</div>
      )}

      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="Leave feedback for another round…"
        rows={3}
        style={{
          width: '100%',
          background: 'var(--ink-2)',
          border: '1px solid var(--hairline)',
          borderRadius: 4,
          color: 'var(--fg-1)',
          fontSize: 13,
          fontFamily: 'inherit',
          padding: '6px 8px',
          resize: 'vertical',
          boxSizing: 'border-box',
        }}
      />
      <Button size="sm" onClick={submit}>Send feedback</Button>
    </div>
  );
}
```

- [ ] **Step 4: Wire it into `web/src/App.tsx`**

Add the import near the other component imports (alongside `NotesSection`):

```typescript
import { ReviewPanel } from './components/ReviewPanel';
```

Add `fetchTaskReview` and `postTaskFeedback` to the existing `import { ... } from './api';` line.

Add state next to `const [taskNotes, setTaskNotes] = useState<Note[]>([]);`:

```typescript
  const [review, setReview] = useState<Note | null>(null);
```

Add an effect next to the existing `taskNotes`-fetching effect (which fetches on `[selected?.id]` and clears to `[]` when nothing is selected — mirror its shape exactly):

```typescript
  useEffect(() => {
    if (!selected) {
      setReview(null);
      return;
    }
    fetchTaskReview(selected.id)
      .then(setReview)
      .catch((e) => setError(String(e)));
  }, [selected?.id]);
```

Add a handler next to the other `handle*` functions (e.g. near `handleUnlinkNote`):

```typescript
  async function handleSendFeedback(text: string) {
    if (!selected) return;
    try {
      const note = await postTaskFeedback(selected.id, text);
      setReview(note);
      const updated = { ...selected, work_status: 'changes-requested' as const };
      setSelected(updated);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    } catch (e) {
      setError(String(e));
    }
  }
```

- [ ] **Step 5: Add `changes-requested` to the `WORK_STATUS_*` constants**

Find `WORK_STATUS_OPTIONS`, `WORK_STATUS_LABEL`, `WORK_STATUS_COLOR` (defined together, `WORK_STATUS_OPTIONS` has a `''`/`'None'` entry first, then `todo`/`in-progress`/`waiting-for-review`/`complete`). Add a `changes-requested` entry to each, positioned after `waiting-for-review` and before `complete` in all three (matching the natural lifecycle order: todo → in-progress → waiting-for-review → changes-requested → complete):

```typescript
const WORK_STATUS_OPTIONS = [
  { value: '', label: 'None' },
  { value: 'todo', label: 'To Do' },
  { value: 'in-progress', label: 'In Progress' },
  { value: 'waiting-for-review', label: 'Waiting for Review' },
  { value: 'changes-requested', label: 'Changes Requested' },
  { value: 'complete', label: 'Complete' },
];

const WORK_STATUS_LABEL: Record<string, string> = {
  'todo': 'To Do',
  'in-progress': 'In Progress',
  'waiting-for-review': 'Waiting for Review',
  'changes-requested': 'Changes Requested',
  'complete': 'Complete',
};

const WORK_STATUS_COLOR: Record<string, string> = {
  'todo': 'var(--fg-4)',
  'in-progress': 'var(--citrine)',
  'waiting-for-review': 'var(--cyan)',
  'changes-requested': 'var(--danger)',
  'complete': 'var(--teal)',
};
```

- [ ] **Step 6: Update `web/src/types.ts`'s `Task.work_status` union**

Find `work_status?: 'todo' | 'in-progress' | 'waiting-for-review' | 'complete';` and add the new value:

```typescript
  work_status?: 'todo' | 'in-progress' | 'waiting-for-review' | 'changes-requested' | 'complete';
```

- [ ] **Step 7: Place `<ReviewPanel>` in the inspector layout**

Find the JSX structure: a `</div>` closes the fields column (which contains the `FieldRow`s including "Work status"), immediately followed by `<NotesSection notes={taskNotes} onCreate={handleCreateNote} onOpen={handleOpenNote} onUnlink={handleUnlinkNote} />`. `ReviewPanel` is a whole content block like `NotesSection`, not a single-line field — it does **not** go inside the fields `<div>` between `FieldRow`s. Place it as a sibling block, before `<NotesSection>`, so review feedback is visually prioritized above the general notes list:

```tsx
            <ReviewPanel review={review} onSendFeedback={handleSendFeedback} />

            <NotesSection
              notes={taskNotes}
              onCreate={handleCreateNote}
              onOpen={handleOpenNote}
              onUnlink={handleUnlinkNote}
            />
```

- [ ] **Step 8: Filter the review-thread note out of the general Notes list**

The `NotesSection` component receives `notes={taskNotes}`. Compute a filtered list right before that JSX (or as a `useMemo` near the other derived values — either is fine, prefer a plain `const` computed inline if `taskNotes`/`selected` are already in scope at that point in the render):

```typescript
  const reviewSlug = selected ? `task-${selected.id}-review-thread` : null;
  const visibleNotes = taskNotes.filter((n) => n.slug !== reviewSlug);
```

Place this near the top of the component body (with the other derived `const`s, e.g. near `agentOptions`), then change the `NotesSection` usage from `notes={taskNotes}` to `notes={visibleNotes}`.

- [ ] **Step 9: Build**

```bash
cd web && npm run build
```

Must be clean — 0 TypeScript errors, 0 vite errors. Paste the real output in your report. There is no frontend test runner in this project (confirmed: `package.json` has no `test` script) — a clean build is the acceptance bar, same as every prior frontend task in this project's history.

- [ ] **Step 10: Commit**

```bash
git add web/package.json web/package-lock.json web/src/api.ts web/src/components/ReviewPanel.tsx web/src/App.tsx web/src/types.ts
git commit -m "feat: add Review panel with markdown-rendered feedback thread"
```

---

## Final integration check (not a subagent task — do this yourself after all 4 tasks are reviewed and merged)

1. `cargo test --lib` and `cargo test --lib --features tui` — both green except the known `auth` failure.
2. `cd web && npm run build` — clean.
3. Restart the real `task_server` (release build) and confirm `curl http://127.0.0.1:4287/api/tasks | head` still returns real data — the live check that nothing in this feature broke the existing database.
4. Smoke-test end-to-end against a **scratch** database (never the real one, per this session's established convention) — pass a throwaway db path as the server's first CLI arg:
   - Create a task, `POST /api/tasks/:id/review` with feedback text via curl, confirm `work_status` becomes `changes-requested` via `GET /api/tasks/:id`.
   - `task --file <scratch db> note append task-<id>-review-thread --body "## Agent response — <date>\n\nDone."` and confirm via `task --file <scratch db> note show task-<id>-review-thread` that both sections appear in order.
   - Confirm the web UI's Review panel renders the thread with real markdown formatting (headers visibly styled, not raw `##` text) and that submitting feedback in the browser updates the Work status badge without a manual page refresh (the existing SSE subscription should also independently confirm this within ~300ms of the POST, per the file-watcher feature already shipped).
5. Once verified, perform the two global skill file updates from the spec's Section 6 (`~/.claude/skills/work-agent-tasks/SKILL.md` and `~/.claude/skills/task-manager/SKILL.md`) — outside this git repo, not part of the tracked tasks above, done by you directly afterward the same way the `work_status` feature's skill sync was handled.
