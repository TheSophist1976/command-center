## ADDED Requirements

### Requirement: Task detail panel
The TUI SHALL provide a toggleable bottom panel that displays all fields of the currently selected task. The panel SHALL be toggled by pressing `Tab` in Normal mode. When visible, the layout SHALL split into a task table (top ~70%) and detail panel (bottom ~30%). The panel SHALL display: ID, title, status, priority, description (full text, wrapped), tags, due date, project, created timestamp, and updated timestamp. The panel content SHALL update as the user navigates between tasks.

#### Scenario: Toggle detail panel on
- **WHEN** the user presses `Tab` in Normal mode with the detail panel hidden
- **THEN** the layout SHALL split to show the task table above and the detail panel below, displaying all fields of the currently selected task

#### Scenario: Toggle detail panel off
- **WHEN** the user presses `Tab` in Normal mode with the detail panel visible
- **THEN** the detail panel SHALL be hidden and the table SHALL expand to fill the available space

#### Scenario: Panel updates on navigation
- **WHEN** the detail panel is visible and the user presses `j` or `k` to navigate
- **THEN** the panel SHALL update to show the details of the newly selected task

#### Scenario: Panel with no task selected
- **WHEN** the detail panel is visible and no task is selected (empty or fully filtered list)
- **THEN** the panel SHALL display "No task selected."

#### Scenario: Panel shows full description
- **WHEN** the detail panel is visible and the selected task has a description
- **THEN** the panel SHALL display the full description text (not truncated)

## MODIFIED Requirements

### Requirement: Task table display
The task table SHALL display columns for ID, status (checkbox), priority, title, and tags — matching the information shown by `task list`. The currently selected row SHALL be visually highlighted.

#### Scenario: Table with tasks
- **WHEN** the TUI loads a file containing tasks
- **THEN** the table SHALL display each task as a row with ID, a checkbox (checked for done), priority, title, and tags columns

#### Scenario: Empty task file
- **WHEN** the TUI loads a file with no tasks
- **THEN** the table area SHALL display a message like "No tasks. Press 'a' to add one."

#### Scenario: Description column shown conditionally
- **WHEN** at least one visible task has a non-empty description
- **THEN** the table SHALL include a "Desc" column after the Title column, displaying each task's description truncated to 30 characters with "…" appended if truncated

#### Scenario: Description column hidden
- **WHEN** no visible tasks have a description
- **THEN** the "Desc" column SHALL be omitted to preserve table width

#### Scenario: Description truncation
- **WHEN** a task's description exceeds 30 characters
- **THEN** the Desc cell SHALL display the first 29 characters followed by "…"

#### Scenario: Short description displayed in full
- **WHEN** a task's description is 30 characters or fewer
- **THEN** the Desc cell SHALL display the full description without truncation
