## Why

Task data lives in a single markdown file with no recovery mechanism. If the file gets corrupted, accidentally deleted, or badly edited, all tasks are lost. A lightweight daily backup provides a safety net without complicating the core application.

## What Changes

- On application startup (both CLI subcommands and TUI), copy the current task file to a date-stamped backup in a `.backups/` sibling directory
- Keep only the most recent 7 daily backups; delete older ones
- Backup files are passive copies — the application never reads from them

## Capabilities

### New Capabilities

- `task-backup`: Daily backup of the task file on application startup with 7-day retention

### Modified Capabilities

_(none)_

## Impact

- `src/storage.rs`: New `backup_daily` function
- `src/main.rs`: Call `backup_daily` after resolving the task file path, before any subcommand runs
- `src/tui.rs`: Call `backup_daily` in `App::new` after loading tasks
- No new dependencies (uses `std::fs` and `chrono` which are already in use)
