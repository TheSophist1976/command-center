## ADDED Requirements

### Requirement: Display due date and project in task table
The TUI task table SHALL display `due_date` and `project` columns when at least one visible task has those fields set. Tasks with `None` values SHALL display empty cells in those columns.

#### Scenario: Table shows due date column
- **WHEN** at least one task in the visible list has a `due_date`
- **THEN** the table SHALL include a `Due` column showing the date in `YYYY-MM-DD` format

#### Scenario: Table shows project column
- **WHEN** at least one task in the visible list has a `project`
- **THEN** the table SHALL include a `Project` column showing the project name

#### Scenario: No due dates or projects
- **WHEN** no visible tasks have `due_date` or `project` set
- **THEN** the `Due` and `Project` columns SHALL be omitted to preserve table width

### Requirement: Display due date and project in task show detail
The TUI SHALL display `due_date` and `project` in the full task detail view (or the equivalent details visible in the table or status line) when those fields are set.

#### Scenario: Due date visible in detail
- **WHEN** the selected task has a `due_date`
- **THEN** the due date SHALL be shown alongside other task metadata

#### Scenario: Project visible in detail
- **WHEN** the selected task has a `project`
- **THEN** the project name SHALL be shown alongside other task metadata

### Requirement: Filter tasks by project in TUI
The TUI filter mode SHALL support a `project:<name>` filter expression that shows only tasks whose `project` field matches the given name (case-insensitive).

#### Scenario: Filter by project
- **WHEN** the user presses `f`, types `project:Work`, and presses `Enter`
- **THEN** the task table SHALL show only tasks with `project` equal to `"Work"` (case-insensitive)

#### Scenario: No matching project
- **WHEN** the user applies a `project:` filter that matches no tasks
- **THEN** the table area SHALL display "No tasks match filter."

## MODIFIED Requirements

### Requirement: Edit task priority
The user SHALL press `p` in normal mode to enter priority-editing mode for the selected task. The footer SHALL display a picker prompt showing all four available priorities. The user SHALL press `c`, `h`, `m`, or `l` to set the priority to critical, high, medium, or low respectively. The change SHALL be persisted to disk immediately. Pressing `Esc` or any other key SHALL cancel without changing the task. The `p` key SHALL be a no-op when no task is selected.

#### Scenario: Set priority to critical
- **WHEN** the user presses `p` on a selected task and then presses `c`
- **THEN** the task's priority SHALL be set to critical, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to high
- **WHEN** the user presses `p` on a selected task and then presses `h`
- **THEN** the task's priority SHALL be set to high, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to medium
- **WHEN** the user presses `p` on a selected task and then presses `m`
- **THEN** the task's priority SHALL be set to medium, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to low
- **WHEN** the user presses `p` on a selected task and then presses `l`
- **THEN** the task's priority SHALL be set to low, the display SHALL update, and the file SHALL be saved

#### Scenario: Cancel priority edit
- **WHEN** the user presses `p` and then presses `Esc` or any key other than `c`, `h`, `m`, or `l`
- **THEN** the task's priority SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `p` and no task is selected (empty or fully filtered list)
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Filter tasks
The user SHALL press `f` or `/` to enter filter mode. The footer SHALL display a text input for a filter expression. Supported filters: `status:open`, `status:done`, `priority:high`, `priority:medium`, `priority:low`, `priority:critical`, `tag:<name>`, `project:<name>`. Pressing `Esc` in normal mode SHALL clear any active filter and show all tasks.

#### Scenario: Filter by status
- **WHEN** the user presses `f`, types "status:open", and presses `Enter`
- **THEN** the task table SHALL show only tasks with open status

#### Scenario: Filter by tag
- **WHEN** the user presses `f`, types "tag:frontend", and presses `Enter`
- **THEN** the task table SHALL show only tasks tagged with "frontend"

#### Scenario: Filter by critical priority
- **WHEN** the user presses `f`, types "priority:critical", and presses `Enter`
- **THEN** the task table SHALL show only tasks with priority critical

#### Scenario: Filter by project
- **WHEN** the user presses `f`, types "project:Work", and presses `Enter`
- **THEN** the task table SHALL show only tasks with project "Work" (case-insensitive)

#### Scenario: Clear filter
- **WHEN** a filter is active and the user presses `Esc`
- **THEN** the filter SHALL be cleared and all tasks SHALL be displayed

#### Scenario: No matching tasks
- **WHEN** the user applies a filter that matches no tasks
- **THEN** the table area SHALL display "No tasks match filter."
