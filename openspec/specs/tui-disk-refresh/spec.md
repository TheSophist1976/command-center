## ADDED Requirements

### Requirement: Manual reload from disk
The TUI SHALL support a `ctrl+r` keybinding in Normal mode that reloads `tasks.md` from disk and replaces the in-memory task list. If the TUI is in any mode other than Normal when `ctrl+r` is pressed, the reload SHALL be blocked and a warning status message SHALL be displayed. On success, the task table SHALL reflect the newly loaded content and the cursor SHALL be clamped to the nearest valid position.

#### Scenario: Reload in Normal mode
- **WHEN** the user presses `ctrl+r` while the TUI is in Normal mode
- **THEN** the system SHALL call `storage::load` on the current file path, replace `app.task_file` with the result, call `app.clamp_selection()`, and set a status message of `"Reloaded N tasks from disk"` where N is the number of tasks loaded

#### Scenario: Reload blocked during edit
- **WHEN** the user presses `ctrl+r` while the TUI is in any non-Normal mode (e.g., `Adding`, `EditingTitle`, `EditingDescription`)
- **THEN** the system SHALL NOT reload from disk and SHALL display the status message `"Cannot reload: finish editing first"`

#### Scenario: Reload with load error
- **WHEN** the user presses `ctrl+r` in Normal mode and `storage::load` returns an error
- **THEN** the system SHALL NOT replace `app.task_file` and SHALL display the error message as the status message
