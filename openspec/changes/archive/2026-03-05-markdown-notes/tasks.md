## 1. Note Storage Module

- [x] 1.1 Create `src/note.rs` module with `Note` struct (slug, title, body), slugify function, and note read/write functions
- [x] 1.2 Add note discovery function: scan task directory for `*.md` files excluding the task file, parse titles
- [x] 1.3 Add slug collision avoidance (append numeric suffix if slug exists)
- [x] 1.4 Add unit tests for slugify, read/write, discovery, and collision avoidance

## 2. Task Note Field

- [x] 2.1 Add `note: Option<String>` field to `Task` struct in `src/task.rs`
- [x] 2.2 Update `src/parser.rs` to parse/serialize `note:<slug>` metadata key
- [x] 2.3 Add unit tests for note metadata round-trip

## 3. Notes View in TUI

- [x] 3.1 Add `View::Notes` variant to view enum, update `next()`/`prev()`/`display_name()`/`from_config()` for view cycle (Recurring → Notes → Today)
- [x] 3.2 Implement notes list rendering: show note titles and last-modified dates, handle empty state
- [x] 3.3 Add `a` keybinding in Notes view to create a new note (prompt for title, then open editor)
- [x] 3.4 Add `Enter` keybinding to open selected note in the inline editor
- [x] 3.5 Add `d` keybinding to delete a note with confirmation, clearing any task links
- [x] 3.6 Add Notes view footer keybinding hints

## 4. Inline Multi-Line Note Editor

- [x] 4.1 Create `NoteEditor` struct with `lines: Vec<String>`, cursor `(row, col)`, viewport offset, and dirty flag
- [x] 4.2 Implement character input, Enter (newline), Backspace, arrow key navigation, and column clamping
- [x] 4.3 Implement viewport scrolling to keep cursor visible
- [x] 4.4 Add `Mode::EditingNote` and render the editor with line numbers and cursor indicator
- [x] 4.5 Implement Ctrl+S to save note to disk, Escape to exit with dirty confirmation
- [x] 4.6 Add unit tests for editor operations (insert, delete, newline, cursor movement)

## 5. Task-Note Linking

- [x] 5.1 Add `n` keybinding in Normal mode to attach/change/clear a note link on the selected task (show note picker)
- [x] 5.2 Add `g` keybinding in Normal mode to open a task's linked note in the editor
- [x] 5.3 Display note link indicator in task list rows for tasks with a linked note
- [x] 5.4 Show linked note slug in the task detail panel

## 6. NLP Note Actions

- [x] 6.1 Add `CreateNote` and `EditNote` variants to `NlpAction` enum in `src/nlp.rs`
- [x] 6.2 Update NLP response parsing to handle `create_note` and `edit_note` actions
- [x] 6.3 Implement `CreateNote` handler: create note file, optionally link to task, display confirmation
- [x] 6.4 Implement `EditNote` handler: update note body, display confirmation or error
- [x] 6.5 Update NLP system prompt with note action instructions, examples, and note slug context
- [x] 6.6 Add existing note slugs to the NLP task context
