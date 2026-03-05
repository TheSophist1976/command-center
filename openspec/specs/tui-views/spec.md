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

### Requirement: Today view filtering
The Today view SHALL show open tasks whose `due_date` equals today's date, whose `due_date` is `None`, or whose `due_date` is in the past (overdue). Tasks with a `due_date` in the future (other than today) SHALL be excluded.

#### Scenario: Task due today is shown
- **WHEN** the Today view is active and a task has `due_date` equal to today
- **THEN** the task SHALL appear in the view

#### Scenario: Task with no due date is shown
- **WHEN** the Today view is active and a task has `due_date` of `None`
- **THEN** the task SHALL appear in the view

#### Scenario: Task due tomorrow is hidden
- **WHEN** the Today view is active and a task has `due_date` equal to tomorrow
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Overdue open task is shown
- **WHEN** the Today view is active and an open task has `due_date` in the past (before today)
- **THEN** the task SHALL appear in the view

#### Scenario: Completed task is hidden
- **WHEN** the Today view is active and a task has status Done
- **THEN** the task SHALL NOT appear in the view

### Requirement: All view filtering
The All view SHALL show all tasks with no view-level filtering applied, including completed tasks.

#### Scenario: All tasks visible
- **WHEN** the All view is active
- **THEN** every task SHALL appear in the view regardless of `due_date` or status

### Requirement: Weekly view filtering
The Weekly view SHALL show open tasks whose `due_date` falls within the current ISO week (Monday through Sunday of the week containing today) OR whose `due_date` is in the past (overdue). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Task due this week is shown
- **WHEN** the Weekly view is active and a task has `due_date` within the current Monday–Sunday range
- **THEN** the task SHALL appear in the view

#### Scenario: Task due next week is hidden
- **WHEN** the Weekly view is active and a task has `due_date` in the following week
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Task with no due date is hidden
- **WHEN** the Weekly view is active and a task has `due_date` of `None`
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Overdue open task is shown
- **WHEN** the Weekly view is active and an open task has `due_date` before the current week
- **THEN** the task SHALL appear in the view

### Requirement: Monthly view filtering
The Monthly view SHALL show open tasks whose `due_date` falls within the current calendar month (same year and month as today) OR whose `due_date` is in the past (overdue). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Task due this month is shown
- **WHEN** the Monthly view is active and a task has `due_date` in the current calendar month
- **THEN** the task SHALL appear in the view

#### Scenario: Task due next month is hidden
- **WHEN** the Monthly view is active and a task has `due_date` in the following month
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Overdue open task is shown
- **WHEN** the Monthly view is active and an open task has `due_date` before the current month
- **THEN** the task SHALL appear in the view

### Requirement: Yearly view filtering
The Yearly view SHALL show open tasks whose `due_date` falls within the current calendar year OR whose `due_date` is in the past (overdue). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Task due this year is shown
- **WHEN** the Yearly view is active and a task has `due_date` in the current calendar year
- **THEN** the task SHALL appear in the view

#### Scenario: Task due next year is hidden
- **WHEN** the Yearly view is active and a task has `due_date` in the following year
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Overdue open task is shown
- **WHEN** the Yearly view is active and an open task has `due_date` before the current year
- **THEN** the task SHALL appear in the view

### Requirement: No Due Date view filtering
The NoDueDate view SHALL show only open tasks whose `due_date` is `None`.

#### Scenario: Task with no due date is shown
- **WHEN** the NoDueDate view is active and a task has `due_date` of `None`
- **THEN** the task SHALL appear in the view

#### Scenario: Task with a due date is hidden
- **WHEN** the NoDueDate view is active and a task has a `due_date` set
- **THEN** the task SHALL NOT appear in the view

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

### Requirement: View and filter stacking
User-applied filters (status, priority, tag, project) SHALL be applied on top of the active view's filter. Clearing a user filter with `Esc` SHALL NOT change the active view.

#### Scenario: Filter stacks on view
- **WHEN** the Weekly view is active and the user applies a `status:open` filter
- **THEN** only tasks that are both due this week AND have status open SHALL be displayed

#### Scenario: Clearing filter preserves view
- **WHEN** the Weekly view is active with a `status:open` filter and the user presses `Esc`
- **THEN** the filter SHALL be cleared but the Weekly view SHALL remain active

### Requirement: View name in header
The TUI header SHALL display the name of the active view alongside the title. The display names SHALL be: "Today", "All Tasks", "This Week", "This Month", "This Year", "No Due Date", "Recurring".

#### Scenario: Header shows view name
- **WHEN** the Recurring view is active
- **THEN** the header SHALL display "Recurring" alongside the title

### Requirement: View hint in footer
The TUI footer SHALL include `v:view` in its keybinding hints when in normal mode.

#### Scenario: Footer shows view keybinding
- **WHEN** the TUI is in normal mode
- **THEN** the footer SHALL include `v:view` in the keybinding hint string

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

### Requirement: View results are sorted
All views SHALL return their filtered task lists sorted according to the default task sort order (due date ascending, priority descending). This applies to every view: Today, All, Weekly, Monthly, Yearly, NoDueDate, and Recurring. The sort SHALL be applied after the view's filter predicate and any user-applied filters, immediately before the results are used for display or selection.

#### Scenario: Today view tasks are sorted
- **WHEN** the Today view contains tasks due today with priorities Medium, Critical, and Low
- **THEN** the tasks SHALL be displayed in order: Critical, Medium, Low

#### Scenario: All view tasks are sorted by due date then priority
- **WHEN** the All view contains tasks with mixed due dates and priorities
- **THEN** the tasks SHALL be displayed sorted by due date ascending (None last), then priority descending within each date group

#### Scenario: Recurring view tasks are sorted
- **WHEN** the Recurring view contains recurring tasks with due dates 2026-03-20, 2026-03-05, and None
- **THEN** the tasks SHALL be displayed in order: 2026-03-05, 2026-03-20, None

#### Scenario: NoDueDate view tasks are sorted by priority
- **WHEN** the NoDueDate view contains tasks with no due date and priorities Low, High, Critical
- **THEN** the tasks SHALL be displayed in order: Critical, High, Low
