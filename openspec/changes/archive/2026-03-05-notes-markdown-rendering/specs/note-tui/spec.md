## MODIFIED Requirements

### Requirement: Inline multi-line note editor
The TUI SHALL provide an inline multi-line text editor for editing note content. The editor SHALL support: character insertion at cursor, Enter for new line, Backspace to delete character before cursor, arrow keys for cursor movement (up/down/left/right), and viewport scrolling when content exceeds the visible area. The editor SHALL display line numbers. The editor SHALL render markdown syntax with visual styling (headings, bold, italic, inline code, code blocks, blockquotes, list markers) using colored and styled text spans. The raw markdown text SHALL remain visible — styling is additive, not replacing syntax characters.

#### Scenario: Type text with newlines
- **WHEN** the user types "Hello" then presses Enter then types "World"
- **THEN** the editor buffer SHALL contain two lines: "Hello" and "World"

#### Scenario: Navigate with arrow keys
- **WHEN** the cursor is at line 3, column 5 and the user presses Up
- **THEN** the cursor SHALL move to line 2, clamping column to the length of line 2 if shorter

#### Scenario: Scroll viewport
- **WHEN** the note content has 50 lines and the visible area shows 20 lines, and the cursor moves to line 25
- **THEN** the viewport SHALL scroll to keep the cursor visible

#### Scenario: Markdown heading is styled
- **WHEN** a line in the editor starts with "# "
- **THEN** the line SHALL be rendered with bold and colored styling

#### Scenario: Code block is styled
- **WHEN** lines are enclosed between ``` fences
- **THEN** the enclosed lines SHALL be rendered with code styling and no inline markdown parsing

#### Scenario: Inline bold is styled
- **WHEN** a line contains text enclosed in ** markers
- **THEN** the enclosed text and markers SHALL be rendered with bold styling
