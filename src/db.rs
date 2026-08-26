use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use chrono::{DateTime, NaiveDate, Utc};
use std::str::FromStr;
use std::fs;

use crate::task::{Effort, Priority, Recurrence, Status, Task, TaskFile};

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

pub fn load(path: &Path) -> Result<TaskFile, String> {
    let needs_migration = !path.exists();
    let mut conn = open_conn(path)?;
    if needs_migration {
        migrate_from_markdown(path, &mut conn)?;
    }

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
}
