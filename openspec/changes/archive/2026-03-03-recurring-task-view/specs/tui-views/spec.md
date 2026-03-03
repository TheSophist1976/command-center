## ADDED Requirements

### Requirement: Recurring view filtering
The Recurring view SHALL show all tasks (open and done) whose `recurrence` field is set (not `None`). Tasks without a recurrence SHALL be excluded. The overdue-in-all-views logic SHALL NOT apply to this view — only recurrence presence determines visibility.

#### Scenario: Recurring open task is shown
- **WHEN** the Recurring view is active and an open task has `recurrence` set to any pattern
- **THEN** the task SHALL appear in the view

#### Scenario: Recurring done task is shown
- **WHEN** the Recurring view is active and a done task has `recurrence` set
- **THEN** the task SHALL appear in the view

#### Scenario: Non-recurring task is hidden
- **WHEN** the Recurring view is active and a task has `recurrence` of `None`
- **THEN** the task SHALL NOT appear in the view

## MODIFIED Requirements

### Requirement: View enum
The TUI SHALL support a set of predefined views: Today, All, Weekly, Monthly, Yearly, NoDueDate, and Recurring. Each view defines a filter predicate applied to the full task list before any user-applied filters. Completed tasks SHALL only appear in the All and Recurring views; all other views SHALL exclude tasks with status Done. Open tasks with a due date in the past (before today) SHALL appear in all time-based views (Today, Weekly, Monthly, Yearly) in addition to their normal matching criteria.

#### Scenario: View variants
- **WHEN** the TUI initializes the view system
- **THEN** the following views SHALL be available: Today, All, Weekly, Monthly, Yearly, NoDueDate, Recurring

#### Scenario: Overdue open tasks shown in time-based views
- **WHEN** a task has status Open and a `due_date` before today
- **THEN** the task SHALL appear in Today, Weekly, Monthly, and Yearly views

#### Scenario: Overdue completed tasks remain hidden
- **WHEN** a task has status Done and a `due_date` before today
- **THEN** the task SHALL NOT appear in Today, Weekly, Monthly, or Yearly views (only in All and Recurring views if recurring)

#### Scenario: Overdue tasks not shown in NoDueDate view
- **WHEN** a task has a `due_date` before today
- **THEN** the task SHALL NOT appear in the NoDueDate view (it has a due date)

### Requirement: View switching keybinding
The user SHALL press `v` in normal mode to cycle to the next view in the order: Today → All → Weekly → Monthly → Yearly → NoDueDate → Recurring → Today. The user SHALL press `V` (shift-v) to cycle to the previous view in reverse order. The view change SHALL take effect immediately, re-filtering the displayed task list.

#### Scenario: Cycle forward through views
- **WHEN** the active view is Today and the user presses `v`
- **THEN** the active view SHALL change to All and the task list SHALL be re-filtered

#### Scenario: Cycle backward through views
- **WHEN** the active view is Today and the user presses `V`
- **THEN** the active view SHALL change to Recurring and the task list SHALL be re-filtered

#### Scenario: Wrap around forward
- **WHEN** the active view is Recurring and the user presses `v`
- **THEN** the active view SHALL change to Today

### Requirement: View name in header
The TUI header SHALL display the name of the active view alongside the title. The display names SHALL be: "Today", "All Tasks", "This Week", "This Month", "This Year", "No Due Date", "Recurring".

#### Scenario: Header shows view name
- **WHEN** the Recurring view is active
- **THEN** the header SHALL display "Recurring" alongside the title
