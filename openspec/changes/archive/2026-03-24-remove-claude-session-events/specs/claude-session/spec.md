## REMOVED Requirements

### Requirement: Structured session output events
**Reason**: The typed `SessionOutputEvent` enum adds significant complexity (event parsing, variant matching, ring-buffer of heterogeneous entries) without sufficient benefit. Reverting to `Vec<String>` simplifies `claude_session.rs` and `tui.rs` and removes a hard-to-test layer.
**Migration**: Session output is stored as `Vec<String>`. All code that matches on `SessionOutputEvent` variants should be replaced with direct string iteration.

### Requirement: Collapsible tool calls
**Reason**: Depends on `ToolCall` variant of `SessionOutputEvent`. Removed along with the event model.
**Migration**: Tool call output will appear as plain text lines in the output detail view.

### Requirement: Permission request modal
**Reason**: Depends on `PermissionRequest` variant of `SessionOutputEvent` and `Mode::PermissionModal`. Removed along with the event model.
**Migration**: Permission requests from the subprocess are no longer intercepted by the TUI; they appear as plain text output lines.

### Requirement: Session ID capture and resume
**Reason**: Depends on `session_id: Option<String>` on `ClaudeSession` and the `SessionIdCaptured` stream event. Removed along with the event model.
**Migration**: Follow-up messages always use `--continue`. The `--resume` CLI flag is removed.

### Requirement: Stdin piped for permission responses
**Reason**: Piped stdin was only needed for writing permission responses (`y\n` / `n\n`). With the permission modal removed there is no reason to hold a `ChildStdin` handle.
**Migration**: Both `launch_claude_session` and `continue_claude_session` revert to `Stdio::null()` for stdin.

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
