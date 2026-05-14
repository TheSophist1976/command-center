## Why

The built-in TUI note editor is minimal and awkward for anything longer than a quick note. Engineers already have preferred editors configured via `$EDITOR`. Obsidian users want notes to live in their vault and open natively. Glow provides a polished read-only markdown preview. Removing the built-in editor in favor of proper external tools gives a significantly better note-editing experience and eliminates a maintenance burden.

## What Changes

- **Remove the built-in inline TUI note editor** (`NoteEditor`, `Mode::EditingNote`, `Mode::ConfirmingNoteExit`, related draw functions and key handlers)
- **External editor via `$EDITOR`**: when a note is opened for editing, the TUI suspends, spawns `$EDITOR <note-path>`, and resumes when the editor exits
- **Glow viewer**: if `note-viewer: glow` is set in config, pressing the view key launches `glow <note-path>` (read-only); editing still requires `$EDITOR`
- **Obsidian**: if `obsidian-vault: <name>` is set in config, opening a note launches the `obsidian://open?vault=<name>&file=<relative-path>` URI instead of `$EDITOR`
- **Error when unconfigured**: if `$EDITOR` is unset and no Obsidian config is present, show a clear error message rather than silently failing
- **Config keys**: `note-viewer` (e.g. `glow`) and `obsidian-vault` + `obsidian-notes-dir` (path to vault notes directory)
- **`g` and `Enter` (Notes view)** key behavior updated to use external open flow

## Capabilities

### New Capabilities
- `external-note-editor`: external editor launch via `$EDITOR`, Obsidian URI, and glow viewer; config keys; error behavior

### Modified Capabilities
- `note-tui`: remove inline editor; update `g`-key and Notes-view `Enter` to use external open flow; `a` (new note) creates the file then opens it externally
- `app-config`: new `note-viewer`, `obsidian-vault`, and `obsidian-notes-dir` config keys

## Impact

- `src/tui.rs` — remove `NoteEditor`, `Mode::EditingNote`, `Mode::ConfirmingNoteExit`, `draw_note_editor`, `handle_note_edit`, `handle_note_confirm`; update `g` key and Notes view open flow
- `src/bin/task_tui.rs` — add external open helper; update note open calls
- `src/config.rs` — no code change (config keys read via existing `read_config_value`)
- `AGENTS.md` — no change needed
