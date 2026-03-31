## 1. note.rs — Auto-create Notes directory on write

- [x] 1.1 In `write_note`, call `fs::create_dir_all(dir)` before writing the note file so `Notes/` is created automatically on first use

## 2. tui.rs — Add notes_dir() helper and update call sites

- [x] 2.1 Add `fn notes_dir(&self) -> PathBuf` to `App` returning `self.task_dir().join("Notes")`
- [x] 2.2 Update `refresh_notes()` to pass `self.notes_dir()` instead of `self.task_dir()`
- [x] 2.3 Replace all `app.task_dir()` calls in note operations with `app.notes_dir()`:
  - `write_note(&app.task_dir(), ...)` → `write_note(&app.notes_dir(), ...)`
  - `unique_slug(&app.task_dir(), ...)` → `unique_slug(&app.notes_dir(), ...)`
  - `delete_note(&app.task_dir(), ...)` → `delete_note(&app.notes_dir(), ...)`
  - `app.task_dir().join(format!("{}.md", slug))` for note paths → `app.notes_dir().join(...)`

## 3. bin/task.rs — CLI notes dir

- [x] 3.1 Change `let dir = path.parent()...` for note subcommands to `let dir = path.parent()...join("Notes")`
  - Only the note subcommand branch needs this; task operations still use the parent dir

## 4. note.rs — Update discover_notes to handle missing Notes dir gracefully

- [x] 4.1 In `discover_notes`, if `dir` does not exist return an empty `Vec` instead of propagating an error or panicking (already may be handled — verify and fix if not)

## 5. Tests

- [x] 5.1 Update any existing note tests that write to/read from the task dir to use a `Notes/` subdirectory path
- [x] 5.2 Add a test verifying `write_note` creates the `Notes/` directory if absent
- [x] 5.3 Add a test verifying `discover_notes` returns empty when the `Notes/` directory does not exist
