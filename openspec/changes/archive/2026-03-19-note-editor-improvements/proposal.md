## Why

The inline note editor is missing `Home`/`End` key support, making it awkward to navigate to the start or end of a line. Long lines are also silently truncated in the display, so content extending past the terminal width is invisible during editing.

## What Changes

- Add `Home` key handling to the note editor: moves cursor to column 0 on the current line
- Add `End` key handling to the note editor: moves cursor to the last character of the current line
- Replace line truncation in the note editor renderer with visual word wrap: long logical lines are rendered across multiple display rows, and cursor placement accounts for the wrapped row offset

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `note-tui`: The inline note editor gains Home/End key navigation and visual word wrap in the renderer. Both are requirement-level changes to the editor's key handling and rendering behavior.

## Impact

- `src/tui.rs`:
  - `handle_note_editor`: add `KeyCode::Home` and `KeyCode::End` arms
  - `NoteEditor`: add `move_to_line_start` and `move_to_line_end` methods
  - `draw_note_editor`: replace truncation logic with word-wrap rendering; adjust cursor screen-position calculation to account for wrapped visual rows
  - `ensure_cursor_visible`: update to count visual rows (logical line may span multiple rows) so scrolling stays correct under word wrap
- No changes to `note.rs`, `storage.rs`, or the CLI
