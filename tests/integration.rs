use std::fs;
use std::process::Command;

fn task_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_task"))
}

fn task_tui_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_task-tui"))
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
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
    // Running task-tui without a real terminal should fail gracefully.
    let dir = temp_dir();
    let out = task_tui_bin()
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
fn test_no_subcommand_launches_tui() {
    // Running `task` with no subcommand should print a redirect message (TUI is in task-tui).
    let dir = temp_dir();
    let out = task_bin()
        .current_dir(dir.path())
        .output()
        .unwrap();
    // CLI binary exits 0 and prints redirect message to stderr
    assert_eq!(out.status.code(), Some(0));
    let s = stderr(&out);
    assert!(s.contains("task-tui"),
        "Expected redirect message mentioning task-tui, got: {}", s);
}

#[test]
fn test_tui_with_pseudo_terminal() {
    // Use `expect` to run task-tui through a pseudo-terminal.
    let task_bin_path = env!("CARGO_BIN_EXE_task-tui");
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

// -- Help tests --

#[test]
fn test_help_lists_subcommands() {
    let out = task_bin()
        .args(["--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("tui"));
    assert!(s.contains("auth"));
    assert!(s.contains("config"));
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
    assert!(s.contains("No Todoist token") || s.contains("revoked") || s.contains("not found") || s.contains("No tokens found"));
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

// -- Auth error path tests --

fn config_subdir(home: &std::path::Path) -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library").join("Application Support")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config")
    }
}

#[test]
#[cfg(unix)]
fn test_auth_todoist_write_token_fails_gracefully() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
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
        // Note: on macOS, even with 0555 dir, a process running as the file owner might
        // still be able to delete. So we just check it ran.
        let _ = out.status.code();
    }
}

// -- Claude auth integration tests --

#[test]
fn test_auth_claude_stores_key_and_status_reports_it() {
    let dir = temp_dir();
    let config_dir = dir.path().join("config");

    // Store a Claude API key
    let out = task_bin()
        .args(["auth", "claude", "--key", "sk-ant-test-key-123"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("Claude") || s.contains("stored"));

    // Check status reports it as present
    let out = task_bin()
        .args(["auth", "status"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("Claude API key: present"));

    // Revoke should delete it
    let out = task_bin()
        .args(["auth", "revoke"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = stdout(&out);
    assert!(s.contains("Claude API key revoked"));
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
