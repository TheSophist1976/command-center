### Requirement: Centralized color theme

The TUI SHALL define all colors in a single `theme` module within `src/tui.rs`. Draw functions SHALL reference theme constants instead of inline `Color::*` values.

#### Scenario: Theme defines bar colors
- **WHEN** the header or footer bar is rendered
- **THEN** it SHALL use `theme::BAR_FG` and `theme::BAR_BG`

#### Scenario: Theme defines priority colors
- **WHEN** a task priority is rendered
- **THEN** it SHALL use `theme::PRIORITY_CRITICAL`, `theme::PRIORITY_HIGH`, `theme::PRIORITY_MEDIUM`, or `theme::PRIORITY_LOW`

#### Scenario: Theme defines selection highlight
- **WHEN** the selected row is highlighted
- **THEN** it SHALL use `theme::HIGHLIGHT_BG` as the background color

#### Scenario: Theme defines chat colors
- **WHEN** chat messages are rendered
- **THEN** user messages SHALL use `theme::CHAT_USER`, task lists SHALL use `theme::CHAT_TASK_LIST`, and errors SHALL use `theme::CHAT_ERROR`

### Requirement: Completed task styling

Done tasks SHALL be visually distinct from open tasks in the table.

#### Scenario: Done task rendered with muted color
- **WHEN** a task with status Done is displayed in the table
- **THEN** the entire row SHALL be rendered with `theme::DONE_TEXT` foreground color

### Requirement: Overdue task visual treatment

Open tasks with due dates in the past SHALL be visually prominent so the user immediately sees they are overdue.

#### Scenario: Overdue open task row is red
- **WHEN** an open task has a due date before today
- **THEN** the entire row SHALL be rendered with `theme::OVERDUE` foreground color, overriding priority coloring

#### Scenario: Overdue open task shows warning marker
- **WHEN** an open task has a due date before today
- **THEN** the status column SHALL display `[!]` instead of `[ ]`

#### Scenario: Completed task is not marked overdue
- **WHEN** a done task has a due date before today
- **THEN** it SHALL NOT use overdue styling (it uses the done muted color and `[x]` marker instead)
