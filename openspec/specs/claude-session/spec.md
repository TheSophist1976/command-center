## ADDED Requirements

### Requirement: Directory picker
When the user triggers a new Claude session, the TUI SHALL display a directory picker modal listing the immediate subdirectories of the configured `claude-code-dir` root (default `~/code`), sorted alphabetically. The user SHALL navigate with `j`/`k` or arrow keys, confirm with `Enter`, and cancel with `Esc`. If the root directory does not exist or contains no subdirectories, the picker SHALL display an informational message.

#### Scenario: Picker lists project directories
- **WHEN** the user presses `C` on a task or note and `~/code` exists with subdirectories
- **THEN** the picker SHALL display the immediate subdirectory names sorted alphabetically

#### Scenario: Picker uses configured root
- **WHEN** `claude-code-dir` is set to `/home/user/projects` in config
- **THEN** the picker SHALL list subdirectories of `/home/user/projects`

#### Scenario: Empty root directory
- **WHEN** the configured root exists but contains no subdirectories
- **THEN** the picker SHALL display "No projects found — set `claude-code-dir` in config"

#### Scenario: Cancel picker
- **WHEN** the user presses `Esc` in the directory picker
- **THEN** the picker SHALL close and the TUI SHALL return to Normal mode with no session started

### Requirement: Session launch
After a directory is selected, the TUI SHALL spawn a `claude --print -p "<context>"` subprocess in the selected working directory, where `<context>` is the task title and description (for task launches) or the note title and body (for note launches). If the `claude` binary is not found in PATH, the TUI SHALL display a status message and not enter Sessions mode.

#### Scenario: Launch from task
- **WHEN** the user selects a directory for a task with title "Fix auth bug" and description "JWT tokens expire too early"
- **THEN** the subprocess SHALL be spawned in the selected directory with initial context containing the task title and description

#### Scenario: Launch from note
- **WHEN** the user selects a directory for a note with title "API Design" and body content
- **THEN** the subprocess SHALL be spawned in the selected directory with initial context containing the note title and body

#### Scenario: claude binary not found
- **WHEN** the `claude` binary is not present in PATH
- **THEN** the TUI SHALL display "claude binary not found — install Claude Code to use sessions" and remain in Normal mode

### Requirement: Sessions panel
The TUI SHALL provide a Sessions panel (Mode::Sessions) displaying all active Claude sessions as a scrollable list. Each entry SHALL show the session label (task/note title), working directory name, status (Running / Waiting / Done), and the last line of output. The user SHALL navigate sessions with `j`/`k`, open a session for detailed output with `Enter`, and return to Normal mode with `Esc` or `q`.

#### Scenario: Sessions panel shows active sessions
- **WHEN** one or more sessions have been launched
- **THEN** the Sessions panel SHALL display each session with its label, directory, status, and last output line

#### Scenario: Running session shows spinner
- **WHEN** a session has status Running (subprocess in-flight)
- **THEN** the Sessions panel SHALL display an animated indicator alongside that entry

#### Scenario: Navigate sessions
- **WHEN** the user presses `j` or `k` in the Sessions panel
- **THEN** the selection SHALL move down or up through the session list

#### Scenario: Close sessions panel
- **WHEN** the user presses `Esc` or `q` in the Sessions panel
- **THEN** the TUI SHALL return to Normal mode; active sessions SHALL continue running in the background

### Requirement: Session output detail
When the user presses `Enter` on a session in the Sessions panel, the TUI SHALL display the full accumulated output for that session in a scrollable view. The output SHALL be stored as a ring buffer of the last 500 lines.

#### Scenario: View session output
- **WHEN** the user presses `Enter` on a session in the Sessions panel
- **THEN** the full accumulated output for that session SHALL be displayed in a scrollable panel

#### Scenario: Output ring buffer
- **WHEN** a session produces more than 500 lines of output
- **THEN** the oldest lines SHALL be dropped and only the most recent 500 lines SHALL be retained

### Requirement: Inline reply
When a session status is WaitingForInput (the subprocess has exited cleanly), the user SHALL press `r` in the Sessions panel to enter SessionReply mode and type a follow-up message. On `Enter`, the TUI SHALL spawn a new `claude --print --continue -p "<message>"` subprocess in the same working directory, transitioning the session back to Running status. On `Esc`, the reply SHALL be cancelled.

#### Scenario: Send follow-up message
- **WHEN** a session is in WaitingForInput status and the user presses `r`, types a message, and presses `Enter`
- **THEN** a new `claude --print --continue -p "<message>"` subprocess SHALL be spawned in the session's working directory and the session status SHALL become Running

#### Scenario: Cancel reply
- **WHEN** the user presses `Esc` in SessionReply mode
- **THEN** the input SHALL be discarded and the session SHALL remain in WaitingForInput status

### Requirement: Session persistence
The TUI SHALL persist session history to `<task-dir>/claude-sessions/` as JSON files, one per session. After each subprocess exits (status becomes WaitingForInput, Done, or Failed), the session SHALL be written to disk. On TUI startup, all session files in that directory SHALL be loaded and displayed in the Sessions panel with their prior status and output. The system SHALL retain only the most recent 30 session files by modification time, deleting older files on each save.

#### Scenario: Session written after turn completes
- **WHEN** a `claude --print` subprocess exits and the session transitions to WaitingForInput
- **THEN** the session SHALL be serialized to `<task-dir>/claude-sessions/<id>-<slug>.json`

#### Scenario: Sessions loaded on startup
- **WHEN** the TUI starts and `<task-dir>/claude-sessions/` contains session files
- **THEN** those sessions SHALL be loaded into `App.claude_sessions` with their persisted status and output

#### Scenario: Loaded sessions are never Running
- **WHEN** a session file is loaded from disk
- **THEN** its status SHALL be set to its persisted value (WaitingForInput, Done, or Failed), never Running

#### Scenario: Retention limit enforced
- **WHEN** saving a session would result in more than 30 session files in the directory
- **THEN** the oldest file(s) by modification time SHALL be deleted to bring the count to 30

### Requirement: Session lifecycle and cleanup
Sessions SHALL transition through statuses: Running (subprocess in-flight) → WaitingForInput (subprocess exited with code 0) or Failed (non-zero exit). On TUI exit, all active Running sessions SHALL have their subprocess killed. Done sessions SHALL remain visible in the Sessions panel until the TUI exits.

#### Scenario: Session completes successfully
- **WHEN** a `claude --print` subprocess exits with code 0
- **THEN** the session status SHALL transition to WaitingForInput

#### Scenario: Session fails
- **WHEN** a `claude --print` subprocess exits with non-zero code or cannot be spawned
- **THEN** the session status SHALL transition to Failed and the last stderr line SHALL be shown

#### Scenario: Cleanup on TUI exit
- **WHEN** the user quits the TUI and one or more sessions have status Running
- **THEN** each Running session's subprocess SHALL be killed before the TUI exits
