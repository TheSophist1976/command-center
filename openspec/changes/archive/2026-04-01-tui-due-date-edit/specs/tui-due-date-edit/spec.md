## ADDED Requirements

### Requirement: Inline due date editing
The user SHALL press `d` in Normal mode to enter `EditingDue` mode for the selected task. The footer SHALL display a text input prompt pre-filled with the task's current due date in `YYYY-MM-DD` format, or an empty string if the task has no due date. Pressing `Enter` SHALL validate and save the input. Pressing `Esc` SHALL cancel without saving and return to Normal mode.

#### Scenario: Open editor pre-filled with existing date
- **WHEN** the user presses `d` on a task that has a due date of `2026-04-15`
- **THEN** the TUI SHALL enter `EditingDue` mode and the input buffer SHALL be pre-filled with `"2026-04-15"`

#### Scenario: Open editor empty for task with no due date
- **WHEN** the user presses `d` on a task that has no due date
- **THEN** the TUI SHALL enter `EditingDue` mode and the input buffer SHALL be empty

#### Scenario: Save a valid date
- **WHEN** the user is in `EditingDue` mode, types `2026-05-01`, and presses `Enter`
- **THEN** the selected task's `due_date` SHALL be set to `2026-05-01`, `updated` SHALL be set to the current UTC time, the file SHALL be saved, and the TUI SHALL return to Normal mode

#### Scenario: Clear the due date with empty input
- **WHEN** the user is in `EditingDue` mode, clears the input buffer, and presses `Enter`
- **THEN** the selected task's `due_date` SHALL be set to `None`, `updated` SHALL be set to the current UTC time, the file SHALL be saved, and the TUI SHALL return to Normal mode

#### Scenario: Reject invalid date input
- **WHEN** the user is in `EditingDue` mode and presses `Enter` with input that is not a valid `YYYY-MM-DD` date (e.g., `"notadate"`, `"2026-13-01"`)
- **THEN** the TUI SHALL remain in `EditingDue` mode and `app.status_message` SHALL be set to `"Invalid date — use YYYY-MM-DD"`

#### Scenario: Cancel editing
- **WHEN** the user is in `EditingDue` mode and presses `Esc`
- **THEN** the TUI SHALL return to Normal mode without modifying the task or saving the file

#### Scenario: No task selected
- **WHEN** the user presses `d` in Normal mode and the task list is empty
- **THEN** the TUI SHALL remain in Normal mode and take no action

## REMOVED Requirements

### Requirement: Delete task with confirmation
**Reason**: `d` key is repurposed for inline due date editing. Deletion is available via `task delete <id>` in the CLI.
**Migration**: Use `task delete <id>` from the terminal to delete tasks.
