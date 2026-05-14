## MODIFIED Requirements

### Requirement: Edit note from Notes view
The user SHALL press `Enter` on a selected note in the Notes view to open it externally. The system SHALL use the external open flow (Obsidian URI if configured, else `$EDITOR`, else error message). The built-in inline editor SHALL NOT be used.

#### Scenario: Open existing note for editing
- **WHEN** the user selects a note and presses `Enter` in the Notes view
- **THEN** the external open flow SHALL be triggered for that note file

### Requirement: Create note from Notes view
The user SHALL press `a` in the Notes view to create a new note. The system SHALL prompt for a title, create the note file with a `# <title>` header, then immediately open it in the external editor using the external open flow.

#### Scenario: Create new note opens external editor
- **WHEN** the user presses `a` in the Notes view, enters a title, and the file is created
- **THEN** the external open flow SHALL be triggered for the new note file

### Requirement: Navigate to linked note via `g` key
The `g` key on a task with linked notes SHALL open the first linked note using the external open flow (not the inline editor).

#### Scenario: `g` key uses external editor
- **WHEN** the user presses `g` on a task with a linked note
- **THEN** the external open flow SHALL be triggered for that note file

### Requirement: View note with `V` key (glow)
The user SHALL press `V` (shift-v) on a selected note in the Notes view or on a task with a linked note to trigger the glow view action. If `note-viewer: glow` is not configured, the system SHALL fall through to the edit action.

#### Scenario: `V` launches glow when configured
- **WHEN** `note-viewer: glow` is in config and the user presses `V` on a note
- **THEN** the TUI SHALL suspend, `glow <note-path>` SHALL run, and the TUI SHALL resume

#### Scenario: `V` falls through to edit when glow not configured
- **WHEN** `note-viewer` is not set and the user presses `V`
- **THEN** the edit action SHALL be triggered instead

## REMOVED Requirements

### Requirement: Edit note from Notes view (inline editor)
**Reason**: Replaced by external editor open flow. The inline `NoteEditor` and associated modes (`EditingNote`, `ConfirmingNoteExit`) are removed entirely.
**Migration**: Set `$EDITOR` in your shell environment or configure `obsidian-vault` in config.

### Requirement: Inline note editor state
**Reason**: `NoteEditor` struct, `Mode::EditingNote`, `Mode::ConfirmingNoteExit`, `draw_note_editor`, `handle_note_edit`, `handle_note_confirm` are all removed.
**Migration**: Use external editor.
