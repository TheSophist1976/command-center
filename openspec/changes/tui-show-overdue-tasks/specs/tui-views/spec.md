## MODIFIED Requirements

### Requirement: View enum
The TUI SHALL support a set of predefined views: Today, All, Weekly, Monthly, Yearly, and NoDueDate. Each view defines a filter predicate applied to the full task list before any user-applied filters. Completed tasks SHALL only appear in the All view; all other views SHALL exclude tasks with status Done. Open tasks with a due date in the past (before today) SHALL appear in all time-based views (Today, Weekly, Monthly, Yearly) in addition to their normal matching criteria.

#### Scenario: View variants
- **WHEN** the TUI initializes the view system
- **THEN** the following views SHALL be available: Today, All, Weekly, Monthly, Yearly, NoDueDate

#### Scenario: Overdue open tasks shown in time-based views
- **WHEN** a task has status Open and a `due_date` before today
- **THEN** the task SHALL appear in Today, Weekly, Monthly, and Yearly views

#### Scenario: Overdue completed tasks remain hidden
- **WHEN** a task has status Done and a `due_date` before today
- **THEN** the task SHALL NOT appear in Today, Weekly, Monthly, or Yearly views (only in All view)

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

### Requirement: Weekly view filtering
The Weekly view SHALL show open tasks whose `due_date` falls within the current ISO week (Monday through Sunday of the week containing today) OR whose `due_date` is in the past (overdue). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Overdue open task is shown
- **WHEN** the Weekly view is active and an open task has `due_date` before the current week
- **THEN** the task SHALL appear in the view

### Requirement: Monthly view filtering
The Monthly view SHALL show open tasks whose `due_date` falls within the current calendar month (same year and month as today) OR whose `due_date` is in the past (overdue). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Overdue open task is shown
- **WHEN** the Monthly view is active and an open task has `due_date` before the current month
- **THEN** the task SHALL appear in the view

### Requirement: Yearly view filtering
The Yearly view SHALL show open tasks whose `due_date` falls within the current calendar year OR whose `due_date` is in the past (overdue). Tasks with `due_date` of `None` SHALL be excluded.

#### Scenario: Overdue open task is shown
- **WHEN** the Yearly view is active and an open task has `due_date` before the current year
- **THEN** the task SHALL appear in the view
