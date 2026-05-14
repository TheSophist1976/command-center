## 1. External Open Helper

- [x] 1.1 Add `fn open_note_external(note_path: &Path, slug: &str) -> Result<(), String>`: Obsidian URI via `open` if configured, else `$EDITOR` with TUI suspend/resume, else error
- [x] 1.2 Add `fn view_note_glow(note_path: &Path, slug: &str) -> Result<(), String>`: `glow` with TUI suspend/resume if `note-viewer: glow` configured; falls through to `open_note_external`
- [x] 1.3 Add `fn build_obsidian_uri(slug: &str) -> Option<String>`: reads `obsidian-vault` and `obsidian-notes-dir` from config, builds URI

## 2. Remove Built-in Note Editor

- [x] 2.1 Remove `NoteEditor` struct and `impl NoteEditor` from `src/tui.rs`
- [x] 2.2 Remove `Mode::EditingNote` and `Mode::ConfirmingNoteExit` variants from the `Mode` enum
- [x] 2.3 Remove `handle_note_editor()` and `handle_note_exit_confirm()` and `save_current_note()` functions
- [x] 2.4 Remove `draw_note_editor()` function
- [x] 2.5 Remove `note_editor: Option<NoteEditor>` field from `App` struct and all initialization sites
- [x] 2.6 Remove the `Mode::EditingNote` and `Mode::ConfirmingNoteExit` arms from the mode dispatch and footer text match blocks

## 3. Update `g` Key (task → note)

- [x] 3.1 Replace the `g`-key handler's note-opening logic with a call to `open_note_external`
- [x] 3.2 Show the error status message in the TUI if `open_note_external` returns `Err`

## 4. Update Notes View (Enter and `a`)

- [x] 4.1 Replace Notes view `Enter` handler's note-opening call with `open_note_external`
- [x] 4.2 Update the Notes view `a` (new note) handler: after creating the file, call `open_note_external` on the new note path instead of opening the inline editor
- [x] 4.3 After returning from external editor (Enter and `a` flows), call `app.refresh_notes()` to pick up any changes

## 5. Add `V` Key for Glow View

- [x] 5.1 Add `KeyCode::Char('V')` handler in Notes view: call `view_note_glow` on the selected note
- [x] 5.2 Update footer hint strings to include `V:view-note` alongside `g:edit-note`

## 6. TUI Cleanup

- [x] 6.1 Remove NoteEditor tests and `char_to_byte_index` tests from test module
- [x] 6.2 Verified `cargo build` and `cargo test` pass — 211 tests pass
