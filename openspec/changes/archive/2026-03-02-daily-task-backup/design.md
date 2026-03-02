## Context

The task file is resolved via `storage::resolve_file_path` and loaded via `storage::load`. Both CLI commands and the TUI call these at startup. There is no backup or recovery mechanism today.

## Goals / Non-Goals

**Goals:**
- Create a daily backup of the task file each time the application starts
- Retain 7 days of backups, automatically pruning older ones
- Backups are passive — never read by the application

**Non-Goals:**
- Backup restoration commands (users can manually copy files)
- Backup on every save (too frequent, one per day is sufficient)
- Backup of the config file (only the task file)

## Decisions

### 1. Backup location: `.backups/` sibling directory

Store backups in a `.backups/` directory next to the task file. For example, if the task file is `/home/user/notes/tasks.md`, backups go to `/home/user/notes/.backups/`. The dot-prefix keeps it hidden on Unix systems.

### 2. Backup filename: `tasks-YYYY-MM-DD.md`

Use the date as the filename suffix. If a backup for today already exists, overwrite it (this updates the daily snapshot to the latest version each time the app opens).

### 3. Retention: keep 7 most recent files

After writing the backup, list all files matching `tasks-*.md` in the `.backups/` directory, sort by name (which sorts chronologically since dates are ISO-formatted), and delete all but the 7 most recent.

### 4. Call site: `storage::backup_daily(path)`

Add a `pub fn backup_daily(task_file_path: &Path)` function in `storage.rs`. It takes the resolved task file path, silently returns if the file doesn't exist, and logs errors to stderr without crashing (backups are best-effort).

Call this from two places:
- `main.rs`: after `resolve_file_path`, before dispatching to any subcommand
- `tui.rs`: in `App::new` after loading tasks (covers the `D:set-dir` reload path too)

Actually, calling from `main.rs` before subcommand dispatch covers all entry points including TUI, so a single call site in `main.rs` is sufficient. The TUI reload (`D:set-dir`) would also benefit from a backup call, but that's an edge case — the daily backup from startup already captured the state.

**Decision: single call in `main.rs` only.**

### 5. Best-effort, no-crash

Backup failures (permission errors, disk full) are silently ignored. The function returns `()` and prints nothing. This ensures backups never break the primary workflow.

## Risks / Trade-offs

- **Disk usage**: 7 copies of the task file. For typical task files (< 100KB), this is negligible.
- **No user visibility**: Users won't know backups exist unless they look. This is intentional — backups are a safety net, not a feature to interact with.
- **Overwrite on same day**: Opening the app multiple times in one day overwrites the day's backup. This means the backup reflects the state at the *last* app launch that day, not the first. Acceptable trade-off for simplicity.
