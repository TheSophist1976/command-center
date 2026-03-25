## ADDED Requirements

### Requirement: ANSI escape code stripping
The system SHALL strip ANSI escape sequences from output lines before appending them to the session ring buffer. The stripper SHALL remove all sequences matching the pattern ESC `[` followed by any number of parameter bytes and a final command byte (covering SGR color/style codes and other common terminal sequences). Lines that consist entirely of whitespace after stripping SHALL still be stored but SHALL be excluded from the "last output" preview in the sessions list.

#### Scenario: ANSI color codes stripped from stored lines
- **WHEN** a subprocess emits a line containing ANSI SGR codes such as `\x1b[32mHello\x1b[0m`
- **THEN** the stored line SHALL be `Hello` with all escape sequences removed

#### Scenario: Non-ANSI content preserved
- **WHEN** a subprocess emits a plain text line with no escape sequences
- **THEN** the stored line SHALL be identical to the emitted line

#### Scenario: Blank-after-strip lines excluded from preview
- **WHEN** a session's last several stored lines are empty or whitespace-only
- **THEN** the sessions list "last output" cell SHALL show the most recent non-blank line instead

### Requirement: Session output follow mode
The session output detail view SHALL maintain a follow mode that automatically scrolls to the bottom when new output lines arrive while the session is Running. Follow mode SHALL be active by default when the user opens the detail view. The user SHALL be able to exit follow mode by manually scrolling up (`j`, `Up`, or `PgUp`). The user SHALL re-enable follow mode by pressing `End` or `G`.

#### Scenario: Auto-scroll while Running
- **WHEN** the detail view is open, follow mode is active, and a new line is appended to the session output
- **THEN** `session_output_scroll` SHALL be updated to show the latest line without user action

#### Scenario: Manual scroll disables follow mode
- **WHEN** the user presses `j` or `Up` in the detail view
- **THEN** follow mode SHALL be disabled and `session_output_scroll` SHALL not change automatically on new output

#### Scenario: End key re-enables follow mode
- **WHEN** the user presses `End` or `G` in the detail view
- **THEN** `session_output_scroll` SHALL jump to the bottom and follow mode SHALL be re-enabled

## MODIFIED Requirements

### Requirement: Session output detail
When the user presses `Enter` on a session in the Sessions panel, the TUI SHALL display the full accumulated output for that session in a scrollable view. The output SHALL be stored as a ring buffer of the last 500 lines. Output lines SHALL be rendered with markdown styling (headings, bold, italic, inline code, code blocks, blockquotes) using the existing `style_markdown_line` function. Turn-boundary sentinel lines SHALL be styled with a dimmed accent color and SHALL NOT be processed as markdown. The view SHALL support `Home`/`g` to jump to the top and `End`/`G` to jump to the bottom.

#### Scenario: View session output
- **WHEN** the user presses `Enter` on a session in the Sessions panel
- **THEN** the full accumulated output for that session SHALL be displayed in a scrollable panel

#### Scenario: Output ring buffer
- **WHEN** a session produces more than 500 lines of output
- **THEN** the oldest lines SHALL be dropped and only the most recent 500 lines SHALL be retained

#### Scenario: Markdown rendered in output detail
- **WHEN** the output detail view displays a line starting with `## `
- **THEN** the line SHALL be rendered with bold markdown heading styling

#### Scenario: Code block rendered in output detail
- **WHEN** the output detail contains a fenced code block delimited by ` ``` `
- **THEN** the lines inside the fence SHALL be styled with code color and SHALL NOT have inline markdown parsing applied

#### Scenario: Home key jumps to top
- **WHEN** the user presses `Home` or `g` in the output detail view
- **THEN** `session_output_scroll` SHALL be set to 0 and follow mode SHALL be disabled

#### Scenario: Turn separator styled distinctly
- **WHEN** the output contains a turn-boundary sentinel line
- **THEN** the sentinel SHALL be rendered with dimmed/accent color and SHALL NOT be processed as markdown

### Requirement: Sessions panel
The TUI SHALL provide a Sessions panel (Mode::Sessions) displaying all active Claude sessions as a scrollable list. Each entry SHALL show the session label (task/note title), working directory name, status (Running / Waiting / Done), and the last non-blank line of output. The user SHALL navigate sessions with `j`/`k`, open a session for detailed output with `Enter`, and return to Normal mode with `Esc` or `q`.

#### Scenario: Sessions panel shows active sessions
- **WHEN** one or more sessions have been launched
- **THEN** the Sessions panel SHALL display each session with its label, directory, status, and last non-blank output line

#### Scenario: Running session shows spinner
- **WHEN** a session has status Running (subprocess in-flight)
- **THEN** the Sessions panel SHALL display an animated indicator alongside that entry

#### Scenario: Navigate sessions
- **WHEN** the user presses `j` or `k` in the Sessions panel
- **THEN** the selection SHALL move down or up through the session list

#### Scenario: Close sessions panel
- **WHEN** the user presses `Esc` or `q` in the Sessions panel
- **THEN** the TUI SHALL return to Normal mode; active sessions SHALL continue running in the background

### Requirement: Inline reply
When a session status is WaitingForInput (the subprocess has exited cleanly), the user SHALL press `r` in the Sessions panel to enter SessionReply mode and type a follow-up message. On `Enter`, the TUI SHALL inject a turn-boundary sentinel line into the session output buffer, then spawn a new `claude --print --continue -p "<message>"` subprocess in the same working directory, transitioning the session back to Running status. On `Esc`, the reply SHALL be cancelled.

#### Scenario: Send follow-up message
- **WHEN** a session is in WaitingForInput status and the user presses `r`, types a message, and presses `Enter`
- **THEN** a turn-boundary sentinel line SHALL be appended to the session output, a new `claude --print --continue -p "<message>"` subprocess SHALL be spawned in the session's working directory, and the session status SHALL become Running

#### Scenario: Cancel reply
- **WHEN** the user presses `Esc` in SessionReply mode
- **THEN** the input SHALL be discarded and the session SHALL remain in WaitingForInput status
