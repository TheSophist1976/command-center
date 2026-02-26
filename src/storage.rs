use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::parser;
use crate::task::TaskFile;

pub fn resolve_file_path(flag: Option<&str>) -> PathBuf {
    resolve_file_path_inner(flag, crate::config::config_path().as_deref())
}

fn resolve_file_path_inner(flag: Option<&str>, config_path: Option<&std::path::Path>) -> PathBuf {
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
                return crate::config::expand_tilde(&default_dir).join("tasks.md");
            }
        }
    }
    PathBuf::from("tasks.md")
}

pub fn load(path: &Path, strict: bool) -> Result<TaskFile, String> {
    if !path.exists() {
        return Ok(TaskFile::new());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(TaskFile::new());
    }
    parser::parse(&content, strict).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        format!("Parse errors:\n{}", msgs.join("\n"))
    })
}

pub fn save(path: &Path, task_file: &TaskFile) -> Result<(), String> {
    let content = parser::serialize(task_file);
    let dir = path.parent().unwrap_or(Path::new("."));

    // Write to temp file first
    let tmp_path = dir.join(format!(".task-tmp-{}", std::process::id()));

    // Acquire advisory lock on the target file before writing
    let _lock_file = if path.exists() {
        let f = fs::File::open(path)
            .map_err(|e| format!("Failed to open {} for locking: {}", path.display(), e))?;
        f.lock_exclusive()
            .map_err(|e| format!("Failed to acquire lock on {}: {}", path.display(), e))?;
        Some(f)
    } else {
        None
    };

    let file = fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut writer = io::BufWriter::new(file);
    writer.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    writer.flush()
        .map_err(|e| format!("Failed to flush temp file: {}", e))?;
    drop(writer);

    // Atomic rename (lock is still held via _lock_file)
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;

    // _lock_file drops here, releasing the lock
    Ok(())
}

pub fn init_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Err(format!("{} already exists. Will not overwrite.", path.display()));
    }
    let task_file = TaskFile::new();
    save(path, &task_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    // -- resolve_file_path --

    #[test]
    fn test_resolve_file_path_with_flag() {
        let p = resolve_file_path(Some("/tmp/my-tasks.md"));
        assert_eq!(p, PathBuf::from("/tmp/my-tasks.md"));
    }

    #[test]
    fn test_resolve_file_path_default() {
        // Remove env var if present; pass None as config path so we get the hardcoded fallback
        unsafe { env::remove_var("TASK_FILE") };
        let p = resolve_file_path_inner(None, None);
        assert_eq!(p, PathBuf::from("tasks.md"));
    }

    #[test]
    fn test_resolve_file_path_config_default_dir() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.md");
        fs::write(&config_path, "default-dir: /my/notes\n").unwrap();
        unsafe { env::remove_var("TASK_FILE") };
        let p = resolve_file_path_inner(None, Some(&config_path));
        assert_eq!(p, PathBuf::from("/my/notes/tasks.md"));
    }

    #[test]
    fn test_resolve_file_path_env_overrides_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.md");
        fs::write(&config_path, "default-dir: /my/notes\n").unwrap();
        unsafe { env::set_var("TASK_FILE", "/tmp/env-tasks.md") };
        let p = resolve_file_path_inner(None, Some(&config_path));
        assert_eq!(p, PathBuf::from("/tmp/env-tasks.md"));
        unsafe { env::remove_var("TASK_FILE") };
    }

    #[test]
    fn test_resolve_file_path_env_var() {
        unsafe { env::set_var("TASK_FILE", "/tmp/env-tasks.md") };
        let p = resolve_file_path(None);
        assert_eq!(p, PathBuf::from("/tmp/env-tasks.md"));
        unsafe { env::remove_var("TASK_FILE") };
    }

    #[test]
    fn test_resolve_file_path_empty_env_var() {
        unsafe { env::set_var("TASK_FILE", "") };
        // Pass None as config path so we get the hardcoded fallback regardless of real config
        let p = resolve_file_path_inner(None, None);
        // Empty env var falls back to default
        assert_eq!(p, PathBuf::from("tasks.md"));
        unsafe { env::remove_var("TASK_FILE") };
    }

    #[test]
    fn test_resolve_file_path_flag_overrides_env() {
        unsafe { env::set_var("TASK_FILE", "/tmp/env-tasks.md") };
        let p = resolve_file_path(Some("/tmp/flag-tasks.md"));
        assert_eq!(p, PathBuf::from("/tmp/flag-tasks.md"));
        unsafe { env::remove_var("TASK_FILE") };
    }

    // -- load --

    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.md");
        let tf = load(&path, false).unwrap();
        assert!(tf.tasks.is_empty());
        assert_eq!(tf.next_id, 1);
    }

    #[test]
    fn test_load_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.md");
        fs::write(&path, "").unwrap();
        let tf = load(&path, false).unwrap();
        assert!(tf.tasks.is_empty());
    }

    #[test]
    fn test_load_whitespace_only_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ws.md");
        fs::write(&path, "   \n\n  ").unwrap();
        let tf = load(&path, false).unwrap();
        assert!(tf.tasks.is_empty());
    }

    #[test]
    fn test_load_valid_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let content = "<!-- format:2 -->\n<!-- next-id:3 -->\n\n# Tasks\n\n## [ ] Task A\n<!-- id:1 priority:medium created:2025-01-01T00:00:00+00:00 -->\n\n## [x] Task B\n<!-- id:2 priority:high created:2025-01-02T00:00:00+00:00 -->\n";
        fs::write(&path, content).unwrap();
        let tf = load(&path, false).unwrap();
        assert_eq!(tf.tasks.len(), 2);
        assert_eq!(tf.next_id, 3);
    }

    #[test]
    fn test_load_strict_parse_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("strict.md");
        // A heading that doesn't match task format
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## Some random heading\n\n## [ ] Valid task\n<!-- id:1 priority:low created:2025-01-15T10:00:00+00:00 -->\n";
        fs::write(&path, content).unwrap();
        let result = load(&path, true);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Parse errors"));
    }

    // -- save --

    #[test]
    fn test_save_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("new.md");
        let tf = TaskFile::new();
        save(&path, &tf).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut tf = TaskFile::new();
        use chrono::Utc;
        use crate::task::{Priority, Status, Task};
        tf.tasks.push(Task {
            id: 1,
            title: "Test roundtrip".to_string(),
            status: Status::Open,
            priority: Priority::High,
            tags: vec!["alpha".to_string()],
            created: Utc::now(),
            updated: None,
            description: Some("Some description".to_string()),
            due_date: None,
            project: None,
        });
        tf.next_id = 2;
        save(&path, &tf).unwrap();

        let loaded = load(&path, false).unwrap();
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
        let path = dir.path().join("tasks.md");
        let mut tf = TaskFile::new();
        use chrono::Utc;
        use crate::task::{Priority, Status, Task};
        tf.tasks.push(Task {
            id: 1,
            title: "Original".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
        });
        save(&path, &tf).unwrap();

        // Now save different content
        let tf2 = TaskFile::new();
        save(&path, &tf2).unwrap();

        let loaded = load(&path, false).unwrap();
        assert_eq!(loaded.tasks.len(), 0);
    }

    // -- init_file --

    #[test]
    fn test_init_file_creates_new() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        assert!(!path.exists());
        init_file(&path).unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        // The serializer writes format:2 (latest format)
        assert!(content.contains("format:2"));
    }

    #[test]
    fn test_init_file_refuses_overwrite() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        fs::write(&path, "existing").unwrap();
        let result = init_file(&path);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("already exists"));
    }

    // -- Error path tests --

    #[test]
    #[cfg(unix)]
    fn test_save_to_readonly_directory() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        // Create a readonly directory
        let readonly_dir = dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let path = readonly_dir.join("tasks.md");
        let tf = TaskFile::new();
        let result = save(&path, &tf);
        // Should fail because we can't create temp file in readonly dir
        assert!(result.is_err());
        // Restore permissions for cleanup
        let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
    }

    #[test]
    fn test_load_read_error_simulation() {
        // Test loading a file that exists but might cause issues
        // We can test the error path by loading a valid file with strict mode that has parse errors
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.md");
        // Write content that will fail strict parsing
        let content = "<!-- format:99 -->\n## [ ] Task\n<!-- id:1 created:2025-01-15T10:00:00+00:00 -->\n";
        fs::write(&path, content).unwrap();
        let result = load(&path, true);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Parse errors"));
    }

    #[test]
    #[cfg(unix)]
    fn test_load_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("unreadable.md");
        fs::write(&path, "some content").unwrap();
        // Make file unreadable
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

        let result = load(&path, false);
        // Should fail because we can't read the file
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Failed to read"));

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
    }

    // -- save: covers line 47 (File::open for locking fails when file is unreadable) --
    #[test]
    #[cfg(unix)]
    fn test_save_file_open_for_lock_fails() {
        // When path.exists() is true but the file can't be opened for locking,
        // save should return an error (covers line 47: "Failed to open {} for locking").
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        // Create the file first
        fs::write(&path, "<!-- format:2 -->\n<!-- next-id:1 -->\n\n# Tasks\n").unwrap();
        // Make the file unreadable (0o000) so File::open fails
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

        use crate::task::TaskFile;
        let tf = TaskFile::new();
        let result = save(&path, &tf);

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));

        // Should fail because we can't open the file for locking
        // (or can't create temp file due to permission error)
        assert!(result.is_err(), "Expected save to fail on unreadable file");
    }
}
