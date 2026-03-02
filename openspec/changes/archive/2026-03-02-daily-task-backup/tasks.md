## 1. Backup function

- [x] 1.1 Add `pub fn backup_daily(task_file_path: &Path)` to `storage.rs`. If the task file doesn't exist, return immediately. Otherwise: create `.backups/` sibling dir, copy task file to `tasks-YYYY-MM-DD.md`, prune files beyond 7. Wrap all operations in error-swallowing logic (silently return on any failure).
- [x] 1.2 Implement the pruning logic: list `tasks-*.md` in `.backups/`, sort by name descending, delete all beyond the first 7.

## 2. Call site

- [x] 2.1 In `main.rs`, call `storage::backup_daily(&path)` after resolving the file path and before dispatching to any subcommand.

## 3. Tests

- [x] 3.1 Add test `backup_daily_creates_backup_file`: create a task file in a temp dir, call `backup_daily`, assert `.backups/tasks-YYYY-MM-DD.md` exists with matching content.
- [x] 3.2 Add test `backup_daily_prunes_old_files`: create 9 fake backup files in `.backups/`, call `backup_daily` (which creates a 10th), assert only 7 remain and the 3 oldest are gone.
- [x] 3.3 Add test `backup_daily_no_file_no_error`: call `backup_daily` with a nonexistent path, assert no panic and no `.backups/` directory created.
- [x] 3.4 Add test `backup_daily_overwrites_same_day`: create a backup file for today with different content, call `backup_daily`, assert the backup now matches the current task file.
