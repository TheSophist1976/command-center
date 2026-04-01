## ADDED Requirements

### Requirement: Due view with sub-window
The TUI SHALL provide a `View::Due` that shows open tasks filtered by a time window. The active window SHALL cycle through five levels: Day, Week, Month, Year, All. The user SHALL press `]` to expand the window to the next level and `[` to shrink it. The window level SHALL be stored in `App` and persist across view switches.

#### Scenario: Day window matches Today behavior
- **WHEN** `View::Due` is active with window `Day`
- **THEN** the task list SHALL show open tasks due today, open tasks with no due date, and open overdue tasks (identical to the former Today view)

#### Scenario: Week window shows tasks due this calendar week
- **WHEN** `View::Due` is active with window `Week`
- **THEN** the task list SHALL show open tasks whose due date falls within the current calendar week (Monday–Sunday), plus overdue open tasks

#### Scenario: Month window shows tasks due this calendar month
- **WHEN** `View::Due` is active with window `Month`
- **THEN** the task list SHALL show open tasks whose due date falls within the current calendar month, plus overdue open tasks

#### Scenario: Year window shows tasks due this calendar year
- **WHEN** `View::Due` is active with window `Year`
- **THEN** the task list SHALL show open tasks whose due date falls within the current calendar year, plus overdue open tasks

#### Scenario: All window shows all open tasks
- **WHEN** `View::Due` is active with window `All`
- **THEN** the task list SHALL show all open tasks regardless of due date (equivalent to the former All view)

#### Scenario: ] expands window
- **WHEN** `View::Due` is active with window `Day` and the user presses `]`
- **THEN** the window SHALL change to `Week`

#### Scenario: ] wraps from All back to Day
- **WHEN** `View::Due` is active with window `All` and the user presses `]`
- **THEN** the window SHALL change to `Day`

#### Scenario: [ shrinks window
- **WHEN** `View::Due` is active with window `Week` and the user presses `[`
- **THEN** the window SHALL change to `Day`

#### Scenario: [ wraps from Day back to All
- **WHEN** `View::Due` is active with window `Day` and the user presses `[`
- **THEN** the window SHALL change to `All`

#### Scenario: ] and [ are no-ops on other views
- **WHEN** any view other than `View::Due` is active and the user presses `]` or `[`
- **THEN** no action SHALL occur

### Requirement: Due view header label
The TUI header SHALL display the active window alongside the view name when `View::Due` is active.

#### Scenario: Header shows window name
- **WHEN** `View::Due` is active with window `Week`
- **THEN** the header SHALL show `Due [This Week]`

#### Scenario: Window label mapping
- **WHEN** each window level is active
- **THEN** the labels SHALL be: Day → "Today", Week → "This Week", Month → "This Month", Year → "This Year", All → "All Tasks"
