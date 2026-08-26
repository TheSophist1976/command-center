# SQLite Task Storage Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `tasks.md` with a SQLite database (`tasks.db`) as the source of truth for tasks, add the missing non-interactive CLI commands agents need (`list`/`show`/`edit`/`done`/`reopen`/`rm`), migrate existing `tasks.md` files automatically, and show the build version in the TUI.

**Architecture:** `Task`/`TaskFile` (`src/task.rs`) stay unchanged and remain the in-memory model everything works with. `src/storage.rs` (file I/O) and the markdown-serialization half of `src/parser.rs` are replaced by a new `src/db.rs` module backed by `rusqlite`. A new `src/commands.rs` module holds the task CRUD command bodies shared by both CLI binaries (`task`, `task-tui`), which are near-duplicates of each other today.

**Tech Stack:** Rust, `rusqlite` (bundled SQLite), `clap`, `chrono`, existing `tempfile`/`assert`-style integration tests.

**Spec:** `docs/superpowers/specs/2026-08-26-sqlite-backend-design.md`

## Global Constraints

- `Task`, `Status`, `Priority`, `Effort`, `Recurrence`, and `TaskFile` (including `next_id`/`format_version`) in `src/task.rs` do not change shape.
- `TaskFile.next_id` is computed as `MAX(id) + 1` on every `db::load` (or `1` if empty) — never persisted in SQLite, matching how it was never truly persisted before either (just cached in a markdown header).
- Task `id` is always supplied explicitly by callers (never DB-autoincrement-assigned) — this is what lets `tui.rs`, `todoist.rs`, and both CLI binaries keep their existing `let id = task_file.next_id; task_file.next_id += 1;` pattern unmodified.
- Notes (`src/note.rs`) are completely unaffected by this change.
- No `task export`/`task import` tooling.
- Old `tasks.md` is left on disk, untouched, after migration — the app just stops reading it.
- New dependency: `rusqlite = { version = "0.32", features = ["bundled"] }`, added as a normal (non-optional, non-`tui`-gated) dependency, since the plain `task` binary (no `tui` feature) needs it too.
- Remove the `fs2` dependency once `src/storage.rs`'s manual file lock is deleted (confirmed unused anywhere else in the codebase).

---

### Task 1: Add `rusqlite` dependency, remove `fs2`

**Files:**
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: `rusqlite` crate available to all subsequent tasks via `use rusqlite::{...}`.

- [ ] **Step 1: Edit `Cargo.toml`**

In the `[dependencies]` table, replace the `fs2 = "0.4"` line with:

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

- [ ] **Step 2: Build to fetch the new dependency and confirm nothing references `fs2` yet**

Run: `cargo build --features tui`
Expected: builds successfully (no code references `rusqlite` yet, and `fs2`'s only usage in `src/storage.rs` still compiles since we haven't touched it — wait, `fs2` was removed from `Cargo.toml` but `src/storage.rs` still does `use fs2::FileExt;`). Expected instead: **build fails** with `error[E0432]: unresolved import 'fs2'` in `src/storage.rs`. This is expected and confirms `storage.rs` is the only `fs2` consumer — it gets replaced in Task 5.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: swap fs2 for rusqlite ahead of SQLite storage backend"
```

(The build failure from Step 2 is expected and resolved by Task 5; do not attempt to fix `storage.rs` in this task.)

---

### Task 2: Core `db.rs` module — schema, connection, load, save

**Files:**
- Create: `src/db.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/db.rs`

**Interfaces:**
- Consumes: `crate::task::{Task, TaskFile, Status, Priority, Effort, Recurrence}` (all `FromStr`/`Display` already implemented in `src/task.rs`); `crate::config::{config_path, read_config_value_from, expand_tilde}` (existing functions, same signatures as used in current `src/storage.rs`).
- Produces (used by later tasks):
  - `pub fn resolve_file_path(flag: Option<&str>) -> PathBuf`
  - `pub fn load(path: &Path) -> Result<TaskFile, String>` (no `strict` parameter — that was markdown-specific)
  - `pub fn save(path: &Path, task_file: &TaskFile) -> Result<(), String>`

This task does NOT yet include migration (Task 3) or backups (Task 4) — `load` on a missing file just creates an empty schema.

- [ ] **Step 1: Write the failing tests**

Create `src/db.rs` with just the test module first (no implementation), so the tests fail to compile — this is the RED step:

```rust
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use chrono::{DateTime, NaiveDate, Utc};
use std::str::FromStr;

use crate::task::{Effort, Priority, Recurrence, Status, Task, TaskFile};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::env;

    fn sample_task(id: u32, title: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            status: Status::Open,
            priority: Priority::High,
            tags: vec!["alpha".to_string()],
            created: Utc::now(),
            updated: None,
            description: Some("Some description".to_string()),
            due_date: None,
            project: None,
            recurrence: None,
            notes: vec![],
            agent: None,
            effort: None,
        }
    }

    #[test]
    fn test_resolve_file_path_with_flag() {
        let p = resolve_file_path(Some("/tmp/my-tasks.db"));
        assert_eq!(p, PathBuf::from("/tmp/my-tasks.db"));
    }

    #[test]
    fn test_resolve_file_path_default() {
        unsafe { env::remove_var("TASK_FILE") };
        let p = resolve_file_path_inner(None, None);
        assert_eq!(p, PathBuf::from("tasks.db"));
    }

    #[test]
    fn test_resolve_file_path_config_default_dir() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "default-dir: /my/notes\n").unwrap();
        unsafe { env::remove_var("TASK_FILE") };
        let p = resolve_file_path_inner(None, Some(&config_path));
        assert_eq!(p, PathBuf::from("/my/notes/tasks.db"));
    }

    #[test]
    fn test_load_nonexistent_creates_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let tf = load(&path).unwrap();
        assert!(tf.tasks.is_empty());
        assert_eq!(tf.next_id, 1);
        assert!(path.exists());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut tf = TaskFile::new();
        tf.tasks.push(sample_task(1, "Test roundtrip"));
        tf.next_id = 2;
        save(&path, &tf).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].title, "Test roundtrip");
        assert_eq!(loaded.tasks[0].priority, Priority::High);
        assert_eq!(loaded.tasks[0].tags, vec!["alpha"]);
        assert_eq!(loaded.tasks[0].description, Some("Some description".to_string()));
        assert_eq!(loaded.next_id, 2);
    }

    #[test]
    fn test_save_overwrites_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut tf = TaskFile::new();
        tf.tasks.push(sample_task(1, "Original"));
        save(&path, &tf).unwrap();

        let tf2 = TaskFile::new();
        save(&path, &tf2).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.tasks.len(), 0);
    }

    #[test]
    fn test_roundtrip_preserves_all_optional_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 7,
            title: "Full task".to_string(),
            status: Status::Done,
            priority: Priority::Critical,
            tags: vec!["a".to_string(), "b".to_string()],
            created: Utc::now(),
            updated: Some(Utc::now()),
            description: Some("desc".to_string()),
            due_date: NaiveDate::from_ymd_opt(2026, 12, 31),
            project: Some("My Project: v2".to_string()),
            recurrence: Some(Recurrence::from_str("weekly:fri").unwrap()),
            notes: vec!["note-a".to_string(), "note-b".to_string()],
            agent: Some("command-center".to_string()),
            effort: Some(Effort::Medium),
        });
        save(&path, &tf).unwrap();

        let loaded = load(&path).unwrap();
        let t = &loaded.tasks[0];
        assert_eq!(t.status, Status::Done);
        assert_eq!(t.priority, Priority::Critical);
        assert_eq!(t.tags, vec!["a", "b"]);
        assert!(t.updated.is_some());
        assert_eq!(t.due_date, NaiveDate::from_ymd_opt(2026, 12, 31));
        assert_eq!(t.project, Some("My Project: v2".to_string()));
        assert_eq!(t.recurrence, Some(Recurrence::from_str("weekly:fri").unwrap()));
        assert_eq!(t.notes, vec!["note-a", "note-b"]);
        assert_eq!(t.agent, Some("command-center".to_string()));
        assert_eq!(t.effort, Some(Effort::Medium));
    }

    #[test]
    fn test_next_id_derived_from_max_existing_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut tf = TaskFile::new();
        tf.tasks.push(sample_task(5, "Five"));
        tf.tasks.push(sample_task(2, "Two"));
        save(&path, &tf).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.next_id, 6);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail to compile**

Run: `cargo test --lib db:: 2>&1 | head -30`
Expected: FAIL with errors like `cannot find function 'resolve_file_path' in this scope`, `cannot find function 'load'`, etc.

- [ ] **Step 3: Implement `db.rs` above the test module**

Add this above the `#[cfg(test)]` block in `src/db.rs`:

```rust
const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS tasks (
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
);
";

pub fn resolve_file_path(flag: Option<&str>) -> PathBuf {
    resolve_file_path_inner(flag, crate::config::config_path().as_deref())
}

fn resolve_file_path_inner(flag: Option<&str>, config_path: Option<&Path>) -> PathBuf {
    if let Some(path) = flag {
        return PathBuf::from(path);
    }
    if let Ok(env_path) = std::env::var("TASK_FILE") {
        if !env_path.is_empty() {
            return PathBuf::from(env_path);
        }
    }
    if let Some(cfg_path) = config_path {
        if let Some(default_dir) = crate::config::read_config_value_from(cfg_path, "default-dir") {
            if !default_dir.is_empty() {
                return crate::config::expand_tilde(&default_dir).join("tasks.db");
            }
        }
    }
    PathBuf::from("tasks.db")
}

fn open_conn(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("Failed to set busy timeout: {}", e))?;
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| format!("Failed to initialize schema: {}", e))?;
    Ok(conn)
}

fn list_to_text(v: &[String]) -> String {
    v.join(",")
}

fn text_to_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let id: u32 = row.get(0)?;
    let title: String = row.get(1)?;
    let status_s: String = row.get(2)?;
    let priority_s: String = row.get(3)?;
    let tags_s: String = row.get(4)?;
    let created_s: String = row.get(5)?;
    let updated_s: Option<String> = row.get(6)?;
    let description: Option<String> = row.get(7)?;
    let due_s: Option<String> = row.get(8)?;
    let project: Option<String> = row.get(9)?;
    let recur_s: Option<String> = row.get(10)?;
    let notes_s: String = row.get(11)?;
    let agent: Option<String> = row.get(12)?;
    let effort_s: Option<String> = row.get(13)?;

    Ok(Task {
        id,
        title,
        status: Status::from_str(&status_s).unwrap_or(Status::Open),
        priority: Priority::from_str(&priority_s).unwrap_or(Priority::Medium),
        tags: text_to_list(&tags_s),
        created: DateTime::parse_from_rfc3339(&created_s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated: updated_s
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
        description,
        due_date: due_s.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        project,
        recurrence: recur_s.and_then(|s| Recurrence::from_str(&s).ok()),
        notes: text_to_list(&notes_s),
        agent,
        effort: effort_s.and_then(|s| Effort::from_str(&s).ok()),
    })
}

fn insert_task_row(conn: &Connection, t: &Task) -> Result<(), String> {
    conn.execute(
        "INSERT INTO tasks (id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
        ],
    )
    .map_err(|e| format!("Failed to insert task {}: {}", t.id, e))?;
    Ok(())
}

pub fn load(path: &Path) -> Result<TaskFile, String> {
    let conn = open_conn(path)?;

    let mut stmt = conn
        .prepare("SELECT id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort FROM tasks ORDER BY id")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;
    let rows = stmt
        .query_map([], row_to_task)
        .map_err(|e| format!("Failed to query tasks: {}", e))?;

    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row.map_err(|e| format!("Failed to read task row: {}", e))?);
    }

    let mut task_file = TaskFile::new();
    let max_id = tasks.iter().map(|t| t.id).max().unwrap_or(0);
    task_file.next_id = max_id + 1;
    task_file.tasks = tasks;
    Ok(task_file)
}

pub fn save(path: &Path, task_file: &TaskFile) -> Result<(), String> {
    let mut conn = open_conn(path)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    tx.execute("DELETE FROM tasks", [])
        .map_err(|e| format!("Failed to clear tasks: {}", e))?;
    for t in &task_file.tasks {
        insert_task_row(&tx, t)?;
    }
    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib db::`
Expected: PASS — all tests in `src/db.rs` green.

- [ ] **Step 5: Register the module in `src/lib.rs`**

In `src/lib.rs`, add `pub mod db;` alongside the other always-on modules (near `pub mod parser;`):

```rust
pub mod auth;
pub mod cli;
pub mod config;
pub mod db;
pub mod note;
pub mod parser;
pub mod storage;
pub mod task;
```

(`storage` stays for now — it's removed in Task 5 once all callers have moved over.)

- [ ] **Step 6: Run the full test suite to confirm nothing else broke**

Run: `cargo build --features tui && cargo test --lib`
Expected: PASS (note: `storage.rs` still references the now-removed `fs2` crate, so this step is expected to fail the same way as Task 1 Step 2 until Task 5 — confirm the *only* failure is the `fs2` import error in `storage.rs`, and `db.rs` tests pass when run in isolation per Step 4).

- [ ] **Step 7: Commit**

```bash
git add src/db.rs src/lib.rs
git commit -m "feat: add SQLite-backed db module with load/save"
```

---

### Task 3: Migration from `tasks.md`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Consumes: `crate::parser::parse(content: &str, strict: bool) -> Result<TaskFile, Vec<ParseError>>` (existing, unchanged).
- Produces: `db::load` now transparently migrates an existing `tasks.md` sitting next to the resolved `.db` path, the first time `load` is called against a path that doesn't exist yet.

- [ ] **Step 1: Write the failing test**

Add to `src/db.rs`'s test module:

```rust
    #[test]
    fn test_load_migrates_existing_markdown() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let md_path = dir.path().join("tasks.md");
        let content = "<!-- format:2 -->\n<!-- next-id:3 -->\n\n# Tasks\n\n## [ ] Task A\n<!-- id:1 priority:high tags:x,y created:2025-01-01T00:00:00+00:00 -->\n\nDescription here.\n\n## [x] Task B\n<!-- id:2 priority:low created:2025-01-02T00:00:00+00:00 updated:2025-01-03T00:00:00+00:00 -->\n";
        std::fs::write(&md_path, content).unwrap();

        let tf = load(&db_path).unwrap();
        assert_eq!(tf.tasks.len(), 2);
        assert_eq!(tf.next_id, 3);
        let task_a = tf.tasks.iter().find(|t| t.id == 1).unwrap();
        assert_eq!(task_a.title, "Task A");
        assert_eq!(task_a.priority, Priority::High);
        assert_eq!(task_a.tags, vec!["x", "y"]);
        assert_eq!(task_a.description, Some("Description here.".to_string()));

        // tasks.md is left untouched
        assert!(md_path.exists());
        assert_eq!(std::fs::read_to_string(&md_path).unwrap(), content);
    }

    #[test]
    fn test_load_no_markdown_no_db_creates_empty() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let tf = load(&db_path).unwrap();
        assert!(tf.tasks.is_empty());
        assert!(db_path.exists());
    }

    #[test]
    fn test_load_does_not_remigrate_once_db_exists() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let md_path = dir.path().join("tasks.md");
        std::fs::write(&md_path, "<!-- format:2 -->\n<!-- next-id:2 -->\n\n# Tasks\n\n## [ ] From markdown\n<!-- id:1 priority:medium created:2025-01-01T00:00:00+00:00 -->\n").unwrap();

        // First load migrates.
        load(&db_path).unwrap();

        // Now save an empty TaskFile directly to the DB (simulating the user deleting the task).
        save(&db_path, &TaskFile::new()).unwrap();

        // A second load must NOT re-migrate from tasks.md (which still has the old task).
        let tf = load(&db_path).unwrap();
        assert!(tf.tasks.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib db::test_load_migrates_existing_markdown db::test_load_no_markdown_no_db_creates_empty db::test_load_does_not_remigrate_once_db_exists`
Expected: FAIL — `test_load_migrates_existing_markdown` fails because migration doesn't exist yet (the DB will just be empty).

- [ ] **Step 3: Implement migration in `db.rs`**

Add this function, and call it from `load`:

```rust
fn migrate_from_markdown(db_path: &Path, conn: &Connection) -> Result<(), String> {
    let md_path = db_path.with_file_name("tasks.md");
    if !md_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&md_path)
        .map_err(|e| format!("Failed to read {} during migration: {}", md_path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(());
    }
    let task_file = crate::parser::parse(&content, false).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        format!("Failed to migrate {}: {}", md_path.display(), msgs.join("\n"))
    })?;
    for t in &task_file.tasks {
        insert_task_row(conn, t)?;
    }
    Ok(())
}
```

Add `use std::fs;` to the top of `src/db.rs`.

Change `load` to run migration only when the `.db` file didn't exist before opening it:

```rust
pub fn load(path: &Path) -> Result<TaskFile, String> {
    let needs_migration = !path.exists();
    let conn = open_conn(path)?;
    if needs_migration {
        migrate_from_markdown(path, &conn)?;
    }

    let mut stmt = conn
        .prepare("SELECT id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort FROM tasks ORDER BY id")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;
    // ... (rest unchanged from Task 2)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib db::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/db.rs
git commit -m "feat: migrate existing tasks.md into SQLite on first load"
```

---

### Task 4: Daily backups via `VACUUM INTO`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Produces: `pub fn backup_daily(path: &Path)` — same name/signature as the current `src/storage.rs::backup_daily`, so Task 5's caller updates are a pure module-path change.

- [ ] **Step 1: Write the failing tests**

Add to `src/db.rs`'s test module:

```rust
    #[test]
    fn test_backup_daily_creates_backup_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut tf = TaskFile::new();
        tf.tasks.push(sample_task(1, "Backed up"));
        save(&path, &tf).unwrap();

        backup_daily(&path);

        let backup_dir = dir.path().join(".backups");
        assert!(backup_dir.exists());
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let backup_path = backup_dir.join(format!("tasks-{}.db", today));
        assert!(backup_path.exists());

        let backed_up = load(&backup_path).unwrap();
        assert_eq!(backed_up.tasks.len(), 1);
        assert_eq!(backed_up.tasks[0].title, "Backed up");
    }

    #[test]
    fn test_backup_daily_no_file_no_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.db");
        backup_daily(&path); // should not panic
        assert!(!dir.path().join(".backups").exists());
    }

    #[test]
    fn test_backup_daily_overwrites_same_day() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut tf = TaskFile::new();
        tf.tasks.push(sample_task(1, "Version 1"));
        save(&path, &tf).unwrap();
        backup_daily(&path);

        let mut tf2 = TaskFile::new();
        tf2.tasks.push(sample_task(1, "Version 2"));
        save(&path, &tf2).unwrap();
        backup_daily(&path);

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let backup_path = dir.path().join(".backups").join(format!("tasks-{}.db", today));
        let backed_up = load(&backup_path).unwrap();
        assert_eq!(backed_up.tasks[0].title, "Version 2");
    }

    #[test]
    fn test_backup_daily_prunes_old_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        save(&path, &TaskFile::new()).unwrap();
        let backup_dir = dir.path().join(".backups");
        fs::create_dir(&backup_dir).unwrap();
        for i in 1..=9 {
            fs::write(backup_dir.join(format!("tasks-2025-01-{:02}.db", i)), "old").unwrap();
        }
        backup_daily(&path);
        let entries: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("tasks-"))
            .collect();
        assert_eq!(entries.len(), 7);
        assert!(!backup_dir.join("tasks-2025-01-01.db").exists());
        assert!(!backup_dir.join("tasks-2025-01-02.db").exists());
        assert!(!backup_dir.join("tasks-2025-01-03.db").exists());
        assert!(backup_dir.join("tasks-2025-01-04.db").exists());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib db::test_backup_daily`
Expected: FAIL — `backup_daily` doesn't exist yet.

- [ ] **Step 3: Implement `backup_daily`**

Add to `src/db.rs`:

```rust
pub fn backup_daily(path: &Path) {
    if !path.exists() {
        return;
    }
    let parent = match path.parent() {
        Some(p) => p,
        None => return,
    };
    let backup_dir = parent.join(".backups");
    if fs::create_dir_all(&backup_dir).is_err() {
        return;
    }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let backup_path = backup_dir.join(format!("tasks-{}.db", today));
    // VACUUM INTO refuses to overwrite an existing file.
    let _ = fs::remove_file(&backup_path);

    if let Ok(conn) = Connection::open(path) {
        let _ = conn.execute("VACUUM INTO ?1", params![backup_path.to_string_lossy()]);
    }

    // Prune: keep only the 7 most recent backups.
    let entries = match fs::read_dir(&backup_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut backups: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("tasks-") && name.ends_with(".db") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    backups.sort();
    if backups.len() > 7 {
        let to_remove = backups.len() - 7;
        for name in &backups[..to_remove] {
            let _ = fs::remove_file(backup_dir.join(name));
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib db::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/db.rs
git commit -m "feat: back up tasks.db daily via VACUUM INTO"
```

---

### Task 5: Cut over every caller from `storage`/markdown to `db`, delete `storage.rs`

**Files:**
- Delete: `src/storage.rs`
- Modify: `src/lib.rs`, `src/bin/task.rs`, `src/bin/task_tui.rs`, `src/tui.rs`, `src/todoist.rs`

**Interfaces:**
- Consumes: `db::resolve_file_path`, `db::load(path)`, `db::save(path, &tf)`, `db::backup_daily(path)` (all from Task 2–4).

This task is a mechanical rename plus dropping the now-gone `strict` bool argument to `load`. No behavioral logic changes.

- [ ] **Step 1: Delete `src/storage.rs` and remove it from `src/lib.rs`**

```bash
rm src/storage.rs
```

In `src/lib.rs`, remove `pub mod storage;`:

```rust
pub mod auth;
pub mod cli;
pub mod config;
pub mod db;
pub mod note;
pub mod parser;
pub mod task;

#[cfg(feature = "tui")]
pub mod claude_session;
#[cfg(feature = "tui")]
pub mod todoist;
#[cfg(feature = "tui")]
pub mod tui;
```

- [ ] **Step 2: Update `src/bin/task.rs`**

Replace every `task::storage::` with `task::db::`, and change the two `load` call sites from `task::storage::load(&path, false)` to `task::db::load(&path)` (there is one at the top of `run()` — actually `run()` doesn't call load directly, only `resolve_file_path`/`backup_daily` — and several inside the `Note`/`Add` match arms).

Specifically:
- Line ~22: `let path = task::storage::resolve_file_path(cli.file.as_deref());` → `task::db::resolve_file_path(...)`
- Line ~23: `task::storage::backup_daily(&path);` → `task::db::backup_daily(&path);`
- All `task::storage::load(&path, false)` → `task::db::load(&path)`
- All `task::storage::save(&path, &task_file)` → `task::db::save(&path, &task_file)`

Use a project-wide search to confirm every occurrence in this file is updated:

```bash
grep -n "task::storage::" src/bin/task.rs
```

Expected after edits: no output.

- [ ] **Step 3: Update `src/bin/task_tui.rs` the same way**

```bash
grep -n "task::storage::" src/bin/task_tui.rs
```

Apply the same substitutions (`task::storage::resolve_file_path` → `task::db::resolve_file_path`, `task::storage::backup_daily` → `task::db::backup_daily`, and this file's own `Add`/`Note` match arms using `task::storage::load(&path, false)` / `task::storage::save`).

Expected after edits: `grep -n "task::storage::" src/bin/task_tui.rs` produces no output.

- [ ] **Step 4: Update `src/tui.rs`**

```bash
grep -n "storage::" src/tui.rs
```

This will show occurrences like:
- `use crate::storage;` (or similar import) → `use crate::db;`
- `storage::load(path, false)` (in `App::new`) → `db::load(path)`
- `storage::save(&self.file_path, &self.task_file)` (in `App::save`) → `db::save(...)`
- `storage::load(&new_path, false)` (in the "change default dir" input handler) → `db::load(&new_path)`

Also update the hardcoded `.join("tasks.md")` in the "change default dir" handler (`InputAction::EditDefaultDir`) to `.join("tasks.db")`:

```rust
let new_path = std::path::PathBuf::from(&trimmed).join("tasks.db");
```

Confirm no leftover references:

```bash
grep -n "storage::\|tasks\.md" src/tui.rs
```

Expected: no output (aside from anything intentionally referencing `tasks.md` for migration purposes, which does not belong in `tui.rs` — migration lives entirely in `db.rs`).

- [ ] **Step 5: Update `src/todoist.rs`**

```bash
grep -n "storage::" src/todoist.rs
```

`todoist.rs` uses `TaskFile` directly but check for any `storage::load`/`storage::save` calls in its non-test code and its `#[cfg(test)]` module; apply the same `storage::` → `db::` substitution and drop the `strict` bool argument from any `load` calls found.

- [ ] **Step 6: Full workspace build and test**

Run: `cargo build --features tui`
Expected: builds cleanly, no references to `fs2` or `storage` remain.

Run: `cargo test --features tui`
Expected: All tests pass. If any test still references `tasks.md` paths or the old `storage` module name, update them to `tasks.db` / `db` to match.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: replace markdown file storage with SQLite db module everywhere"
```

---

### Task 6: Trim `parser.rs` to migration-only surface

**Files:**
- Modify: `src/parser.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `parser::parse` (used by `db::migrate_from_markdown`) and `parser::parse_due_date_input` (used by `commands::add`/`commands::edit` in Task 7) remain; `parser::serialize` and its tests are deleted since nothing calls it once `storage.rs` is gone.

- [ ] **Step 1: Confirm `serialize` has no remaining callers**

```bash
grep -rn "parser::serialize\|::serialize(" src/ tests/
```

Expected: no output (Task 5 already removed the only caller, `storage::save`).

- [ ] **Step 2: Delete the `serialize` function and its dedicated tests**

In `src/parser.rs`, delete the `pub fn serialize(task_file: &TaskFile) -> String { ... }` function (the block starting `pub fn serialize` through its closing `}`, just before `parse_due_date_input`'s doc comment).

Delete the tests that exercise `serialize` specifically (these assert on markdown text output, which no longer exists as a code path): `test_round_trip`, `test_serialize_includes_format_version_2`, `test_serialize_includes_next_id`, `test_serialize_open_task`, `test_serialize_done_task`, `test_serialize_with_all_optional_fields`, `test_serialize_project_with_colon`, `test_serialize_project_roundtrip_with_special_chars`, and `test_notes_round_trip_single`/`test_notes_round_trip_multiple` (these two call `serialize` — confirm by reading each test body before deleting; if a test only calls `parse`, keep it).

Keep every other test — they test `parse()` and `parse_due_date_input()`, both of which remain live code (migration and CLI due-date parsing respectively).

- [ ] **Step 3: Run tests**

Run: `cargo test --lib parser::`
Expected: PASS — remaining tests (all `parse`/`parse_due_date_input` tests) still pass; no leftover references to deleted tests cause compile errors.

- [ ] **Step 4: Commit**

```bash
git add src/parser.rs
git commit -m "refactor: drop unused markdown serializer from parser.rs"
```

---

### Task 7: `src/commands.rs` — shared task CRUD command bodies

**Files:**
- Modify: `src/task.rs` (un-gate `find_task`/`remove_task` from `#[cfg(test)]`)
- Modify: `src/cli.rs` (add `List`/`Show`/`Edit`/`Done`/`Reopen`/`Rm` subcommands)
- Create: `src/commands.rs`
- Modify: `src/lib.rs` (register `pub mod commands;`)
- Modify: `src/bin/task.rs`, `src/bin/task_tui.rs` (dispatch new subcommands, replace inline `Add` logic with `commands::add`)

**Interfaces:**
- Consumes: `TaskFile::find_task(&self, id: u32) -> Option<&Task>`, `TaskFile::find_task_mut`, `TaskFile::remove_task(&mut self, id: u32) -> Option<Task>` (all in `src/task.rs`, `find_task`/`remove_task` currently `#[cfg(test)]`-gated — un-gate them here since production code now needs them); `db::load`, `db::save` (Task 2); `parser::parse_due_date_input` (Task 6); `task::next_due_date` (existing, `src/task.rs`).
- Produces:
  - `pub struct commands::AddArgs { title, priority, due, project, tags, agent, description }` (all `Option<String>` except `title: String`, `priority: String`)
  - `pub struct commands::ListArgs { status, agent, project, tag, due_before }` (all `Option<String>`)
  - `pub struct commands::EditArgs { title, priority, due, project, tags, agent, description, effort }` (all `Option<String>`)
  - `pub fn commands::add(path: &Path, args: AddArgs) -> Result<String, (i32, String)>`
  - `pub fn commands::list(path: &Path, args: ListArgs) -> Result<String, (i32, String)>`
  - `pub fn commands::show(path: &Path, id: u32) -> Result<String, (i32, String)>`
  - `pub fn commands::edit(path: &Path, id: u32, args: EditArgs) -> Result<String, (i32, String)>`
  - `pub fn commands::done(path: &Path, id: u32) -> Result<String, (i32, String)>`
  - `pub fn commands::reopen(path: &Path, id: u32) -> Result<String, (i32, String)>`
  - `pub fn commands::rm(path: &Path, id: u32) -> Result<String, (i32, String)>`

- [ ] **Step 1: Un-gate `find_task` and `remove_task` in `src/task.rs`**

In `src/task.rs`, remove the `#[cfg(test)]` attribute directly above `pub fn find_task` and directly above `pub fn remove_task` (leave `find_task_mut` as-is, it's already unconditionally `pub`):

```rust
    pub fn find_task(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn find_task_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn remove_task(&mut self, id: u32) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }
```

Run: `cargo build --features tui` — expected: still compiles (removing a `#[cfg(test)]` gate never breaks a build, it only makes the function available in more configurations).

- [ ] **Step 2: Write the failing tests for `commands.rs`**

Create `src/commands.rs`:

```rust
use std::path::Path;
use std::str::FromStr;

use chrono::Utc;

use crate::db;
use crate::parser;
use crate::task::{Effort, Priority, Status, Task};

fn parse_csv(s: Option<&str>) -> Vec<String> {
    s.unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub struct AddArgs {
    pub title: String,
    pub priority: String,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
}

pub struct ListArgs {
    pub status: Option<String>,
    pub agent: Option<String>,
    pub project: Option<String>,
    pub tag: Option<String>,
    pub due_before: Option<String>,
}

pub struct EditArgs {
    pub title: Option<String>,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub effort: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        (dir, path)
    }

    #[test]
    fn test_add_creates_task() {
        let (_dir, path) = setup();
        let msg = add(&path, AddArgs {
            title: "New task".to_string(),
            priority: "high".to_string(),
            due: Some("2026-12-31".to_string()),
            project: Some("Work".to_string()),
            tags: Some("a,b".to_string()),
            agent: Some("bot".to_string()),
            description: Some("desc".to_string()),
        }).unwrap();
        assert!(msg.contains("Created task 1"));

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].priority, Priority::High);
        assert_eq!(tf.tasks[0].tags, vec!["a", "b"]);
    }

    #[test]
    fn test_add_invalid_priority_errors() {
        let (_dir, path) = setup();
        let err = add(&path, AddArgs {
            title: "X".to_string(), priority: "urgent".to_string(), due: None,
            project: None, tags: None, agent: None, description: None,
        }).unwrap_err();
        assert_eq!(err.0, 1);
        assert!(err.1.contains("Invalid priority"));
    }

    #[test]
    fn test_list_filters_by_status_and_agent() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Open task".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("bot".to_string()), description: None }).unwrap();
        add(&path, AddArgs { title: "Other agent".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("human".to_string()), description: None }).unwrap();
        done(&path, 2).unwrap();

        let out = list(&path, ListArgs { status: Some("open".to_string()), agent: Some("bot".to_string()), project: None, tag: None, due_before: None }).unwrap();
        assert!(out.contains("Open task"));
        assert!(!out.contains("Other agent"));
    }

    #[test]
    fn test_list_no_matches() {
        let (_dir, path) = setup();
        let out = list(&path, ListArgs { status: None, agent: Some("nobody".to_string()), project: None, tag: None, due_before: None }).unwrap();
        assert_eq!(out, "No matching tasks.");
    }

    #[test]
    fn test_show_existing_task() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Show me".to_string(), priority: "low".to_string(), due: None, project: None, tags: None, agent: None, description: Some("body text".to_string()) }).unwrap();
        let out = show(&path, 1).unwrap();
        assert!(out.contains("Show me"));
        assert!(out.contains("priority: low"));
        assert!(out.contains("body text"));
    }

    #[test]
    fn test_show_missing_task_errors() {
        let (_dir, path) = setup();
        let err = show(&path, 99).unwrap_err();
        assert_eq!(err.0, 1);
        assert!(err.1.contains("not found"));
    }

    #[test]
    fn test_edit_updates_only_given_fields() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Original".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();
        edit(&path, 1, EditArgs { title: None, priority: Some("critical".to_string()), due: None, project: None, tags: None, agent: None, description: None, effort: None }).unwrap();

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].title, "Original");
        assert_eq!(tf.tasks[0].priority, Priority::Critical);
        assert!(tf.tasks[0].updated.is_some());
    }

    #[test]
    fn test_edit_missing_task_errors() {
        let (_dir, path) = setup();
        let err = edit(&path, 1, EditArgs { title: Some("x".to_string()), priority: None, due: None, project: None, tags: None, agent: None, description: None, effort: None }).unwrap_err();
        assert_eq!(err.0, 1);
    }

    #[test]
    fn test_done_marks_status_done() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Finish me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();
        let msg = done(&path, 1).unwrap();
        assert_eq!(msg, "Completed task 1");

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].status, Status::Done);
    }

    #[test]
    fn test_done_recurring_task_spawns_next_occurrence() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Recurring".to_string(), priority: "medium".to_string(), due: Some("2026-01-05".to_string()), project: None, tags: None, agent: None, description: None }).unwrap();
        {
            let mut tf = db::load(&path).unwrap();
            tf.tasks[0].recurrence = Some(crate::task::Recurrence::from_str("weekly").unwrap());
            db::save(&path, &tf).unwrap();
        }

        let msg = done(&path, 1).unwrap();
        assert!(msg.contains("Next occurrence: task 2"));

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks.len(), 2);
        let original = tf.find_task(1).unwrap();
        assert_eq!(original.status, Status::Done);
        let spawned = tf.find_task(2).unwrap();
        assert_eq!(spawned.status, Status::Open);
        assert_eq!(spawned.title, "Recurring");
        assert_eq!(spawned.due_date, chrono::NaiveDate::from_ymd_opt(2026, 1, 12));
    }

    #[test]
    fn test_done_already_done_is_a_noop_message() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "T".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();
        done(&path, 1).unwrap();
        let msg = done(&path, 1).unwrap();
        assert_eq!(msg, "Task 1 already done");
    }

    #[test]
    fn test_reopen_sets_status_open() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "T".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();
        done(&path, 1).unwrap();
        reopen(&path, 1).unwrap();
        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].status, Status::Open);
    }

    #[test]
    fn test_rm_deletes_task() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Doomed".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None }).unwrap();
        let msg = rm(&path, 1).unwrap();
        assert!(msg.contains("Doomed"));
        let tf = db::load(&path).unwrap();
        assert!(tf.tasks.is_empty());
    }

    #[test]
    fn test_rm_missing_task_errors() {
        let (_dir, path) = setup();
        let err = rm(&path, 1).unwrap_err();
        assert_eq!(err.0, 1);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail to compile**

Run: `cargo test --lib commands:: 2>&1 | head -30`
Expected: FAIL — `cannot find function 'add' in this scope`, etc. (the struct definitions exist but no functions yet).

- [ ] **Step 4: Implement the command functions**

Add these functions to `src/commands.rs`, above the `#[cfg(test)]` module:

```rust
pub fn add(path: &Path, args: AddArgs) -> Result<String, (i32, String)> {
    let mut task_file = db::load(path).map_err(|e| (1, e))?;
    let priority = Priority::from_str(&args.priority).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let due_date = args.due.as_deref().and_then(|d| parser::parse_due_date_input(d, today));
    let tags = parse_csv(args.tags.as_deref());

    let id = task_file.next_id;
    task_file.next_id += 1;
    task_file.tasks.push(Task {
        id,
        title: args.title.clone(),
        status: Status::Open,
        priority,
        tags,
        created: Utc::now(),
        updated: None,
        description: args.description,
        due_date,
        project: args.project,
        recurrence: None,
        notes: Vec::new(),
        agent: args.agent,
        effort: None,
    });
    db::save(path, &task_file).map_err(|e| (1, e))?;
    Ok(format!("Created task {}: {}", id, args.title))
}

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

    let mut tasks: Vec<&Task> = task_file
        .tasks
        .iter()
        .filter(|t| status_filter.map_or(true, |s| t.status == s))
        .filter(|t| args.agent.as_deref().map_or(true, |a| t.agent.as_deref() == Some(a)))
        .filter(|t| args.project.as_deref().map_or(true, |p| t.project.as_deref() == Some(p)))
        .filter(|t| args.tag.as_deref().map_or(true, |tag| t.tags.iter().any(|x| x == tag)))
        .filter(|t| due_before.map_or(true, |d| t.due_date.map_or(false, |td| td <= d)))
        .collect();

    tasks.sort_by(|a, b| {
        let date_cmp = match (a.due_date, b.due_date) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        };
        date_cmp.then(a.priority.cmp(&b.priority))
    });

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

pub fn show(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let task_file = db::load(path).map_err(|e| (1, e))?;
    let t = task_file.find_task(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;

    let mut out = format!(
        "[{}] {} ({})\n",
        if t.status == Status::Done { "x" } else { " " },
        t.title,
        t.id
    );
    out.push_str(&format!("priority: {}\n", t.priority));
    if !t.tags.is_empty() {
        out.push_str(&format!("tags: {}\n", t.tags.join(",")));
    }
    if let Some(d) = t.due_date {
        out.push_str(&format!("due: {}\n", d.format("%Y-%m-%d")));
    }
    if let Some(ref p) = t.project {
        out.push_str(&format!("project: {}\n", p));
    }
    if let Some(ref r) = t.recurrence {
        out.push_str(&format!("recur: {}\n", r));
    }
    if !t.notes.is_empty() {
        out.push_str(&format!("notes: {}\n", t.notes.join(",")));
    }
    if let Some(ref a) = t.agent {
        out.push_str(&format!("agent: {}\n", a));
    }
    if let Some(ref e) = t.effort {
        out.push_str(&format!("effort: {}\n", e));
    }
    out.push_str(&format!("created: {}\n", t.created.to_rfc3339()));
    if let Some(u) = t.updated {
        out.push_str(&format!("updated: {}\n", u.to_rfc3339()));
    }
    if let Some(ref desc) = t.description {
        out.push('\n');
        out.push_str(desc);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

pub fn edit(path: &Path, id: u32, args: EditArgs) -> Result<String, (i32, String)> {
    let mut task_file = db::load(path).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let priority = args.priority.as_deref().map(Priority::from_str).transpose().map_err(|e| (1, e))?;
    let effort = args.effort.as_deref().map(Effort::from_str).transpose().map_err(|e| (1, e))?;
    let due_date = args
        .due
        .as_deref()
        .map(|d| {
            parser::parse_due_date_input(d, today).ok_or_else(|| (1, format!("Invalid due date: '{}'", d)))
        })
        .transpose()?;
    let tags = args.tags.as_deref().map(|s| parse_csv(Some(s)));

    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    if let Some(title) = args.title {
        t.title = title;
    }
    if let Some(p) = priority {
        t.priority = p;
    }
    if let Some(d) = due_date {
        t.due_date = Some(d);
    }
    if let Some(p) = args.project {
        t.project = Some(p);
    }
    if let Some(tg) = tags {
        t.tags = tg;
    }
    if let Some(a) = args.agent {
        t.agent = Some(a);
    }
    if let Some(d) = args.description {
        t.description = Some(d);
    }
    if let Some(e) = effort {
        t.effort = Some(e);
    }
    t.updated = Some(Utc::now());
    let title = t.title.clone();

    db::save(path, &task_file).map_err(|e| (1, e))?;
    Ok(format!("Updated task {}: {}", id, title))
}

pub fn done(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::load(path).map_err(|e| (1, e))?;
    let idx = task_file
        .tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| (1, format!("Task {} not found", id)))?;

    if task_file.tasks[idx].status == Status::Done {
        return Ok(format!("Task {} already done", id));
    }

    task_file.tasks[idx].status = Status::Done;
    task_file.tasks[idx].updated = Some(Utc::now());

    let mut spawned = None;
    if let Some(recur) = task_file.tasks[idx].recurrence {
        let parent = task_file.tasks[idx].clone();
        let next_due = crate::task::next_due_date(&recur, parent.due_date);
        let new_id = task_file.next_id;
        task_file.next_id += 1;
        task_file.tasks.push(Task {
            id: new_id,
            title: parent.title.clone(),
            status: Status::Open,
            priority: parent.priority,
            tags: parent.tags.clone(),
            created: Utc::now(),
            updated: None,
            description: parent.description.clone(),
            due_date: Some(next_due),
            project: parent.project.clone(),
            recurrence: Some(recur),
            notes: parent.notes.clone(),
            agent: parent.agent.clone(),
            effort: parent.effort,
        });
        spawned = Some((new_id, next_due));
    }

    db::save(path, &task_file).map_err(|e| (1, e))?;
    match spawned {
        Some((new_id, next_due)) => Ok(format!(
            "Completed task {}. Next occurrence: task {}, due {}",
            id, new_id, next_due
        )),
        None => Ok(format!("Completed task {}", id)),
    }
}

pub fn reopen(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::load(path).map_err(|e| (1, e))?;
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    t.status = Status::Open;
    t.updated = Some(Utc::now());
    db::save(path, &task_file).map_err(|e| (1, e))?;
    Ok(format!("Reopened task {}", id))
}

pub fn rm(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::load(path).map_err(|e| (1, e))?;
    let removed = task_file.remove_task(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    db::save(path, &task_file).map_err(|e| (1, e))?;
    Ok(format!("Deleted task {}: {}", id, removed.title))
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib commands::`
Expected: PASS — all tests in `src/commands.rs` green.

- [ ] **Step 6: Register the module**

In `src/lib.rs`, add `pub mod commands;`:

```rust
pub mod auth;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod note;
pub mod parser;
pub mod task;

#[cfg(feature = "tui")]
pub mod claude_session;
#[cfg(feature = "tui")]
pub mod todoist;
#[cfg(feature = "tui")]
pub mod tui;
```

- [ ] **Step 7: Add the new subcommands to `src/cli.rs`**

In `src/cli.rs`, add these variants to the `Command` enum (alongside the existing `Add` variant):

```rust
    /// List tasks
    List {
        /// Filter by status: open or done
        #[arg(long)]
        status: Option<String>,

        /// Filter by assigned agent
        #[arg(long)]
        agent: Option<String>,

        /// Filter by project
        #[arg(long)]
        project: Option<String>,

        /// Filter by a single tag
        #[arg(long)]
        tag: Option<String>,

        /// Only show tasks due on or before this date (YYYY-MM-DD)
        #[arg(long)]
        due_before: Option<String>,
    },

    /// Show a task's full detail
    Show {
        /// Task id
        id: u32,
    },

    /// Edit a task
    Edit {
        /// Task id
        id: u32,

        #[arg(long)]
        title: Option<String>,

        #[arg(short, long)]
        priority: Option<String>,

        #[arg(short, long)]
        due: Option<String>,

        #[arg(long)]
        project: Option<String>,

        #[arg(long)]
        tags: Option<String>,

        #[arg(long)]
        agent: Option<String>,

        #[arg(long)]
        description: Option<String>,

        #[arg(long)]
        effort: Option<String>,
    },

    /// Mark a task done
    Done {
        /// Task id
        id: u32,
    },

    /// Reopen a completed task
    Reopen {
        /// Task id
        id: u32,
    },

    /// Delete a task
    Rm {
        /// Task id
        id: u32,
    },
```

- [ ] **Step 8: Wire up dispatch in `src/bin/task.rs`**

Replace the existing `Some(Command::Add { title, priority, due, project, tags, agent, description }) => { ... }` block with:

```rust
        Some(Command::Add { title, priority, due, project, tags, agent, description }) => {
            let msg = task::commands::add(&path, task::commands::AddArgs {
                title, priority, due, project, tags, agent, description,
            })?;
            println!("{}", msg);
            Ok(())
        }

        Some(Command::List { status, agent, project, tag, due_before }) => {
            let msg = task::commands::list(&path, task::commands::ListArgs {
                status, agent, project, tag, due_before,
            })?;
            println!("{}", msg);
            Ok(())
        }

        Some(Command::Show { id }) => {
            println!("{}", task::commands::show(&path, id)?);
            Ok(())
        }

        Some(Command::Edit { id, title, priority, due, project, tags, agent, description, effort }) => {
            let msg = task::commands::edit(&path, id, task::commands::EditArgs {
                title, priority, due, project, tags, agent, description, effort,
            })?;
            println!("{}", msg);
            Ok(())
        }

        Some(Command::Done { id }) => {
            println!("{}", task::commands::done(&path, id)?);
            Ok(())
        }

        Some(Command::Reopen { id }) => {
            println!("{}", task::commands::reopen(&path, id)?);
            Ok(())
        }

        Some(Command::Rm { id }) => {
            println!("{}", task::commands::rm(&path, id)?);
            Ok(())
        }
```

Note the removed manual `use std::str::FromStr;`-based priority parsing and manual `Task` construction — `commands::add` handles that now. Check whether `use std::str::FromStr;` at the top of `src/bin/task.rs` is still needed for anything else in the file; if not, remove it.

- [ ] **Step 9: Wire up the identical dispatch in `src/bin/task_tui.rs`**

Apply the same replacement as Step 8 to `src/bin/task_tui.rs`'s `run()` function (it has its own copy of the `Add` match arm at the same relative position).

- [ ] **Step 10: Run the full test suite**

Run: `cargo build --features tui && cargo test --features tui`
Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "feat: add list/show/edit/done/reopen/rm task CLI commands"
```

---

### Task 8: CLI integration tests for the new subcommands

**Files:**
- Modify: `tests/integration.rs`

**Interfaces:**
- Consumes: `task_bin()`, `temp_dir()`, `stdout()`, `stderr()` (existing helpers already in `tests/integration.rs`).

- [ ] **Step 1: Write the tests**

Add to `tests/integration.rs`:

```rust
// -- Task CRUD CLI tests --

fn task_db_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("tasks.db")
}

#[test]
fn test_add_and_list() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "add", "Buy milk", "--priority", "high"])
        .output()
        .unwrap();
    assert!(out.status.success(), "add failed: {}", stderr(&out));
    assert!(stdout(&out).contains("Created task 1: Buy milk"));

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "list"])
        .output()
        .unwrap();
    assert!(out.status.success(), "list failed: {}", stderr(&out));
    assert!(stdout(&out).contains("Buy milk"));
}

#[test]
fn test_list_no_tasks() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(stdout(&out).contains("No matching tasks."));
}

#[test]
fn test_show_task() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());
    task_bin().args(["--file", db_path.to_str().unwrap(), "add", "Read a book"]).output().unwrap();

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "show", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(stdout(&out).contains("Read a book"));
}

#[test]
fn test_edit_task() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());
    task_bin().args(["--file", db_path.to_str().unwrap(), "add", "Original title"]).output().unwrap();

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "edit", "1", "--title", "New title"])
        .output()
        .unwrap();
    assert!(out.status.success(), "edit failed: {}", stderr(&out));
    assert!(stdout(&out).contains("Updated task 1: New title"));
}

#[test]
fn test_done_and_reopen_task() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());
    task_bin().args(["--file", db_path.to_str().unwrap(), "add", "Finish this"]).output().unwrap();

    let out = task_bin().args(["--file", db_path.to_str().unwrap(), "done", "1"]).output().unwrap();
    assert!(out.status.success());
    assert!(stdout(&out).contains("Completed task 1"));

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "list", "--status", "open"])
        .output()
        .unwrap();
    assert!(!stdout(&out).contains("Finish this"));

    let out = task_bin().args(["--file", db_path.to_str().unwrap(), "reopen", "1"]).output().unwrap();
    assert!(out.status.success());
    assert!(stdout(&out).contains("Reopened task 1"));
}

#[test]
fn test_rm_task() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());
    task_bin().args(["--file", db_path.to_str().unwrap(), "add", "Delete me"]).output().unwrap();

    let out = task_bin().args(["--file", db_path.to_str().unwrap(), "rm", "1"]).output().unwrap();
    assert!(out.status.success());
    assert!(stdout(&out).contains("Deleted task 1: Delete me"));

    let out = task_bin().args(["--file", db_path.to_str().unwrap(), "list"]).output().unwrap();
    assert!(stdout(&out).contains("No matching tasks."));
}

#[test]
fn test_show_missing_task_fails() {
    let dir = temp_dir();
    let db_path = task_db_path(dir.path());

    let out = task_bin().args(["--file", db_path.to_str().unwrap(), "show", "42"]).output().unwrap();
    assert!(!out.status.success());
    assert!(stderr(&out).contains("not found"));
}

#[test]
fn test_migration_from_existing_markdown() {
    let dir = temp_dir();
    let md_path = dir.path().join("tasks.md");
    let db_path = task_db_path(dir.path());
    fs::write(&md_path, "<!-- format:2 -->\n<!-- next-id:2 -->\n\n# Tasks\n\n## [ ] Legacy task\n<!-- id:1 priority:medium created:2025-01-01T00:00:00+00:00 -->\n").unwrap();

    let out = task_bin()
        .args(["--file", db_path.to_str().unwrap(), "list"])
        .output()
        .unwrap();
    assert!(out.status.success(), "list failed: {}", stderr(&out));
    assert!(stdout(&out).contains("Legacy task"));
    assert!(md_path.exists(), "tasks.md should be left in place after migration");
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --test integration --features tui`
Expected: PASS (these exercise the code from Task 7, already implemented — this task adds coverage at the process/CLI boundary, so it's expected to be GREEN immediately; if anything fails, it indicates a wiring bug in Task 7's `bin/task.rs` dispatch to fix before proceeding).

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add CLI integration coverage for list/show/edit/done/reopen/rm"
```

---

### Task 9: Update `AGENTS.md` for the CLI workflow

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Rewrite the file-format sections as CLI usage**

Replace the sections `## Step 1: Always Read First`, `## File Format` (and all its subsections: Header, Section heading, Task block, Metadata fields, Rules for Each Operation, Rules to Never Break) with:

```markdown
## Working with Tasks

Tasks are stored in a SQLite database (`tasks.db` in your task directory), not a text file. Use the `task` CLI for every operation — do not attempt to open or edit `tasks.db` directly.

### Finding your tasks

```bash
task --file <task-dir>/tasks.db list --agent <your-agent-name> --status open
```

(Or, if you're running from inside your agent's working directory and it's already configured as `default-dir` for your profile, you can omit `--file`.)

### Adding a task

```bash
task add "Task title" --priority high --due 2026-03-15 --project Work --tags frontend,auth --agent command-center --description "Optional longer description"
```

Only `title` is required; `--priority` defaults to `medium`. The id is assigned automatically — it will be printed in the output (`Created task 12: Task title`).

### Editing a task

```bash
task edit <id> --priority critical --due 2026-04-01
```

Only the fields you pass are changed. `updated` is set automatically.

### Completing a task

```bash
task done <id>
```

If the task has a recurrence set, completing it automatically creates the next occurrence as a new open task and reports its id.

### Reopening a task

```bash
task reopen <id>
```

### Deleting a task

```bash
task rm <id>
```

### Viewing full task detail

```bash
task show <id>
```

### Rules to Never Break

- **Never guess or hand-construct a task id** — always use the id the CLI reports back to you (from `add`, `list`, or `show`).
- **Never edit `tasks.db` with a text editor or by hand** — it's a SQLite database, not a text file; use the CLI for every read and write.
```

- [ ] **Step 2: Confirm the unaffected sections are still present and untouched**

Re-read the file and confirm these sections are unchanged: `# Tasks File — AI Instructions` intro, `## Finding Your Tasks` (the config-lookup one, near the top — distinct from the new "Finding your tasks" CLI subsection above; consider renaming the new one to avoid a duplicate heading, e.g. keep the original as `## Finding Your Agent Profile` if it's ambiguous), `## Reading Your Instructions`, `## Reading and Updating Memory`, `## TUI Auto-Filter`.

If there's a heading collision between the existing "Finding Your Tasks" (which explains resolving your agent name from config) and the new "Finding your tasks" CLI subsection added in Step 1, rename the original section to `## Finding Your Agent Profile` so both remain distinct and readable.

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs: rewrite AGENTS.md for SQLite-backed CLI task workflow"
```

---

### Task 10: Show build version in the TUI header

**Files:**
- Modify: `src/tui.rs`

**Interfaces:**
- Consumes: `env!("CARGO_PKG_VERSION")` (compile-time constant, no new dependency).

- [ ] **Step 1: Write the failing test**

`draw_header` is a rendering function that writes into a `ratatui::Frame`; the simplest testable unit is extracting the title string it builds. Add a small helper and test it directly. In `src/tui.rs`, near `draw_header`, add:

```rust
#[cfg(test)]
mod header_tests {
    use super::*;

    #[test]
    fn test_header_title_includes_version() {
        let title = header_title("Due [Overdue]", None);
        assert!(title.contains(env!("CARGO_PKG_VERSION")));
        assert!(title.starts_with(" task-manager v"));
    }

    #[test]
    fn test_header_title_includes_filter_when_active() {
        let title = header_title("Recurring", Some("agent:bot"));
        assert!(title.contains("filter: agent:bot"));
        assert!(title.contains(env!("CARGO_PKG_VERSION")));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --features tui --lib header_tests`
Expected: FAIL — `header_title` doesn't exist yet.

- [ ] **Step 3: Extract and implement `header_title`, update `draw_header` to use it**

Replace the body of `draw_header` (currently building `title` inline) with a call to a new pure function:

```rust
fn header_title(view_label: &str, active_filter_summary: Option<&str>) -> String {
    match active_filter_summary {
        Some(summary) => format!(
            " task-manager v{}  |  {}  |  filter: {} ",
            env!("CARGO_PKG_VERSION"),
            view_label,
            summary
        ),
        None => format!(" task-manager v{}  |  {} ", env!("CARGO_PKG_VERSION"), view_label),
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let view_label = if app.view == View::Due {
        format!("Due [{}]", app.due_window.label())
    } else {
        app.view.display_name().to_string()
    };
    let title = header_title(
        &view_label,
        if app.filter.is_active() { Some(&app.filter.summary()) } else { None },
    );
    let header = Paragraph::new(title).style(
        Style::default()
            .fg(theme::BAR_FG)
            .bg(theme::BAR_BG)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, area);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --features tui --lib header_tests`
Expected: PASS.

- [ ] **Step 5: Manually verify in the running TUI**

Run: `cargo run --features tui --bin task-tui -- --file /tmp/manual-test-tasks.db`
Expected: the header bar at the top reads ` task-manager v3.4.0  |  <view> ` (version matching `Cargo.toml`'s `version` field). Press `q` to quit.

- [ ] **Step 6: Commit**

```bash
git add src/tui.rs
git commit -m "feat: show crate version in TUI header"
```

---

### Task 11: Final full-suite verification

**Files:** none (verification only)

- [ ] **Step 1: Run the complete test suite with all features**

Run: `cargo test --features tui`
Expected: PASS, zero failures.

- [ ] **Step 2: Run clippy to catch anything the compiler alone wouldn't**

Run: `cargo clippy --features tui --all-targets -- -D warnings`
Expected: no warnings. Fix any that appear (most likely: unused imports left over from the `storage`→`db` rename, e.g. `use std::str::FromStr;` in `bin/task.rs`/`bin/task_tui.rs` if nothing else in those files uses it after Task 7).

- [ ] **Step 3: Confirm `fs2` is fully gone**

Run: `grep -rn "fs2" Cargo.toml Cargo.lock src/`
Expected: no output in `Cargo.toml`/`src/` (an entry may briefly remain in `Cargo.lock` until the next `cargo build`, which Step 1 already ran, so `Cargo.lock` should also be clean).

- [ ] **Step 4: Manual end-to-end smoke test of the migration path**

```bash
mkdir -p /tmp/sqlite-migration-smoke
cat > /tmp/sqlite-migration-smoke/tasks.md <<'EOF'
<!-- format:2 -->
<!-- next-id:2 -->

# Tasks

## [ ] Smoke test task
<!-- id:1 priority:high created:2025-01-01T00:00:00+00:00 -->
EOF
cargo run --bin task -- --file /tmp/sqlite-migration-smoke/tasks.db list
```

Expected: prints the "Smoke test task" row, `/tmp/sqlite-migration-smoke/tasks.db` now exists, and `/tmp/sqlite-migration-smoke/tasks.md` is still present and unchanged. Clean up: `rm -rf /tmp/sqlite-migration-smoke`.

- [ ] **Step 5: Bump the crate version and commit**

This change is user-facing (new commands, new storage format) — bump the patch/minor version in `Cargo.toml` per the project's existing versioning convention (see recent commits like "chore: bump to v3.4.0"). Given the scope (new feature set), bump the minor version:

```toml
version = "3.5.0"
```

```bash
cargo build --features tui  # regenerate Cargo.lock with the new version
git add Cargo.toml Cargo.lock
git commit -m "chore: bump to v3.5.0"
```

---

## Self-Review Notes

- **Spec coverage:** schema/migration (Tasks 2–3), backups (Task 4), storage cutover (Task 5), `parser.rs` trim (Task 6), new CLI commands (Tasks 7–8), `AGENTS.md` rewrite (Task 9), version display (Task 10), dependency swap (Task 1), final verification (Task 11). All spec sections are covered.
- **Placeholder scan:** no TBD/TODO markers; every step has runnable code or an exact command.
- **Type consistency:** `db::load(path: &Path) -> Result<TaskFile, String>` and `db::save(path: &Path, task_file: &TaskFile) -> Result<(), String>` signatures introduced in Task 2 are used identically in Tasks 3, 4, 5, 7. `commands::{AddArgs, ListArgs, EditArgs}` field names in Task 7 match the `cli.rs` subcommand field names used in the same task's dispatch code, and match the CLI flag names documented in Task 9's `AGENTS.md` rewrite.
