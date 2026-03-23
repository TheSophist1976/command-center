## ADDED Requirements

### Requirement: Structured session output events
The system SHALL store Claude session output as `Vec<SessionOutputEvent>` where `SessionOutputEvent` is an enum with variants: `Text(String)`, `ToolCall { name, input_preview, result_lines, collapsed }`, `PermissionRequest { tool, input_preview }`, and `TurnSeparator`. The `ToolCall` variant SHALL be initialized with `collapsed: true` and `result_lines: Vec::new()`. When a `tool_result` event arrives in the stream, the result lines SHALL be appended to the most recent `ToolCall` event in the buffer rather than emitting a new event.

#### Scenario: Tool call and result stored as one unit
- **WHEN** the stream emits a `tool_use` event followed by a `tool_result` event
- **THEN** both SHALL be represented as a single `ToolCall` event in the session output buffer, with the result lines populated from the `tool_result` event

#### Scenario: Text lines stored as Text events
- **WHEN** the stream emits an assistant `text` content block
- **THEN** each line of the text SHALL be stored as a separate `Text(String)` event

### Requirement: Collapsible tool calls
Tool call events SHALL be rendered as a single collapsed line by default (`⚙  Name: input_preview [+]`). The user SHALL be able to toggle a tool call between collapsed and expanded states by pressing `Enter` or `Space` while the tool call line is focused in the output detail view. When expanded, the result lines SHALL be rendered below the header line in green. Navigation within the output detail view SHALL move the focus between visible lines; focusing a tool call header SHALL enable the toggle action.

#### Scenario: Tool call collapsed by default
- **WHEN** a session with a completed tool call is displayed in the output detail view
- **THEN** the tool call SHALL be rendered as a single amber line ending in `[+]` with the result lines hidden

#### Scenario: Expand tool call with Enter
- **WHEN** the user navigates focus to a collapsed tool call line and presses `Enter` or `Space`
- **THEN** the tool call SHALL expand to show result lines in green below the header, and the header SHALL change to `[-]`

#### Scenario: Collapse tool call with Enter
- **WHEN** a tool call is expanded and the user presses `Enter` or `Space` on its header
- **THEN** the tool call SHALL collapse back to a single line

### Requirement: Permission request modal
When a `permission_request` event arrives in the stream for the currently selected session, the TUI SHALL enter `Mode::PermissionModal` and display a centered overlay showing the tool name, input preview, and key hints `[y] Allow  [n] Deny  [a] Allow session`. The session output SHALL remain visible behind the modal. The user's keypress SHALL write the appropriate response (`y\n` or `n\n`) to the subprocess stdin and return the TUI to `Mode::Sessions`.

#### Scenario: Modal shown on permission request
- **WHEN** the stream emits a `permission_request` event for the selected session
- **THEN** the TUI SHALL display the permission modal overlay with tool name and input preview

#### Scenario: Allow permission
- **WHEN** the user presses `y` in the permission modal
- **THEN** `y\n` SHALL be written to the subprocess stdin and the modal SHALL close

#### Scenario: Deny permission
- **WHEN** the user presses `n` in the permission modal
- **THEN** `n\n` SHALL be written to the subprocess stdin and the modal SHALL close

#### Scenario: Permission modal for non-selected session
- **WHEN** a `permission_request` event arrives for a session that is not currently selected
- **THEN** the session status SHALL be updated to `WaitingForInput` and the sessions list SHALL indicate the session needs attention (e.g. status badge)

### Requirement: Session ID capture and resume
The system SHALL extract the `session_id` field from `result` stream events and store it on `ClaudeSession.session_id: Option<String>`. When `continue_claude_session` is called and a `session_id` is stored, the subprocess SHALL be spawned with `--resume <session_id>` instead of `--continue`.

#### Scenario: Session ID captured from stream
- **WHEN** the stream emits a `result` event containing a `session_id` field
- **THEN** `ClaudeSession.session_id` SHALL be set to that value

#### Scenario: Resume uses session ID when available
- **WHEN** the user sends a follow-up reply and `session.session_id` is `Some(id)`
- **THEN** the subprocess SHALL be spawned with `--resume <id>` and not `--continue`

#### Scenario: Resume falls back to --continue
- **WHEN** the user sends a follow-up reply and `session.session_id` is `None`
- **THEN** the subprocess SHALL be spawned with `--continue` as before

### Requirement: Stdin piped for permission responses
Both `launch_claude_session` and `continue_claude_session` SHALL spawn the claude subprocess with `Stdio::piped()` for stdin. The `ClaudeSession` struct SHALL hold `stdin: Option<ChildStdin>` (excluded from serialization). When the child process exits, `stdin` SHALL be dropped.

#### Scenario: Stdin available after launch
- **WHEN** a new Claude session is launched
- **THEN** `session.stdin` SHALL be `Some(ChildStdin)` and ready to receive input

## MODIFIED Requirements

### Requirement: Session output detail
When the user presses `Enter` on a session in the Sessions panel, the TUI SHALL display the full accumulated output for that session in a scrollable view. Output SHALL be stored as a ring buffer of up to 500 `SessionOutputEvent` entries. The detail view SHALL render each event type distinctly: `Text` events with markdown styling, `ToolCall` events as collapsible units, `PermissionRequest` events as yellow warning lines, and `TurnSeparator` events as dimmed separators. Word wrap SHALL be applied to `Text` event lines. Follow mode and `Home`/`End`/`g`/`G` navigation SHALL apply to rendered line positions.

#### Scenario: View session output
- **WHEN** the user presses `Enter` on a session in the Sessions panel
- **THEN** the full accumulated output SHALL be displayed with per-event-type styling

#### Scenario: Event ring buffer limit
- **WHEN** a session accumulates more than 500 `SessionOutputEvent` entries
- **THEN** the oldest events SHALL be dropped to keep the buffer at 500

#### Scenario: Markdown rendered in text events
- **WHEN** a `Text` event line starts with `## `
- **THEN** it SHALL be rendered with bold markdown heading styling
