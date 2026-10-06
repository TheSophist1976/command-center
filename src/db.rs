use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use chrono::{DateTime, NaiveDate, Utc};
use std::str::FromStr;
use std::fs;

use crate::task::{Effort, Priority, Recurrence, Status, Task, TaskFile, WorkStatus};

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

/// Resolves the directory Notes are stored under.
/// Uses the `notes-dir` config key if set, otherwise falls back to
/// `Notes/` alongside the resolved tasks.db path.
pub fn resolve_notes_dir(file_path: &Path) -> PathBuf {
    resolve_notes_dir_inner(file_path, crate::config::config_path().as_deref())
}

fn resolve_notes_dir_inner(file_path: &Path, config_path: Option<&Path>) -> PathBuf {
    if let Some(cfg_path) = config_path {
        if let Some(notes_dir) = crate::config::read_config_value_from(cfg_path, "notes-dir") {
            if !notes_dir.is_empty() {
                return crate::config::expand_tilde(&notes_dir);
            }
        }
    }
    file_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("Notes")
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
    // ALTER TABLE has no "ADD COLUMN IF NOT EXISTS" — this fails with a
    // "duplicate column" error on every database that already has the
    // column, which is the expected, common case. Any other failure here
    // (locked/corrupt file) will surface immediately on the next query
    // against `tasks` anyway, so discarding the error is safe.
    conn.execute("ALTER TABLE tasks ADD COLUMN work_status TEXT", [])
        .ok();
    conn.execute("ALTER TABLE tasks ADD COLUMN instructions TEXT", [])
        .ok();
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
    let work_status_s: Option<String> = row.get(14)?;
    let instructions: Option<String> = row.get(15)?;

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
        instructions,
        due_date: due_s.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        project,
        recurrence: recur_s.and_then(|s| Recurrence::from_str(&s).ok()),
        notes: text_to_list(&notes_s),
        agent,
        effort: effort_s.and_then(|s| Effort::from_str(&s).ok()),
        work_status: work_status_s.and_then(|s| WorkStatus::from_str(&s).ok()),
    })
}

fn insert_task_row(conn: &Connection, t: &Task) -> Result<(), String> {
    conn.execute(
        "INSERT INTO tasks (id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort, work_status, instructions)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
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
            t.instructions,
        ],
    )
    .map_err(|e| format!("Failed to insert task {}: {}", t.id, e))?;
    Ok(())
}

fn migrate_from_markdown(db_path: &Path, conn: &mut Connection) -> Result<(), String> {
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
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start migration transaction: {}", e))?;
    for t in &task_file.tasks {
        insert_task_row(&tx, t)?;
    }
    tx.commit()
        .map_err(|e| format!("Failed to commit migration transaction: {}", e))?;
    Ok(())
}

fn load_from_conn(conn: &Connection) -> Result<TaskFile, String> {
    let mut stmt = conn
        .prepare("SELECT id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort, work_status, instructions FROM tasks ORDER BY id")
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

fn open_migrated(path: &Path) -> Result<Connection, String> {
    let needs_migration = !path.exists();
    let mut conn = open_conn(path)?;
    if needs_migration {
        migrate_from_markdown(path, &mut conn)?;
    }
    Ok(conn)
}

/// Read-only snapshot. Never write this back with `save`; use `begin` for any
/// read-modify-write so concurrent writers cannot lose each other's changes.
pub fn load(path: &Path) -> Result<TaskFile, String> {
    load_from_conn(&open_migrated(path)?)
}

/// Tasks plus the data version they were read at, from one consistent snapshot.
pub fn load_versioned(path: &Path) -> Result<(TaskFile, i64), String> {
    let conn = open_migrated(path)?;
    conn.execute_batch("BEGIN")
        .map_err(|e| format!("Failed to start read transaction: {}", e))?;
    let result = load_from_conn(&conn).and_then(|tf| Ok((tf, read_version(&conn)?)));
    let _ = conn.execute_batch("ROLLBACK");
    result
}

fn read_version(conn: &Connection) -> Result<i64, String> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| format!("Failed to read data version: {}", e))
}

/// Change counter bumped by every committed write. Unlike the file's mtime it is
/// reliable in WAL mode (where the main file is not touched on each commit).
pub fn version(path: &Path) -> Result<i64, String> {
    read_version(&open_migrated(path)?)
}

fn write_all(conn: &Connection, task_file: &TaskFile, next_version: i64) -> Result<(), String> {
    conn.execute("DELETE FROM tasks", [])
        .map_err(|e| format!("Failed to clear tasks: {}", e))?;
    for t in &task_file.tasks {
        insert_task_row(conn, t)?;
    }
    conn.execute_batch(&format!("PRAGMA user_version = {}", next_version))
        .map_err(|e| format!("Failed to bump data version: {}", e))
}

/// A read-modify-write transaction. `begin` takes SQLite's write lock *before*
/// reading, so no other process can commit between our read and our `commit`.
/// Dropping without `commit` rolls everything back.
pub struct TaskTxn {
    conn: Connection,
    task_file: TaskFile,
    version: i64,
    finished: bool,
}

pub fn begin(path: &Path) -> Result<TaskTxn, String> {
    let conn = open_migrated(path)?;
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("Failed to lock {}: {}", path.display(), e))?;
    let mut txn = TaskTxn { conn, task_file: TaskFile::new(), version: 0, finished: false };
    txn.task_file = load_from_conn(&txn.conn)?;
    txn.version = read_version(&txn.conn)?;
    Ok(txn)
}

impl TaskTxn {
    /// The data version this transaction read, as returned by `db::version`.
    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn commit(mut self) -> Result<(), String> {
        write_all(&self.conn, &self.task_file, self.version + 1)?;
        self.conn
            .execute_batch("COMMIT")
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for TaskTxn {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.conn.execute_batch("ROLLBACK");
        }
    }
}

impl std::ops::Deref for TaskTxn {
    type Target = TaskFile;
    fn deref(&self) -> &TaskFile {
        &self.task_file
    }
}

impl std::ops::DerefMut for TaskTxn {
    fn deref_mut(&mut self) -> &mut TaskFile {
        &mut self.task_file
    }
}

/// Unconditionally replaces every task. Last writer wins, so only use it to seed
/// a database; read-modify-write code must go through `begin`.
pub fn save(path: &Path, task_file: &TaskFile) -> Result<(), String> {
    let mut conn = open_conn(path)?;
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    let next_version = read_version(&tx)? + 1;
    write_all(&tx, task_file, next_version)?;
    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::env;

    #[test]
    fn concurrent_begin_commit_loses_no_updates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut seed = TaskFile::new();
        seed.tasks = (1..=4).map(|i| sample_task(i, &format!("task {}", i))).collect();
        save(&path, &seed).unwrap();

        // Each thread edits a different task and adds one. With a plain load/save
        // pair, overlapping threads overwrite each other's rows.
        let handles: Vec<_> = (1..=4u32)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for round in 0..5 {
                        let mut txn = begin(&path).unwrap();
                        txn.find_task_mut(i).unwrap().title = format!("task {} round {}", i, round);
                        let id = txn.next_id;
                        txn.next_id += 1;
                        txn.tasks.push(sample_task(id, &format!("added by {} round {}", i, round)));
                        txn.commit().unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let tf = load(&path).unwrap();
        assert_eq!(tf.tasks.len(), 4 + 4 * 5, "an added task was lost");
        let mut ids: Vec<u32> = tf.tasks.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), tf.tasks.len(), "duplicate ids were handed out");
        for i in 1..=4u32 {
            assert_eq!(tf.find_task(i).unwrap().title, format!("task {} round 4", i), "edit to task {} was lost", i);
        }
    }

    #[test]
    fn dropping_a_transaction_without_commit_changes_nothing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let mut seed = TaskFile::new();
        seed.tasks = vec![sample_task(1, "keep")];
        save(&path, &seed).unwrap();
        let before = version(&path).unwrap();

        {
            let mut txn = begin(&path).unwrap();
            txn.tasks.clear();
        }

        assert_eq!(load(&path).unwrap().tasks.len(), 1);
        assert_eq!(version(&path).unwrap(), before);
    }

    #[test]
    fn every_commit_bumps_the_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        save(&path, &TaskFile::new()).unwrap();
        let v0 = version(&path).unwrap();
        begin(&path).unwrap().commit().unwrap();
        assert_eq!(version(&path).unwrap(), v0 + 1);
        save(&path, &TaskFile::new()).unwrap();
        assert_eq!(version(&path).unwrap(), v0 + 2);
        assert_eq!(load_versioned(&path).unwrap().1, v0 + 2);
    }

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
            instructions: None,
            due_date: None,
            project: None,
            recurrence: None,
            notes: vec![],
            agent: None,
            effort: None,
            work_status: None,
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
    fn test_resolve_notes_dir_default_fallback() {
        let p = resolve_notes_dir_inner(&PathBuf::from("/my/tasks/tasks.db"), None);
        assert_eq!(p, PathBuf::from("/my/tasks/Notes"));
    }

    #[test]
    fn test_resolve_notes_dir_config_override() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "default-dir: /my/tasks\nnotes-dir: /my/notes\n").unwrap();
        let p = resolve_notes_dir_inner(&PathBuf::from("/my/tasks/tasks.db"), Some(&config_path));
        assert_eq!(p, PathBuf::from("/my/notes"));
    }

    #[test]
    fn test_resolve_notes_dir_config_without_notes_dir_key() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "default-dir: /my/tasks\n").unwrap();
        let p = resolve_notes_dir_inner(&PathBuf::from("/my/tasks/tasks.db"), Some(&config_path));
        assert_eq!(p, PathBuf::from("/my/tasks/Notes"));
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
            description: Some("desc".to_string()), instructions: None,
            due_date: NaiveDate::from_ymd_opt(2026, 12, 31),
            project: Some("My Project: v2".to_string()),
            recurrence: Some(Recurrence::from_str("weekly:fri").unwrap()),
            notes: vec!["note-a".to_string(), "note-b".to_string()],
            agent: Some("command-center".to_string()),
            effort: Some(Effort::Medium),
            work_status: Some(WorkStatus::InProgress),
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
        assert_eq!(t.work_status, Some(WorkStatus::InProgress));
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

    #[test]
    fn test_migration_transaction_atomicity_on_insert_failure() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let md_path = dir.path().join("tasks.md");
        // Create a markdown file with two tasks sharing the same ID (will cause constraint violation on second insert)
        let content = "<!-- format:2 -->\n<!-- next-id:3 -->\n\n# Tasks\n\n## [ ] Task 1\n<!-- id:1 priority:high created:2025-01-01T00:00:00+00:00 -->\n\n## [ ] Task 2\n<!-- id:1 priority:medium created:2025-01-02T00:00:00+00:00 -->\n";
        std::fs::write(&md_path, content).unwrap();

        // Migration should fail due to duplicate ID
        let result = load(&db_path);
        assert!(result.is_err(), "Expected migration to fail due to duplicate task ID");

        // The database file exists (created by open_conn), but should be empty (transaction rolled back)
        assert!(db_path.exists());
        let tf = load(&db_path).unwrap();
        assert!(tf.tasks.is_empty(), "Database should be empty after failed migration");
    }

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
        // instructions column was added by the same migration path.
        assert_eq!(reloaded.tasks[0].instructions, None);
        let mut reloaded = reloaded;
        reloaded.tasks[0].instructions = Some("do it".to_string());
        save(&path, &reloaded).unwrap();
        assert_eq!(load(&path).unwrap().tasks[0].instructions.as_deref(), Some("do it"));
    }
}
