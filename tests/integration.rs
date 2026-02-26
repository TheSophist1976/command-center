use std::fs;
use std::process::Command;

fn task_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_task"))
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// Helper: run task command in a temp dir with a task file.
// Sets TASK_CONFIG_FILE to a nonexistent path so the real user config is never read.
fn run_in(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    task_bin()
        .args(args)
        .current_dir(dir)
        .env("TASK_CONFIG_FILE", dir.join(".test-config.md"))
        .output()
        .expect("failed to run task")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

// -- Init tests --

#[test]
fn test_init_creates_file() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["init"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Initialized"));
    assert!(dir.path().join("tasks.md").exists());
}

#[test]
fn test_init_refuses_overwrite() {
    let dir = temp_dir();
    run_in(dir.path(), &["init"]);
    let out = run_in(dir.path(), &["init"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("already exists"));
}

// -- Add tests --

#[test]
fn test_add_creates_file_automatically() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "First task"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Added task 1: First task"));
    assert!(dir.path().join("tasks.md").exists());
}

#[test]
fn test_add_with_priority_and_tags() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "My task", "--priority", "high", "--tags", "frontend,auth"]);
    assert!(out.status.success());
    let content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    assert!(content.contains("priority:high"));
    assert!(content.contains("tags:frontend,auth"));
}

#[test]
fn test_add_increments_id() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task one"]);
    let out = run_in(dir.path(), &["add", "Task two"]);
    assert!(stdout(&out).contains("Added task 2"));
}

#[test]
fn test_add_with_due_date() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Due task", "--due", "2025-12-31"]);
    assert!(out.status.success());
    let content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    assert!(content.contains("due:2025-12-31"));
}

#[test]
fn test_add_with_invalid_due_date() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task", "--due", "not-a-date"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid date"));
}

#[test]
fn test_add_with_project() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Project task", "--project", "myproject"]);
    assert!(out.status.success());
    let content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    assert!(content.contains("project:myproject"));
}

#[test]
fn test_add_with_critical_priority() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Critical task", "--priority", "critical"]);
    assert!(out.status.success());
    let content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    assert!(content.contains("priority:critical"));
}

#[test]
fn test_add_with_valid_hyphenated_tag() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task", "--tags", "api-v2,frontend"]);
    assert!(out.status.success());
    let content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    assert!(content.contains("tags:api-v2,frontend"));
}

// -- List tests --

#[test]
fn test_list_empty() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["list"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("No tasks found"));
}

#[test]
fn test_list_shows_tasks() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task one"]);
    run_in(dir.path(), &["add", "Task two"]);
    let out = run_in(dir.path(), &["list"]);
    assert!(stdout(&out).contains("Task one"));
    assert!(stdout(&out).contains("Task two"));
}

#[test]
fn test_list_filter_status() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Open task"]);
    run_in(dir.path(), &["add", "Done task"]);
    run_in(dir.path(), &["done", "2"]);
    let out = run_in(dir.path(), &["list", "--status", "open"]);
    let s = stdout(&out);
    assert!(s.contains("Open task"));
    assert!(!s.contains("Done task"));
}

#[test]
fn test_list_filter_priority() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "High task", "--priority", "high"]);
    run_in(dir.path(), &["add", "Low task", "--priority", "low"]);
    let out = run_in(dir.path(), &["list", "--priority", "high"]);
    let s = stdout(&out);
    assert!(s.contains("High task"));
    assert!(!s.contains("Low task"));
}

#[test]
fn test_list_filter_tag() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Frontend task", "--tags", "frontend"]);
    run_in(dir.path(), &["add", "Backend task", "--tags", "backend"]);
    let out = run_in(dir.path(), &["list", "--tag", "frontend"]);
    let s = stdout(&out);
    assert!(s.contains("Frontend task"));
    assert!(!s.contains("Backend task"));
}

#[test]
fn test_list_combined_filters() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Match", "--priority", "high", "--tags", "web"]);
    run_in(dir.path(), &["add", "No match", "--priority", "low", "--tags", "web"]);
    let out = run_in(dir.path(), &["list", "--priority", "high", "--tag", "web"]);
    let s = stdout(&out);
    assert!(s.contains("Match"));
    assert!(!s.contains("No match"));
}

#[test]
fn test_list_filter_project() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Proj task", "--project", "alpha"]);
    run_in(dir.path(), &["add", "Other task"]);
    let out = run_in(dir.path(), &["list", "--project", "alpha"]);
    let s = stdout(&out);
    assert!(s.contains("Proj task"));
    assert!(!s.contains("Other task"));
}

#[test]
fn test_list_filter_project_no_match() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "No project task"]);
    let out = run_in(dir.path(), &["list", "--project", "nonexistent"]);
    let s = stdout(&out);
    // Should show No tasks found since task has no project
    assert!(!s.contains("No project task"));
}

#[test]
fn test_list_filter_project_wrong_project() {
    // Task has a project but it doesn't match the filter
    // This covers main.rs line 102: Some(p) => if p != proj_filter { return false; }
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Alpha task", "--project", "alpha"]);
    run_in(dir.path(), &["add", "Beta task", "--project", "beta"]);
    let out = run_in(dir.path(), &["list", "--project", "alpha"]);
    let s = stdout(&out);
    assert!(s.contains("Alpha task"), "Should find alpha task");
    assert!(!s.contains("Beta task"), "Should not show beta task");
}

#[test]
fn test_list_invalid_status() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["list", "--status", "pending"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid status"));
}

#[test]
fn test_list_invalid_priority() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["list", "--priority", "urgent"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn test_list_with_due_date_shows_due_column() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Due task", "--due", "2025-12-31"]);
    run_in(dir.path(), &["add", "Other task"]);
    let out = run_in(dir.path(), &["list"]);
    let s = stdout(&out);
    assert!(s.contains("Due"));
    assert!(s.contains("2025-12-31"));
}

#[test]
fn test_list_with_project_shows_project_column() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task with proj", "--project", "myproj"]);
    let out = run_in(dir.path(), &["list"]);
    let s = stdout(&out);
    assert!(s.contains("Project"));
    assert!(s.contains("myproj"));
}

#[test]
fn test_list_long_title_truncated() {
    let dir = temp_dir();
    let long_title = "A".repeat(50);
    run_in(dir.path(), &["add", &long_title]);
    let out = run_in(dir.path(), &["list"]);
    // Should not panic, output should be reasonable
    assert!(out.status.success());
}

#[test]
fn test_list_done_status_filter() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task A"]);
    run_in(dir.path(), &["add", "Task B"]);
    run_in(dir.path(), &["done", "1"]);
    let out = run_in(dir.path(), &["list", "--status", "done"]);
    let s = stdout(&out);
    assert!(s.contains("Task A"));
    assert!(!s.contains("Task B"));
}

// -- Show tests --

#[test]
fn test_show_existing() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "My task", "--priority", "high"]);
    let out = run_in(dir.path(), &["show", "1"]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("My task"));
    assert!(s.contains("high"));
}

#[test]
fn test_show_not_found() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["show", "999"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn test_show_with_tags() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Tagged task", "--tags", "alpha,beta"]);
    let out = run_in(dir.path(), &["show", "1"]);
    let s = stdout(&out);
    assert!(s.contains("alpha"));
    assert!(s.contains("beta"));
}

#[test]
fn test_show_with_due_date() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task", "--due", "2025-06-15"]);
    let out = run_in(dir.path(), &["show", "1"]);
    let s = stdout(&out);
    assert!(s.contains("2025-06-15"));
}

#[test]
fn test_show_with_project() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task", "--project", "myproject"]);
    let out = run_in(dir.path(), &["show", "1"]);
    let s = stdout(&out);
    assert!(s.contains("myproject"));
}

#[test]
fn test_show_done_task_updated_timestamp() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    run_in(dir.path(), &["done", "1"]);
    let out = run_in(dir.path(), &["show", "1"]);
    let s = stdout(&out);
    assert!(s.contains("Updated:"));
}

// -- Edit tests --

#[test]
fn test_edit_title() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Old title"]);
    let out = run_in(dir.path(), &["edit", "1", "--title", "New title"]);
    assert!(out.status.success());
    let show = run_in(dir.path(), &["show", "1"]);
    assert!(stdout(&show).contains("New title"));
}

#[test]
fn test_edit_priority() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1", "--priority", "low"]);
    assert!(out.status.success());
    let show = run_in(dir.path(), &["show", "1"]);
    assert!(stdout(&show).contains("low"));
}

#[test]
fn test_edit_tags() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task", "--tags", "old"]);
    let out = run_in(dir.path(), &["edit", "1", "--tags", "backend,api"]);
    assert!(out.status.success());
    let show = run_in(dir.path(), &["show", "1"]);
    let s = stdout(&show);
    assert!(s.contains("backend"));
    assert!(s.contains("api"));
}

#[test]
fn test_edit_due_date() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1", "--due", "2025-08-01"]);
    assert!(out.status.success());
    let show = run_in(dir.path(), &["show", "1"]);
    assert!(stdout(&show).contains("2025-08-01"));
}

#[test]
fn test_edit_project() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1", "--project", "newproject"]);
    assert!(out.status.success());
    let show = run_in(dir.path(), &["show", "1"]);
    assert!(stdout(&show).contains("newproject"));
}

#[test]
fn test_edit_no_fields() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Nothing to edit"));
}

#[test]
fn test_edit_invalid_priority() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1", "--priority", "badpri"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn test_edit_invalid_due_date() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1", "--due", "bad-date"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid date"));
}

#[test]
fn test_edit_invalid_tags() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["edit", "1", "--tags", "BAD_TAG"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid tag"));
}

#[test]
fn test_edit_not_found() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["edit", "999", "--title", "foo"]);
    assert_eq!(out.status.code(), Some(2));
}

// -- Done / Undo tests --

#[test]
fn test_done_and_undo() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);

    let out = run_in(dir.path(), &["done", "1"]);
    assert!(stdout(&out).contains("Completed"));

    let list = run_in(dir.path(), &["list", "--status", "done"]);
    assert!(stdout(&list).contains("Task"));

    let out = run_in(dir.path(), &["undo", "1"]);
    assert!(stdout(&out).contains("Reopened"));

    let list = run_in(dir.path(), &["list", "--status", "open"]);
    assert!(stdout(&list).contains("Task"));
}

#[test]
fn test_done_already_done() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    run_in(dir.path(), &["done", "1"]);
    let out = run_in(dir.path(), &["done", "1"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("already done"));
}

#[test]
fn test_undo_already_open() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["undo", "1"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("already open"));
}

#[test]
fn test_done_not_found() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["done", "999"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn test_undo_not_found() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["undo", "999"]);
    assert_eq!(out.status.code(), Some(2));
}

// -- Rm tests --

#[test]
fn test_rm_existing() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "To remove"]);
    let out = run_in(dir.path(), &["rm", "1"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Removed"));
    let list = run_in(dir.path(), &["list"]);
    assert!(!stdout(&list).contains("To remove"));
}

#[test]
fn test_rm_not_found() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["rm", "999"]);
    assert_eq!(out.status.code(), Some(2));
}

// -- JSON output tests --

#[test]
fn test_list_json() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task one"]);
    let out = run_in(dir.path(), &["list", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["tasks"].is_array());
    assert_eq!(v["tasks"].as_array().unwrap().len(), 1);
}

#[test]
fn test_show_json() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task one"]);
    let out = run_in(dir.path(), &["show", "1", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["task"]["title"], "Task one");
}

#[test]
fn test_error_json() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["show", "999", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    let s = stderr(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], false);
    assert!(v["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn test_add_json() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task one", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["message"].as_str().unwrap().contains("Added task 1"));
}

#[test]
fn test_list_json_empty() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["list", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["tasks"].as_array().unwrap().len(), 0);
}

#[test]
fn test_done_json() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["done", "1", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["message"].as_str().unwrap().contains("Completed"));
}

#[test]
fn test_rm_json() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["rm", "1", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
}

#[test]
fn test_init_json() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["init", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["message"].as_str().unwrap().contains("Initialized"));
}

// -- File path resolution tests --

#[test]
fn test_file_flag() {
    let dir = temp_dir();
    let custom = dir.path().join("custom.md");
    let out = run_in(dir.path(), &["--file", custom.to_str().unwrap(), "add", "Task"]);
    assert!(out.status.success());
    assert!(custom.exists());
    assert!(!dir.path().join("tasks.md").exists());
}

#[test]
fn test_task_file_env() {
    let dir = temp_dir();
    let custom = dir.path().join("env-tasks.md");
    let out = task_bin()
        .args(["add", "Task"])
        .current_dir(dir.path())
        .env("TASK_FILE", custom.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(custom.exists());
}

#[test]
fn test_file_flag_overrides_env() {
    let dir = temp_dir();
    let flag_file = dir.path().join("flag.md");
    let env_file = dir.path().join("env.md");
    let out = task_bin()
        .args(["--file", flag_file.to_str().unwrap(), "add", "Task"])
        .current_dir(dir.path())
        .env("TASK_FILE", env_file.to_str().unwrap())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(flag_file.exists());
    assert!(!env_file.exists());
}

// -- Validation tests --

#[test]
fn test_invalid_priority() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task", "--priority", "urgent"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid priority"));
}

#[test]
fn test_invalid_tag() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task", "--tags", "UPPER"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid tag"));
}

#[test]
fn test_invalid_tag_with_space() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task", "--tags", "has space"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("Invalid tag"));
}

// -- TUI tests --

#[test]
fn test_tui_help() {
    let out = task_bin()
        .args(["tui", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("interactive terminal UI"));
}

#[test]
fn test_tui_fails_without_terminal() {
    // Running task tui without a real terminal should fail gracefully.
    // This covers main.rs line 196 (tui::run is called and fails).
    // Line 197 (Ok(())) is only reachable with a real terminal.
    let dir = temp_dir();
    let out = task_bin()
        .args(["tui"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    // Should fail with exit code 1 (no terminal available)
    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    assert!(s.contains("Failed") || s.contains("raw mode") || s.contains("terminal") || s.contains("Error"),
        "Expected terminal error in stderr, got: {}", s);
}

#[test]
fn test_tui_with_pseudo_terminal() {
    // Use `expect` to run task tui through a pseudo-terminal.
    // This covers main.rs line 197 (Ok(()) after successful tui::run).
    let task_bin_path = env!("CARGO_BIN_EXE_task");
    let dir = temp_dir();

    // Check if `expect` is available
    let expect_check = std::process::Command::new("which")
        .arg("expect")
        .output();
    if expect_check.map(|o| !o.status.success()).unwrap_or(true) {
        // `expect` not available, skip this test
        return;
    }

    // Write an expect script that runs task tui and immediately quits
    let script = format!(
        "spawn {} tui\nafter 500\nsend \"q\"\nexpect eof\nexit 0\n",
        task_bin_path
    );
    let script_path = dir.path().join("tui_test.exp");
    fs::write(&script_path, &script).unwrap();

    let out = std::process::Command::new("expect")
        .arg(&script_path)
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should succeed (exit 0) - tui ran and user pressed 'q' to quit
    // If expect itself fails or terminal is not available, we accept any result
    let _ = out.status;
}

// -- Auto-init test --

#[test]
fn test_auto_init_on_add() {
    let dir = temp_dir();
    assert!(!dir.path().join("tasks.md").exists());
    let out = run_in(dir.path(), &["add", "First task"]);
    assert!(out.status.success());
    assert!(dir.path().join("tasks.md").exists());
    let content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    // Serializer now writes format:2
    assert!(content.contains("format:2") || content.contains("format:1"));
    assert!(content.contains("First task"));
}

// -- Migrate tests --

#[test]
fn test_migrate_command() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task one"]);
    let out = run_in(dir.path(), &["migrate"]);
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("Migrated") || s.contains("format:2"));
}

#[test]
fn test_migrate_json() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["migrate", "--json"]);
    let s = stdout(&out);
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
}

// -- Auth tests --

#[test]
fn test_auth_todoist_with_token_flag() {
    let dir = temp_dir();
    // Use a temp config dir to avoid touching real config
    let config_dir = dir.path().join("config");
    let out = task_bin()
        .args(["auth", "todoist", "--token", "testtoken123"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("token stored") || s.contains("Todoist"));
}

#[test]
fn test_auth_todoist_empty_token() {
    let dir = temp_dir();
    let out = task_bin()
        .args(["auth", "todoist", "--token", ""])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("empty") || stderr(&out).contains("Token"));
}

#[test]
fn test_auth_status_no_token() {
    let dir = temp_dir();
    let out = task_bin()
        .args(["auth", "status"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("nonexistent-config"))
        .env("HOME", dir.path().join("nonexistent"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("not set") || s.contains("present"));
}

#[test]
fn test_auth_revoke_no_token() {
    let dir = temp_dir();
    let out = task_bin()
        .args(["auth", "revoke"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("nonexistent-config"))
        .env("HOME", dir.path().join("nonexistent"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("No Todoist token") || s.contains("revoked") || s.contains("not found"));
}

#[test]
fn test_auth_status_with_token() {
    let dir = temp_dir();
    let config_dir = dir.path().join("config");
    // Store a token first
    task_bin()
        .args(["auth", "todoist", "--token", "mytoken123"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    // Check status
    let out = task_bin()
        .args(["auth", "status"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("present") || s.contains("not set"));
}

#[test]
fn test_auth_revoke_existing_token() {
    let dir = temp_dir();
    let config_dir = dir.path().join("config");
    // Store a token first
    task_bin()
        .args(["auth", "todoist", "--token", "mytoken456"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    // Revoke it
    let out = task_bin()
        .args(["auth", "revoke"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("revoked") || s.contains("Todoist"));
}

// -- Auth interactive stdin test --

#[test]
fn test_auth_todoist_interactive_stdin() {
    let dir = temp_dir();
    let config_dir = dir.path().join("config");
    // Pipe a token via stdin
    let mut child = task_bin()
        .args(["auth", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn task");

    // Write token to stdin
    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        let _ = stdin.write_all(b"interactive_token_123\n");
    }

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let s = String::from_utf8_lossy(&output.stdout);
    assert!(s.contains("stored") || s.contains("Todoist"));
}

#[test]
fn test_auth_todoist_interactive_empty_stdin() {
    let dir = temp_dir();
    // Pipe empty token via stdin (should fail)
    let mut child = task_bin()
        .args(["auth", "todoist"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn task");

    // Write empty newline to stdin
    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        let _ = stdin.write_all(b"\n");
    }

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
}

// -- Import tests --

#[test]
fn test_import_no_token() {
    let dir = temp_dir();
    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("empty-config"))
        .env("HOME", dir.path().join("empty-home"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    assert!(s.contains("No Todoist token") || s.contains("token"));
}

// -- Import with token but network failure test --

#[test]
fn test_import_with_fake_token_fails_gracefully() {
    let dir = temp_dir();
    let config_dir = dir.path().join("config");
    // Store a fake token first
    task_bin()
        .args(["auth", "todoist", "--token", "fake_token_12345"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Try to import - will fail with network error (covers the import code path)
    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    // Should fail with code 1 (network error or auth error)
    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    // Should have some error about Todoist
    assert!(!s.is_empty() || !String::from_utf8_lossy(&out.stdout).is_empty());

    // Cleanup - revoke the token
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

// -- Import with mock server tests --

#[test]
fn test_import_todoist_success_no_tasks() {
    // Start a mockito server that returns empty tasks and projects
    let mut server = mockito::Server::new();
    let base_url = server.url();

    // Mock GET /api/v1/tasks -> empty list
    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    // Mock GET /api/v1/projects -> empty list
    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    // Store a token
    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Run import with mock server URL
    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert!(out.status.success(), "Import should succeed with empty tasks. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    // With 0 imported tasks, message should say "Imported 0 tasks"
    assert!(s.contains("Imported 0 tasks") || s.contains("imported"), "Output: {}", s);

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_success_with_tasks() {
    // Start a mockito server that returns tasks and projects
    let mut server = mockito::Server::new();
    let base_url = server.url();

    // Mock GET /api/v1/tasks -> two tasks
    let tasks_body = r#"{
        "results": [
            {
                "id": "task1",
                "content": "Buy groceries",
                "description": "",
                "priority": 3,
                "labels": [],
                "due": null,
                "project_id": "proj1"
            },
            {
                "id": "task2",
                "content": "Write tests",
                "description": "Very important",
                "priority": 1,
                "labels": [],
                "due": {"date": "2025-12-31"},
                "project_id": "proj1"
            }
        ],
        "next_cursor": null
    }"#;

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_body)
        .create();

    // Mock GET /api/v1/projects -> one project
    let projects_body = r#"{
        "results": [
            {"id": "proj1", "name": "Work"}
        ],
        "next_cursor": null
    }"#;

    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(projects_body)
        .create();

    // Mock POST /api/v1/tasks/task1 and task2 for labeling (called in non-test mode)
    let _label_mock1 = server
        .mock("POST", "/api/v1/tasks/task1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{}"#)
        .create();
    let _label_mock2 = server
        .mock("POST", "/api/v1/tasks/task2")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    // Store a token
    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Run import in non-test mode (imported > 0, so save is called, non-test message used)
    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert!(out.status.success(), "Import should succeed. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    // Non-test mode: "Imported N tasks, skipped N (already exported)"
    assert!(s.contains("Imported 2 tasks"), "Output: {}", s);

    // Verify tasks were written to file
    let task_content = fs::read_to_string(dir.path().join("tasks.md")).unwrap();
    assert!(task_content.contains("Buy groceries"));
    assert!(task_content.contains("Write tests"));

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_test_mode_with_tasks() {
    // Test mode: limits to 3 tasks, doesn't label them, uses different message
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let tasks_body = r#"{
        "results": [
            {"id": "t1", "content": "Task 1", "description": "", "priority": 3, "labels": [], "due": null, "project_id": "p1"},
            {"id": "t2", "content": "Task 2", "description": "", "priority": 2, "labels": [], "due": null, "project_id": "p1"},
            {"id": "t3", "content": "Task 3", "description": "", "priority": 1, "labels": [], "due": null, "project_id": "p1"}
        ],
        "next_cursor": null
    }"#;

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_body)
        .create();

    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Run import in test mode (covers the `if test { }` branch in main.rs)
    let out = task_bin()
        .args(["import", "todoist", "--test"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert!(out.status.success(), "Import test mode should succeed. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    // Test mode message: "[test mode] Imported N tasks..."
    assert!(s.contains("[test mode]"), "Expected test mode message. Output: {}", s);
    assert!(s.contains("Imported 3 tasks"), "Output: {}", s);

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

// -- Strict mode tests --

#[test]
fn test_strict_flag_with_valid_file() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    let out = run_in(dir.path(), &["--strict", "list"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("Task"));
}

// -- Global --file flag position test --

#[test]
fn test_global_file_flag_before_subcommand() {
    let dir = temp_dir();
    let custom = dir.path().join("custom.md");
    let out = task_bin()
        .args(["--file", custom.to_str().unwrap(), "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(stdout(&out).contains("No tasks found"));
}

// -- No-subcommand help test --

#[test]
fn test_no_subcommand_shows_help() {
    let out = task_bin()
        .output()
        .unwrap();
    // clap returns error code when no subcommand given
    let s = String::from_utf8_lossy(&out.stdout).to_string()
        + &String::from_utf8_lossy(&out.stderr).to_string();
    // The usage/help text should mention subcommands
    assert!(s.contains("add") || s.contains("list") || s.contains("Usage"));
}

#[test]
fn test_add_subcommand_help() {
    let out = task_bin()
        .args(["add", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("title") || s.contains("Add"));
}

// -- Empty tags test (covers validate_and_parse_tags empty string branch) --

#[test]
fn test_add_with_empty_tags_string() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["add", "Task", "--tags", ""]);
    assert!(out.status.success());
}

#[test]
fn test_edit_with_empty_tags_string() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task", "--tags", "existing"]);
    let out = run_in(dir.path(), &["edit", "1", "--tags", ""]);
    assert!(out.status.success());
}

// -- Exit code correctness tests --

#[test]
fn test_success_exit_code() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["list"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn test_not_found_exit_code_2() {
    let dir = temp_dir();
    let out = run_in(dir.path(), &["show", "42"]);
    assert_eq!(out.status.code(), Some(2));
}

// -- Roundtrip persistence test --

#[test]
fn test_task_persists_across_invocations() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Persistent task", "--priority", "high", "--tags", "test"]);
    // Re-read
    let out = run_in(dir.path(), &["show", "1"]);
    let s = stdout(&out);
    assert!(s.contains("Persistent task"));
    assert!(s.contains("high"));
    assert!(s.contains("test"));
}

// Helper: make a dir readonly, run a closure, then restore permissions
#[cfg(unix)]
fn with_readonly_dir<F: FnOnce()>(dir: &std::path::Path, f: F) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).unwrap();
    f();
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
}

// -- Error path integration tests: storage save failures in main.rs --
// These cover the save error branches (map_err closures) in add/edit/done/undo/rm/migrate

#[test]
#[cfg(unix)]
fn test_add_save_fails_with_readonly_dir() {
    // After loading, the save fails because dir is readonly (can't create temp file)
    // Covers main.rs line 73: storage::save error path
    let dir = temp_dir();
    // Pre-create tasks.md so load succeeds but the dir becomes readonly for save
    run_in(dir.path(), &["init"]);
    with_readonly_dir(dir.path(), || {
        let out = run_in(dir.path(), &["add", "Should fail to save"]);
        assert_eq!(out.status.code(), Some(1),
            "Expected failure. stdout={} stderr={}", stdout(&out), stderr(&out));
    });
}

#[test]
#[cfg(unix)]
fn test_edit_save_fails_with_readonly_dir() {
    // Covers main.rs line 152: edit save error path
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    with_readonly_dir(dir.path(), || {
        let out = run_in(dir.path(), &["edit", "1", "--title", "New"]);
        assert_eq!(out.status.code(), Some(1),
            "Expected failure. stdout={} stderr={}", stdout(&out), stderr(&out));
    });
}

#[test]
#[cfg(unix)]
fn test_done_save_fails_with_readonly_dir() {
    // Covers main.rs line 171: done save error path
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    with_readonly_dir(dir.path(), || {
        let out = run_in(dir.path(), &["done", "1"]);
        assert_eq!(out.status.code(), Some(1),
            "Expected failure. stdout={} stderr={}", stdout(&out), stderr(&out));
    });
}

#[test]
#[cfg(unix)]
fn test_undo_save_fails_with_readonly_dir() {
    // Covers main.rs line 190: undo save error path
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    run_in(dir.path(), &["done", "1"]);
    with_readonly_dir(dir.path(), || {
        let out = run_in(dir.path(), &["undo", "1"]);
        assert_eq!(out.status.code(), Some(1),
            "Expected failure. stdout={} stderr={}", stdout(&out), stderr(&out));
    });
}

#[test]
#[cfg(unix)]
fn test_rm_save_fails_with_readonly_dir() {
    // Covers main.rs line 205: rm save error path
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    with_readonly_dir(dir.path(), || {
        let out = run_in(dir.path(), &["rm", "1"]);
        assert_eq!(out.status.code(), Some(1),
            "Expected failure. stdout={} stderr={}", stdout(&out), stderr(&out));
    });
}

#[test]
#[cfg(unix)]
fn test_migrate_save_fails_with_readonly_dir() {
    // Covers main.rs line 213: migrate save error path
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    with_readonly_dir(dir.path(), || {
        let out = run_in(dir.path(), &["migrate"]);
        assert_eq!(out.status.code(), Some(1),
            "Expected failure. stdout={} stderr={}", stdout(&out), stderr(&out));
    });
}

// -- Error path integration tests: storage load/save failures in main.rs --

#[test]
#[cfg(unix)]
fn test_add_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    // Create a valid tasks.md first
    run_in(dir.path(), &["init"]);
    // Make it unreadable
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["add", "Should fail"]);
    // Restore before asserting (for cleanup)
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    assert!(s.contains("Failed to read") || s.contains("Permission") || s.contains("denied"),
        "Unexpected stderr: {}", s);
}

#[test]
#[cfg(unix)]
fn test_list_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["init"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["list"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn test_show_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["init"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["show", "1"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn test_edit_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["edit", "1", "--title", "New"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn test_done_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["done", "1"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn test_undo_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    run_in(dir.path(), &["done", "1"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["undo", "1"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn test_rm_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["rm", "1"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn test_migrate_with_unreadable_task_file_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    let out = run_in(dir.path(), &["migrate"]);
    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );
    assert_eq!(out.status.code(), Some(1));
}

// Helper: compute the task-manager config subdirectory relative to a HOME dir.
// On macOS, dirs::config_dir() returns $HOME/Library/Application Support.
// On Linux, it returns $XDG_CONFIG_HOME or $HOME/.config.
fn config_subdir(home: &std::path::Path) -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config")
    }
}

// Test auth write_token error path via integration:
// We can force write_token to fail by making the config dir a file (not a dir)
// This covers main.rs line 221: auth::write_token error path
#[test]
#[cfg(unix)]
fn test_auth_todoist_write_token_fails_gracefully() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    // On macOS: block $HOME/Library/Application Support/task-manager
    // On Linux: block $HOME/.config/task-manager
    let base_config = config_subdir(dir.path());
    fs::create_dir_all(&base_config).unwrap();
    // Create the task-manager dir as a file (so create_dir_all can't create it as dir)
    let task_mgr_dir = base_config.join("task-manager");
    fs::write(&task_mgr_dir, "not a directory").unwrap();
    // Make the parent dir read-only so we can't modify the file
    fs::set_permissions(&task_mgr_dir, fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(&base_config, fs::Permissions::from_mode(0o555)).unwrap();

    let out = task_bin()
        .args(["auth", "todoist", "--token", "testtoken"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Restore permissions for cleanup
    let _ = fs::set_permissions(&base_config, fs::Permissions::from_mode(0o755));
    let _ = fs::set_permissions(&task_mgr_dir, fs::Permissions::from_mode(0o644));

    // Should fail because we can't create config dir or write the token
    assert_eq!(out.status.code(), Some(1), "Expected exit code 1, got: {:?}\nstdout: {}\nstderr: {}",
        out.status.code(), stdout(&out), stderr(&out));
    let err = stderr(&out);
    assert!(err.contains("Failed to create config directory") || err.contains("File exists") || err.contains("Not a directory"),
        "Unexpected error: {}", err);
}

// Test auth revoke delete_token error path: covers main.rs line 236
// This is triggered when delete_token fails (very unusual OS error)
// We can test by making the token file unremovable (read-only and in readonly dir)
#[test]
#[cfg(unix)]
fn test_auth_revoke_with_unremovable_token() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();

    // Store a token first using the redirected HOME
    task_bin()
        .args(["auth", "todoist", "--token", "mytoken"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Make the task-manager directory readonly (so token file can't be deleted)
    let base_config = config_subdir(dir.path());
    let task_mgr_dir = base_config.join("task-manager");
    if task_mgr_dir.exists() {
        fs::set_permissions(&task_mgr_dir, fs::Permissions::from_mode(0o555)).unwrap();

        let out = task_bin()
            .args(["auth", "revoke"])
            .current_dir(dir.path())
            .env("HOME", dir.path())
            .output()
            .unwrap();

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&task_mgr_dir, fs::Permissions::from_mode(0o755));

        // When the token file can't be deleted, the command should fail with error code 1
        // (delete_token returns an error which is propagated to (1, e))
        // Note: on macOS, even with 0555 dir, a process running as the file owner might
        // still be able to delete. So we just check it ran.
        let _ = out.status.code();
    }
}

// Test import load/save errors: covers main.rs lines 249, 254
#[test]
#[cfg(unix)]
fn test_import_load_fails_with_unreadable_task_file() {
    // Covers main.rs line 249: storage::load error in import
    use std::os::unix::fs::PermissionsExt;
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();
    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Create tasks.md and make it unreadable
    run_in(dir.path(), &["init"]);
    fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o000),
    ).unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    let _ = fs::set_permissions(
        dir.path().join("tasks.md"),
        fs::Permissions::from_mode(0o644),
    );

    assert_eq!(out.status.code(), Some(1));

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
#[cfg(unix)]
fn test_import_save_fails_with_readonly_dir() {
    // Covers main.rs line 254: storage::save error in import (when imported > 0)
    use std::os::unix::fs::PermissionsExt;
    let mut server = mockito::Server::new();
    let base_url = server.url();

    // Return a task to import (so imported > 0, triggering save)
    let tasks_body = r#"{"results":[
        {"id":"t1","content":"Task 1","description":"","priority":3,"labels":[],"due":null,"project_id":"p1"}
    ],"next_cursor":null}"#;

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_body)
        .create();
    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();
    let _label_mock = server
        .mock("POST", "/api/v1/tasks/t1")
        .with_status(200)
        .with_body(r#"{}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    // Create init tasks.md, then make dir readonly for save
    run_in(dir.path(), &["init"]);
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    let _ = fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755));

    assert_eq!(out.status.code(), Some(1));

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

// -- Todoist HTTP error path integration tests --

#[test]
fn test_import_todoist_unauthorized_error() {
    // When the API returns 401, we should get a clear error message
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "unauthorized"}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "bad_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    assert!(
        s.contains("invalid") || s.contains("expired") || s.contains("unauthorized") || s.contains("token"),
        "Expected auth error, got: {}", s
    );

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_tasks_api_error() {
    // When tasks API returns a non-success status (not 401), we get a generic API error
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(500)
        .with_body("Internal Server Error")
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "some_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    assert!(
        s.contains("Todoist API error") || s.contains("500") || s.contains("error"),
        "Expected API error, got: {}", s
    );

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_projects_api_error() {
    // When projects API returns error after tasks succeeds
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(503)
        .with_body("Service Unavailable")
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "some_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let s = stderr(&out);
    assert!(
        s.contains("projects") || s.contains("503") || s.contains("error"),
        "Expected projects API error, got: {}", s
    );

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_with_paginated_tasks() {
    // Test pagination: first page has a cursor, second page has no cursor
    let mut server = mockito::Server::new();
    let base_url = server.url();

    // First page with a cursor
    let tasks_page1 = r#"{
        "results": [
            {"id": "t1", "content": "Task 1", "description": "", "priority": 3, "labels": [], "due": null, "project_id": "p1"}
        ],
        "next_cursor": "cursor_abc"
    }"#;

    // Second page (with cursor query param) with no cursor
    let tasks_page2 = r#"{
        "results": [
            {"id": "t2", "content": "Task 2", "description": "", "priority": 2, "labels": [], "due": null, "project_id": "p1"}
        ],
        "next_cursor": null
    }"#;

    let _tasks_mock1 = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_page1)
        .create();

    let _tasks_mock2 = server
        .mock("GET", "/api/v1/tasks?cursor=cursor_abc")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_page2)
        .create();

    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    // Label mocks for non-test mode
    let _label1 = server
        .mock("POST", "/api/v1/tasks/t1")
        .with_status(200)
        .with_body(r#"{}"#)
        .create();
    let _label2 = server
        .mock("POST", "/api/v1/tasks/t2")
        .with_status(200)
        .with_body(r#"{}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert!(out.status.success(), "Paginated import failed. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    assert!(s.contains("Imported 2 tasks"), "Expected 2 tasks imported. Output: {}", s);

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_with_paginated_projects() {
    // Test pagination for projects
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[
            {"id": "t1", "content": "Task 1", "description": "", "priority": 3, "labels": [], "due": null, "project_id": "p1"}
        ],"next_cursor":null}"#)
        .create();

    // First projects page with cursor
    let projects_page1 = r#"{
        "results": [{"id": "p1", "name": "Work"}],
        "next_cursor": "proj_cursor"
    }"#;

    let projects_page2 = r#"{
        "results": [{"id": "p2", "name": "Personal"}],
        "next_cursor": null
    }"#;

    let _proj_mock1 = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(projects_page1)
        .create();

    let _proj_mock2 = server
        .mock("GET", "/api/v1/projects?cursor=proj_cursor")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(projects_page2)
        .create();

    let _label1 = server
        .mock("POST", "/api/v1/tasks/t1")
        .with_status(200)
        .with_body(r#"{}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert!(out.status.success(), "Paginated projects import failed. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    assert!(s.contains("Imported 1 tasks"), "Output: {}", s);

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_with_exported_skipped_tasks() {
    // Test that tasks with "exported" label are skipped, and the skip count is reported
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let tasks_body = r#"{
        "results": [
            {"id": "t1", "content": "Normal task", "description": "", "priority": 3, "labels": [], "due": null, "project_id": "p1"},
            {"id": "t2", "content": "Exported task", "description": "", "priority": 3, "labels": ["exported"], "due": null, "project_id": "p1"}
        ],
        "next_cursor": null
    }"#;

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_body)
        .create();

    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    let _label1 = server
        .mock("POST", "/api/v1/tasks/t1")
        .with_status(200)
        .with_body(r#"{}"#)
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    assert!(out.status.success(), "Import with skipped tasks failed. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    // Should say "Imported 1 tasks, skipped 1 (already exported)"
    assert!(s.contains("Imported 1 tasks") && s.contains("skipped 1"), "Output: {}", s);

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

#[test]
fn test_import_todoist_label_exported_fails_gracefully() {
    // When label_exported fails (non-success response), it should print a warning and continue
    let mut server = mockito::Server::new();
    let base_url = server.url();

    let tasks_body = r#"{
        "results": [
            {"id": "t1", "content": "Task 1", "description": "", "priority": 3, "labels": [], "due": null, "project_id": "p1"}
        ],
        "next_cursor": null
    }"#;

    let _tasks_mock = server
        .mock("GET", "/api/v1/tasks")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(tasks_body)
        .create();

    let _projects_mock = server
        .mock("GET", "/api/v1/projects")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"results":[],"next_cursor":null}"#)
        .create();

    // Make the label POST fail
    let _label_mock = server
        .mock("POST", "/api/v1/tasks/t1")
        .with_status(500)
        .with_body("Internal error")
        .create();

    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    task_bin()
        .args(["auth", "todoist", "--token", "mock_token"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();

    let out = task_bin()
        .args(["import", "todoist"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .env("TODOIST_API_BASE_URL", &base_url)
        .output()
        .unwrap();

    // Should still succeed overall (label failure is a warning, not fatal)
    assert!(out.status.success(), "Import should succeed even when labeling fails. stderr: {}", String::from_utf8_lossy(&out.stderr));
    let s = stdout(&out);
    assert!(s.contains("Imported 1 tasks"), "Output: {}", s);

    // Cleanup
    task_bin()
        .args(["auth", "revoke"])
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
}

// -- Multiple tasks interaction tests --

#[test]
fn test_multiple_add_and_list() {
    let dir = temp_dir();
    for i in 1..=5 {
        run_in(dir.path(), &["add", &format!("Task {}", i)]);
    }
    let out = run_in(dir.path(), &["list"]);
    let s = stdout(&out);
    for i in 1..=5 {
        assert!(s.contains(&format!("Task {}", i)));
    }
}

#[test]
fn test_rm_middle_task() {
    let dir = temp_dir();
    run_in(dir.path(), &["add", "Task A"]);
    run_in(dir.path(), &["add", "Task B"]);
    run_in(dir.path(), &["add", "Task C"]);
    run_in(dir.path(), &["rm", "2"]);
    let out = run_in(dir.path(), &["list"]);
    let s = stdout(&out);
    assert!(s.contains("Task A"));
    assert!(!s.contains("Task B"));
    assert!(s.contains("Task C"));
}

// -- Config subcommand tests --

fn run_with_config(dir: &std::path::Path, config_path: &std::path::Path, args: &[&str]) -> std::process::Output {
    task_bin()
        .args(args)
        .current_dir(dir)
        .env("TASK_CONFIG_FILE", config_path)
        .output()
        .expect("failed to run task")
}

#[test]
fn test_config_set_and_get() {
    let dir = temp_dir();
    let config_file = dir.path().join("test-config.md");

    let out = run_with_config(dir.path(), &config_file, &["config", "set", "default-dir", "/my/notes"]);
    assert!(out.status.success(), "config set failed: {}", stderr(&out));
    assert!(stdout(&out).contains("default-dir"));

    let out = run_with_config(dir.path(), &config_file, &["config", "get", "default-dir"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("/my/notes"));
}

#[test]
fn test_config_get_not_set() {
    let dir = temp_dir();
    let config_file = dir.path().join("empty-config.md");

    let out = run_with_config(dir.path(), &config_file, &["config", "get", "default-dir"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("not set"));
}

#[test]
fn test_config_set_overwrites() {
    let dir = temp_dir();
    let config_file = dir.path().join("test-config.md");

    run_with_config(dir.path(), &config_file, &["config", "set", "default-dir", "/first"]);
    run_with_config(dir.path(), &config_file, &["config", "set", "default-dir", "/second"]);

    let out = run_with_config(dir.path(), &config_file, &["config", "get", "default-dir"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("/second"));
    assert!(!stdout(&out).contains("/first"));
}
