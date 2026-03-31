## Why

Notes are currently stored in the same directory as `tasks.md`, mixing note files with the task file. Moving notes to a dedicated `Notes/` subfolder separates concerns, makes the directory cleaner, and makes it obvious where to find notes.

## What Changes

- Notes are stored in `<task-dir>/Notes/` instead of `<task-dir>/`
- The `Notes/` directory is created automatically when the first note is written
- All note read/write/discover/delete operations target `<task-dir>/Notes/`
- The TUI passes the notes directory (`<task-dir>/Notes/`) instead of `task_dir()` to all note functions
- The CLI `task note` subcommands resolve the notes directory the same way
- Existing notes in the parent directory are **not** auto-migrated (manual move required)

## Capabilities

### New Capabilities

*(none — this is a storage location change only)*

### Modified Capabilities

- `note-storage`: Note files are stored in a `Notes/` subdirectory of the task directory, not the task directory itself. Discovery, read, write, and delete all operate on this subdirectory.

## Impact

- `src/note.rs`: No changes needed — all functions already accept a `dir: &Path` argument
- `src/tui.rs`: Add `fn notes_dir(&self) -> PathBuf` returning `task_dir().join("Notes")`; replace all `task_dir()` calls in note operations with `notes_dir()`; update `refresh_notes()` to pass `notes_dir()`
- `src/bin/task.rs`: Note CLI subcommands must resolve and pass the notes directory
- `openspec/specs/note-storage/spec.md`: Delta spec updating storage location requirement
