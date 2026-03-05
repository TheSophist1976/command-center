## Context

The task manager stores tasks in a single markdown file (`tasks.md`) in a user-configured directory. The TUI uses a mode-based state machine for editing (Normal, Adding, EditingDescription, EditingDetailPanel, NlpChat, etc.). Text input uses a single `input_buffer: String` field — currently single-line only. The NLP system supports actions like Filter, Update, Message, ShowTasks, and SetRecurrence. File I/O uses atomic writes with temp files and file locking via `src/storage.rs`.

## Goals / Non-Goals

**Goals:**
- Store notes as individual `.md` files in the same directory as `tasks.md`
- Provide a Notes view in the TUI for listing/creating/editing notes
- Support multi-line inline editing in the TUI for note content
- Allow optional note-to-task linking via a `note` metadata field
- Enable AI to create and edit notes via new NLP actions

**Non-Goals:**
- Rich markdown rendering/preview in the TUI (just plain text editing)
- Note search/full-text search across notes
- Note versioning or conflict resolution
- Subdirectories or hierarchical note organization
- Attachments or embedded images

## Decisions

### 1. Notes stored as individual `.md` files

Each note is a separate `<slug>.md` file in the task directory. The slug is derived from the title (lowercase, hyphens for spaces, alphanumeric only). The first line of the file is a `# Title` heading, followed by the body content.

**Alternative**: Store notes in a single `notes.md` file. Rejected — individual files are simpler to manage, easier for external tools to consume, and avoid parsing complexity.

### 2. Note discovery via filesystem scan

List notes by globbing `*.md` in the task directory, excluding `tasks.md` itself and any backup files. No separate index file needed.

**Alternative**: Maintain a notes index in the task file. Rejected — adds coupling and sync issues. The filesystem is the index.

### 3. Multi-line TUI editor using a `Vec<String>` buffer

Add a new `NoteEditor` struct with a `lines: Vec<String>` buffer, a cursor position `(row, col)`, and basic editing operations (insert char, newline, backspace, delete line). This is separate from the existing `input_buffer` to avoid conflicts.

The editor supports:
- Character input, Enter for newline, Backspace/Delete
- Arrow keys for cursor movement
- Ctrl+S to save, Escape to exit (with dirty confirmation)
- Basic viewport scrolling when content exceeds visible area

**Alternative**: Reuse `input_buffer` with newline support. Rejected — would break all existing single-line input modes.

### 4. Notes view as a new `View` variant

Add `View::Notes` to the view enum, accessible via the `v`/`V` cycle or a direct keybinding (`n`). The Notes view shows a list of note files with title and last-modified date. Selecting a note opens the inline editor.

### 5. Task-note link via `note` metadata field

Add an optional `note: Option<String>` field to the Task struct. The value is the note's slug (filename without `.md`). The TUI detail panel shows the linked note and allows navigation to it. Creating a note from a task context auto-links it.

**Alternative**: Store the link in the note file. Rejected — keeping it on the task side is consistent with existing metadata patterns (tags, project, recurrence) and makes it easy to show in the task list.

### 6. NLP actions: CreateNote and EditNote

Add two new NLP action variants:
- `CreateNote { title, content, task_id: Option<u32> }` — creates a new note file, optionally links it to a task
- `EditNote { slug, content, description }` — replaces the body content of an existing note

The system prompt is updated with note-related instructions and examples.

## Risks / Trade-offs

- **[Filename collisions]** Two notes with the same title after slugification would collide. → Append a numeric suffix if the slug already exists.
- **[Large notes]** The inline editor loads the entire file into memory. → Acceptable for notes (not megabyte files). No mitigation needed.
- **[External edits]** Notes edited outside the TUI won't trigger a refresh. → Reload notes list on entering the Notes view. Acceptable for v1.
- **[Multi-line editor complexity]** Building a text editor is non-trivial. → Keep it minimal: no syntax highlighting, no undo/redo, no word wrap editing. Just basic insert/delete/navigate.
