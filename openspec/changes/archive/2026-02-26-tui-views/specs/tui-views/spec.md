## ADDED Requirements

### Requirement: View enum
The TUI SHALL support a set of predefined views: Today, All, Weekly, Monthly, Yearly, and NoDueDate. Each view defines a filter predicate applied to the full task list before any user-applied filters.

#### Scenario: View variants
- **WHEN** the TUI initializes the view system
- **THEN** the following views SHALL be available: Today, All, Weekly, Monthly, Yearly, NoDueDate

### Requirement: Today view filtering
The Today view SHALL show tasks whose `due_date` equals today's date OR whose `due_date` is `None`. Tasks with a `due_date` in the past or future (other than today) SHALL be excluded.

#### Scenario: Task due today is shown
- **WHEN** the Today view is active and a task has `due_date` equal to today
- **THEN** the task SHALL appear in the view

#### Scenario: Task with no due date is shown
- **WHEN** the Today view is active and a task has `due_date` of `None`
- **THEN** the task SHALL appear in the view

#### Scenario: Task due tomorrow is hidden
- **WHEN** the Today view is active and a task has `due_date` equal to tomorrow
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Overdue task is hidden
- **WHEN** the Today view is active and a task has `due_date` in the past (before today)
- **THEN** the task SHALL NOT appear in the view

### Requirement: All view filtering
The All view SHALL show all tasks with no view-level filtering applied.

#### Scenario: All tasks visible
- **WHEN** the All view is active
- **THEN** every task SHALL appear in the view regardless of `due_date`

### Requirement: Weekly view filtering
The Weekly view SHALL show tasks whose `due_date` falls within the current ISO week (Monday through Sunday of the week containing today). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Task due this week is shown
- **WHEN** the Weekly view is active and a task has `due_date` within the current Monday–Sunday range
- **THEN** the task SHALL appear in the view

#### Scenario: Task due next week is hidden
- **WHEN** the Weekly view is active and a task has `due_date` in the following week
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Task with no due date is hidden
- **WHEN** the Weekly view is active and a task has `due_date` of `None`
- **THEN** the task SHALL NOT appear in the view

### Requirement: Monthly view filtering
The Monthly view SHALL show tasks whose `due_date` falls within the current calendar month (same year and month as today). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Task due this month is shown
- **WHEN** the Monthly view is active and a task has `due_date` in the current calendar month
- **THEN** the task SHALL appear in the view

#### Scenario: Task due next month is hidden
- **WHEN** the Monthly view is active and a task has `due_date` in the following month
- **THEN** the task SHALL NOT appear in the view

### Requirement: Yearly view filtering
The Yearly view SHALL show tasks whose `due_date` falls within the current calendar year. Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Task due this year is shown
- **WHEN** the Yearly view is active and a task has `due_date` in the current calendar year
- **THEN** the task SHALL appear in the view

#### Scenario: Task due next year is hidden
- **WHEN** the Yearly view is active and a task has `due_date` in the following year
- **THEN** the task SHALL NOT appear in the view

### Requirement: No Due Date view filtering
The NoDueDate view SHALL show only tasks whose `due_date` is `None`.

#### Scenario: Task with no due date is shown
- **WHEN** the NoDueDate view is active and a task has `due_date` of `None`
- **THEN** the task SHALL appear in the view

#### Scenario: Task with a due date is hidden
- **WHEN** the NoDueDate view is active and a task has a `due_date` set
- **THEN** the task SHALL NOT appear in the view

### Requirement: View switching keybinding
The user SHALL press `v` in normal mode to cycle to the next view in the order: Today → All → Weekly → Monthly → Yearly → NoDueDate → Today. The user SHALL press `V` (shift-v) to cycle to the previous view in reverse order. The view change SHALL take effect immediately, re-filtering the displayed task list.

#### Scenario: Cycle forward through views
- **WHEN** the active view is Today and the user presses `v`
- **THEN** the active view SHALL change to All and the task list SHALL be re-filtered

#### Scenario: Cycle backward through views
- **WHEN** the active view is Today and the user presses `V`
- **THEN** the active view SHALL change to NoDueDate and the task list SHALL be re-filtered

#### Scenario: Wrap around forward
- **WHEN** the active view is NoDueDate and the user presses `v`
- **THEN** the active view SHALL change to Today

### Requirement: View and filter stacking
User-applied filters (status, priority, tag, project) SHALL be applied on top of the active view's filter. Clearing a user filter with `Esc` SHALL NOT change the active view.

#### Scenario: Filter stacks on view
- **WHEN** the Weekly view is active and the user applies a `status:open` filter
- **THEN** only tasks that are both due this week AND have status open SHALL be displayed

#### Scenario: Clearing filter preserves view
- **WHEN** the Weekly view is active with a `status:open` filter and the user presses `Esc`
- **THEN** the filter SHALL be cleared but the Weekly view SHALL remain active

### Requirement: View name in header
The TUI header SHALL display the name of the active view alongside the title. The display names SHALL be: "Today", "All Tasks", "This Week", "This Month", "This Year", "No Due Date".

#### Scenario: Header shows view name
- **WHEN** the Weekly view is active
- **THEN** the header SHALL display "This Week" alongside the title

### Requirement: View hint in footer
The TUI footer SHALL include `v:view` in its keybinding hints when in normal mode.

#### Scenario: Footer shows view keybinding
- **WHEN** the TUI is in normal mode
- **THEN** the footer SHALL include `v:view` in the keybinding hint string
