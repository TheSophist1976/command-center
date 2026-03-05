## MODIFIED Requirements

### Requirement: View enum
The TUI SHALL support a set of predefined views: Today, All, Weekly, Monthly, Yearly, NoDueDate, and Recurring. Each view defines a filter predicate applied to the full task list before any user-applied filters. Completed tasks SHALL only appear in the All view; all other views (including Recurring) SHALL exclude tasks with status Done. Open tasks with a due date in the past (before today) SHALL appear in all time-based views (Today, Weekly, Monthly, Yearly) in addition to their normal matching criteria.

#### Scenario: Overdue completed tasks remain hidden
- **WHEN** a task has status Done and a `due_date` before today
- **THEN** the task SHALL NOT appear in Today, Weekly, Monthly, Yearly, or Recurring views (only in the All view)

### Requirement: Recurring view filtering
The Recurring view SHALL show only open tasks whose `recurrence` field is set (not `None`). Tasks without a recurrence SHALL be excluded. Done tasks with a recurrence SHALL be excluded. The overdue-in-all-views logic SHALL NOT apply to this view — only recurrence presence and open status determine visibility.

#### Scenario: Recurring open task is shown
- **WHEN** the Recurring view is active and an open task has `recurrence` set to any pattern
- **THEN** the task SHALL appear in the view

#### Scenario: Recurring done task is hidden
- **WHEN** the Recurring view is active and a done task has `recurrence` set
- **THEN** the task SHALL NOT appear in the view

#### Scenario: Non-recurring task is hidden
- **WHEN** the Recurring view is active and a task has `recurrence` of `None`
- **THEN** the task SHALL NOT appear in the view
