## ADDED Requirements

### Requirement: Session launch keybinding in Normal mode
The TUI SHALL support pressing `C` (uppercase) while a task is selected in Normal mode to open the directory picker and initiate a Claude session with the selected task's content as context.

#### Scenario: Launch session from task list
- **WHEN** the user presses `C` with a task selected in Normal mode
- **THEN** the TUI SHALL enter Mode::SessionDirectoryPicker with the selected task's title and description queued as session context

#### Scenario: Footer hint for session launch
- **WHEN** the TUI is in Normal mode
- **THEN** the footer SHALL include `C:claude` in the keybinding hints

### Requirement: Session modes
The TUI SHALL support three new modes: `SessionDirectoryPicker` (directory selection modal), `Sessions` (session list panel), and `SessionReply` (inline text input for follow-up messages). These modes SHALL follow the mode state machine defined in the `claude-session` capability spec.

#### Scenario: Enter SessionDirectoryPicker
- **WHEN** the user presses `C` in Normal mode or the Notes view
- **THEN** the TUI mode SHALL become SessionDirectoryPicker

#### Scenario: Enter Sessions panel after launch
- **WHEN** a directory is confirmed in SessionDirectoryPicker
- **THEN** the TUI mode SHALL become Sessions and the new session SHALL appear in the panel

#### Scenario: Enter SessionReply
- **WHEN** the user presses `r` on a WaitingForInput session in the Sessions panel
- **THEN** the TUI mode SHALL become SessionReply with an empty input buffer

### Requirement: Reopen Sessions panel
The user SHALL press `C` while in Normal mode with no task selected, or navigate to the Sessions panel via a keybinding, to reopen the Sessions panel when sessions already exist. If no sessions exist, pressing `C` with no task selected SHALL open the directory picker as if a task were selected (with empty context).

#### Scenario: Reopen Sessions panel
- **WHEN** sessions exist and the user is in Normal mode and presses `C` with no task highlighted
- **THEN** the TUI SHALL enter Sessions mode showing the existing sessions
