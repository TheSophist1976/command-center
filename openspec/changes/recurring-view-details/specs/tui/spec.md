## MODIFIED Requirements

### Requirement: Task table display
The task table SHALL display columns for ID, status (checkbox), priority, title, and tags — matching the information shown by `task list`. The currently selected row SHALL be visually highlighted. When at least one visible task has a recurrence set, a recurrence indicator column SHALL be displayed showing `↻` for recurring tasks and blank for non-recurring tasks, followed by a "Pattern" column showing the human-readable recurrence pattern (e.g., "Weekly", "Daily", "3rd Thu") for recurring tasks and blank for non-recurring tasks.

#### Scenario: Table with tasks
- **WHEN** the TUI loads a file containing tasks
- **THEN** the table SHALL display each task as a row with ID, a checkbox (checked for done), priority, title, and tags columns. Priority cells SHALL use colors from the `theme` module.

#### Scenario: Done task row styling
- **WHEN** a task with status Done is rendered in the table
- **THEN** all cells in that row SHALL use `theme::DONE_TEXT` foreground color, overriding priority coloring

#### Scenario: Overdue row styling
- **WHEN** an open task's due date is strictly before today's date (tasks due today are NOT overdue)
- **THEN** the entire row SHALL be rendered in `theme::OVERDUE` foreground color and the status column SHALL display `[!]` instead of `[ ]`

#### Scenario: Task due today is not overdue
- **WHEN** an open task has a due date equal to today
- **THEN** it SHALL be displayed with normal styling, not overdue styling

#### Scenario: Empty task file
- **WHEN** the TUI loads a file with no tasks
- **THEN** the table area SHALL display a message like "No tasks. Press 'a' to add one."

#### Scenario: Description column shown conditionally
- **WHEN** at least one visible task has a non-empty description
- **THEN** the table SHALL include a "Desc" column after the Title column, displaying each task's description truncated to 30 characters with "…" appended if truncated

#### Scenario: Description column hidden
- **WHEN** no visible tasks have a description
- **THEN** the "Desc" column SHALL be omitted to preserve table width

#### Scenario: Recurrence pattern column shown
- **WHEN** at least one visible task has a recurrence set
- **THEN** the table SHALL include a "Pattern" column after the "↻" column, displaying the recurrence pattern text (e.g., "Weekly", "Daily", "3rd Thu") for recurring tasks and blank for non-recurring tasks

#### Scenario: Recurrence columns hidden
- **WHEN** no visible tasks have a recurrence
- **THEN** both the "↻" column and the "Pattern" column SHALL be omitted
