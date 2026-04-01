## MODIFIED Requirements

### Requirement: View enum
The TUI SHALL support a set of predefined views: Today, All, Weekly, Monthly, Yearly, NoDueDate, Recurring, ByAgent, and Notes. Each view defines a filter predicate applied to the full task list before any user-applied filters. Completed tasks SHALL only appear in the All view; all other views SHALL exclude tasks with status Done. The Notes view is a separate view that displays markdown notes rather than tasks. The ByAgent view displays all open tasks grouped under agent section headers. Open tasks with a due date in the past (before today) SHALL appear in all time-based views (Today, Weekly, Monthly, Yearly) in addition to their normal matching criteria.

#### Scenario: View variants
- **WHEN** the TUI initializes the view system
- **THEN** the following views SHALL be available: Today, All, Weekly, Monthly, Yearly, NoDueDate, Recurring, ByAgent, Notes

#### Scenario: Overdue open tasks shown in time-based views
- **WHEN** a task has status Open and a `due_date` before today
- **THEN** the task SHALL appear in Today, Weekly, Monthly, and Yearly views

#### Scenario: Overdue completed tasks remain hidden
- **WHEN** a task has status Done and a `due_date` before today
- **THEN** the task SHALL NOT appear in Today, Weekly, Monthly, or Yearly views (only in All view)

#### Scenario: Overdue tasks not shown in NoDueDate view
- **WHEN** a task has a `due_date` before today
- **THEN** the task SHALL NOT appear in the NoDueDate view (it has a due date)

### Requirement: View switching keybinding
The user SHALL press `v` in normal mode to cycle to the next view in the order: Today → All → Weekly → Monthly → Yearly → NoDueDate → Recurring → ByAgent → Notes → Today. The user SHALL press `V` (shift-v) to cycle to the previous view in reverse order. The view change SHALL take effect immediately, re-filtering the displayed task list.

#### Scenario: Cycle forward through views
- **WHEN** the active view is Today and the user presses `v`
- **THEN** the active view SHALL change to All and the task list SHALL be re-filtered

#### Scenario: Cycle backward through views
- **WHEN** the active view is Today and the user presses `V`
- **THEN** the active view SHALL change to Notes and the task list SHALL be re-filtered

#### Scenario: Cycle forward from Recurring to ByAgent
- **WHEN** the active view is Recurring and the user presses `v`
- **THEN** the active view SHALL change to ByAgent

#### Scenario: Wrap around forward
- **WHEN** the active view is Notes and the user presses `v`
- **THEN** the active view SHALL change to Today
