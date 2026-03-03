<!-- MODIFIED: tui -->

### MODIFIED Requirement: Task table display

#### MODIFIED Scenario: Priority column coloring
- **WHEN** the task table is rendered
- **THEN** priority cells SHALL use colors from the `theme` module instead of hardcoded `Color::*` values

#### ADDED Scenario: Done task row styling
- **WHEN** a task with status Done is rendered in the table
- **THEN** all cells in that row SHALL use `theme::DONE_TEXT` foreground color, overriding priority coloring

#### ADDED Scenario: Overdue row styling
- **WHEN** an open task's due date is before today's date
- **THEN** the entire row SHALL be rendered in `theme::OVERDUE` foreground color and the status column SHALL display `[!]` instead of `[ ]`

### MODIFIED Requirement: Header and footer bars

#### MODIFIED Scenario: Bar styling
- **WHEN** the header or footer bar is rendered
- **THEN** it SHALL use `theme::BAR_FG` foreground and `theme::BAR_BG` background instead of hardcoded Cyan/Black

### MODIFIED Requirement: Detail panel

#### MODIFIED Scenario: Active field highlight
- **WHEN** the detail panel edit mode highlights the active field
- **THEN** it SHALL use `theme::HIGHLIGHT_BG` instead of hardcoded `Color::DarkGray`
