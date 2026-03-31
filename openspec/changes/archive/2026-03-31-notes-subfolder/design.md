## Context

All note functions in `src/note.rs` (`write_note`, `read_note`, `delete_note`, `discover_notes`, `unique_slug`) already accept a `dir: &Path` parameter. The storage location is entirely controlled by the caller. The change is purely in what path is passed — no changes to `note.rs` itself are needed.

Current call sites pass `app.task_dir()` (the directory containing `tasks.md`). After this change they will pass `app.notes_dir()` which returns `task_dir().join("Notes")`.

## Goals / Non-Goals

**Goals:**
- Notes stored in `<task-dir>/Notes/` for both TUI and CLI
- `Notes/` directory created automatically on first write
- Clean, minimal change — only call sites updated

**Non-Goals:**
- Auto-migration of existing notes from parent dir to `Notes/`
- Configurable notes directory name
- Nested note subdirectories

## Decisions

**Add `notes_dir()` helper to `App`**
Centralises the path computation. All TUI call sites replace `task_dir()` with `notes_dir()` for note operations. Avoids scattering `.join("Notes")` calls throughout the code.

**Auto-create `Notes/` on write**
`write_note` already calls `fs::write`. Wrap the write in `fs::create_dir_all(dir)` before writing so the directory is created transparently on first use. This is the only change needed in `note.rs`.

**CLI resolves notes dir the same way**
The CLI resolves `task_dir` from the file path, then appends `Notes/`. No new config key needed.

**No migration**
Existing notes in the parent directory will not appear in the Notes view after this change. Users need to manually `mv *.md Notes/` (excluding `tasks.md`). This is acceptable — the change is opt-in via deploy.

## Risks / Trade-offs

- [Risk] Existing notes become invisible after upgrade → Mitigation: document in deploy notes; no data is lost, just needs a manual move.
- [Risk] `discover_notes` currently excludes the task filename by name; after this change the notes dir contains only `.md` notes so the exclusion is a no-op but harmless.

## Migration Plan

1. Deploy new binary
2. Manually move existing note files: `mkdir -p Notes && mv *.md Notes/` (from the task directory, excluding `tasks.md`)
