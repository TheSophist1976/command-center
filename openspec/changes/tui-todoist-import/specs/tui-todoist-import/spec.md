## ADDED Requirements

### Requirement: Import Todoist tasks from TUI
The user SHALL press `i` in normal mode to trigger a Todoist import. The system SHALL check for a stored Todoist API token via `auth::read_token()`. If a token is present, the system SHALL call `todoist::run_import(token, &mut task_file, false)` to fetch and import tasks. After a successful import, the system SHALL save the task file to disk and update the displayed task list. The `i` key SHALL be a no-op when the TUI is in any non-Normal mode.

#### Scenario: Successful import
- **WHEN** the user presses `i` in normal mode and a valid Todoist token is stored
- **THEN** the system SHALL call `todoist::run_import`, save the updated task file, clamp the selection, and display a status message like "Imported 5 tasks, skipped 2 (already exported)"

#### Scenario: No token stored
- **WHEN** the user presses `i` in normal mode and no Todoist token is stored
- **THEN** the system SHALL display a status message "No Todoist token. Run `task auth todoist` from the CLI." and SHALL NOT make any API calls

#### Scenario: API error during import
- **WHEN** the user presses `i` and `todoist::run_import` returns an error (e.g., 401 unauthorized, network failure)
- **THEN** the system SHALL display the error as a status message in the footer and SHALL NOT modify the task file

#### Scenario: Zero tasks imported
- **WHEN** the user presses `i` and all Todoist tasks are already labeled as exported
- **THEN** the system SHALL display a status message like "Imported 0 tasks, skipped 10 (already exported)"

### Requirement: Status message display
The `App` struct SHALL include a `status_message: Option<String>` field. When set, the footer SHALL display the status message text instead of the default keybinding hints during Normal mode. The status message SHALL be cleared on the next keypress in Normal mode.

#### Scenario: Status message shown after import
- **WHEN** a Todoist import completes (success or error) and the TUI renders
- **THEN** the footer SHALL display the status message text

#### Scenario: Status message cleared on next keypress
- **WHEN** a status message is displayed and the user presses any key in Normal mode
- **THEN** the status message SHALL be cleared and the footer SHALL return to showing keybinding hints

#### Scenario: No status message by default
- **WHEN** the TUI launches and no import has been performed
- **THEN** `status_message` SHALL be `None` and the footer SHALL show the default keybinding hints
