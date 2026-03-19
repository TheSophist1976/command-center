## ADDED Requirements

### Requirement: Notes view
The TUI SHALL include a Notes view accessible through the view cycle (`v`/`V` keybinding). The Notes view SHALL display a list of all discovered notes showing the note title and last-modified date. The view SHALL refresh the notes list each time it is entered.

#### Scenario: Enter Notes view
- **WHEN** the user cycles to the Notes view
- **THEN** the TUI SHALL display a list of all note files in the task directory with their titles and modification dates

#### Scenario: Empty notes list
- **WHEN** the Notes view is active and no note files exist
- **THEN** the TUI SHALL display a message indicating no notes exist and hint how to create one

### Requirement: Create note from Notes view
The user SHALL press `a` in the Notes view to create a new note. The system SHALL prompt for a title, then open the inline note editor with an empty body. On save, the note file SHALL be created in the task directory.

#### Scenario: Create new note
- **WHEN** the user presses `a` in the Notes view and enters title "Sprint Retro"
- **THEN** the inline editor SHALL open with an empty body, and on save, `sprint-retro.md` SHALL be created

### Requirement: Edit note from Notes view
The user SHALL press `Enter` on a selected note in the Notes view to open it in the inline editor. The editor SHALL load the note's current body content for editing.

#### Scenario: Open existing note for editing
- **WHEN** the user selects a note and presses `Enter`
- **THEN** the inline editor SHALL open with the note's current body content loaded

### Requirement: Delete note from Notes view
The user SHALL press `d` on a selected note in the Notes view to delete it. The system SHALL prompt for confirmation before deleting the file. If the note is linked to any task, the task's `note` field SHALL be cleared.

#### Scenario: Delete note with confirmation
- **WHEN** the user presses `d` on a note and confirms deletion
- **THEN** the note file SHALL be deleted from the filesystem

#### Scenario: Delete linked note clears task reference
- **WHEN** a deleted note was linked to a task via the `note` field
- **THEN** the task's `note` field SHALL be set to `None`

### Requirement: Inline multi-line note editor
The TUI SHALL provide an inline multi-line text editor for editing note content. The editor SHALL support: character insertion at cursor, Enter for new line, Backspace to delete character before cursor, arrow keys for cursor movement (up/down/left/right), `Home` to move the cursor to column 0 of the current logical line, `End` to move the cursor to the last character of the current logical line, and viewport scrolling when content exceeds the visible area. The editor SHALL display line numbers. Long logical lines SHALL be rendered using visual word wrap across multiple display rows rather than being truncated at the terminal width; cursor placement and viewport scrolling SHALL account for wrapped visual rows. The editor SHALL render markdown syntax with visual styling (headings, bold, italic, inline code, code blocks, blockquotes, list markers) using colored and styled text spans. The raw markdown text SHALL remain visible — styling is additive, not replacing syntax characters.

#### Scenario: Type text with newlines
- **WHEN** the user types "Hello" then presses Enter then types "World"
- **THEN** the editor buffer SHALL contain two lines: "Hello" and "World"

#### Scenario: Navigate with arrow keys
- **WHEN** the cursor is at line 3, column 5 and the user presses Up
- **THEN** the cursor SHALL move to line 2, clamping column to the length of line 2 if shorter

#### Scenario: Home moves cursor to line start
- **WHEN** the cursor is at any column on a logical line and the user presses `Home`
- **THEN** the cursor SHALL move to column 0 of that logical line

#### Scenario: End moves cursor to line end
- **WHEN** the cursor is on a logical line and the user presses `End`
- **THEN** the cursor SHALL move to the last character position of that logical line

#### Scenario: Long line wraps visually
- **WHEN** a logical line exceeds the terminal text width
- **THEN** the line SHALL be rendered across multiple display rows rather than truncated, with the line number shown only on the first display row

#### Scenario: Cursor position accounts for wrapped rows
- **WHEN** the cursor is at a column beyond the terminal text width on a wrapped line
- **THEN** the cursor SHALL appear on the correct visual row and column within the wrapped display

#### Scenario: Scroll viewport
- **WHEN** the note content has 50 lines and the visible area shows 20 lines, and the cursor moves to line 25
- **THEN** the viewport SHALL scroll to keep the cursor visible, counting visual (wrapped) rows

#### Scenario: Markdown heading is styled
- **WHEN** a line in the editor starts with "# "
- **THEN** the line SHALL be rendered with bold and colored styling

#### Scenario: Code block is styled
- **WHEN** lines are enclosed between ``` fences
- **THEN** the enclosed lines SHALL be rendered with code styling and no inline markdown parsing

#### Scenario: Inline bold is styled
- **WHEN** a line contains text enclosed in ** markers
- **THEN** the enclosed text and markers SHALL be rendered with bold styling

### Requirement: Save and exit note editor
The user SHALL press `Ctrl+S` to save the note and remain in the editor, or `Escape` to exit. If the content has unsaved changes when pressing Escape, the system SHALL prompt for confirmation (save/discard/cancel).

#### Scenario: Save with Ctrl+S
- **WHEN** the user presses Ctrl+S in the note editor
- **THEN** the note file SHALL be written to disk and the user SHALL remain in the editor

#### Scenario: Exit with unsaved changes
- **WHEN** the user presses Escape with unsaved changes
- **THEN** the system SHALL prompt: save changes, discard changes, or cancel (return to editor)

#### Scenario: Exit with no changes
- **WHEN** the user presses Escape with no unsaved changes
- **THEN** the editor SHALL close and return to the Notes view

### Requirement: Notes view in view cycle
The Notes view SHALL be included in the view cycle between Recurring and Today: Today → All → Weekly → Monthly → Yearly → NoDueDate → Recurring → Notes → Today.

#### Scenario: Cycle to Notes view
- **WHEN** the active view is Recurring and the user presses `v`
- **THEN** the active view SHALL change to Notes

#### Scenario: Cycle from Notes view
- **WHEN** the active view is Notes and the user presses `v`
- **THEN** the active view SHALL change to Today

### Requirement: Notes view header and footer
The Notes view SHALL display "Notes" in the header. The footer SHALL show keybinding hints relevant to the Notes view: `a:new  Enter:edit  d:delete  v:view  q:quit`.

#### Scenario: Footer hints in Notes view
- **WHEN** the Notes view is active
- **THEN** the footer SHALL display notes-specific keybinding hints

## ADDED Requirements

### Requirement: Session launch keybinding in Notes view
The user SHALL press `C` in the Notes view (with a note selected) to open the directory picker and initiate a Claude session with the selected note's title and body as context. The footer in the Notes view SHALL include `C:claude` in its keybinding hints.

#### Scenario: Launch session from Notes view
- **WHEN** the user presses `C` with a note selected in the Notes view
- **THEN** the TUI SHALL enter Mode::SessionDirectoryPicker with the selected note's title and body queued as session context

#### Scenario: Footer hint in Notes view
- **WHEN** the Notes view is active
- **THEN** the footer SHALL display `a:new  Enter:edit  d:delete  v:view  C:claude  q:quit`
