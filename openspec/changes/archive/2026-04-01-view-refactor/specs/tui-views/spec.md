## REMOVED Requirements

### Requirement: Today view filtering
**Reason**: Replaced by `View::Due` with window `Day`, which provides identical behavior.
**Migration**: Use `View::Due` (window defaults to Day on first launch).

### Requirement: All view filtering
**Reason**: Replaced by `View::Due` with window `All`.
**Migration**: Press `]` from any Due window to expand to All.

### Requirement: Weekly view filtering
**Reason**: Replaced by `View::Due` with window `Week`.
**Migration**: Press `]` from Day window to reach Week.

### Requirement: Monthly view filtering
**Reason**: Replaced by `View::Due` with window `Month`.
**Migration**: Press `]` from Week window to reach Month.

### Requirement: Yearly view filtering
**Reason**: Replaced by `View::Due` with window `Year`.
**Migration**: Press `]` from Month window to reach Year.

## MODIFIED Requirements

### Requirement: View enum
The TUI SHALL support a set of predefined views: Due, NoDueDate, Recurring, and Notes. Each view defines a filter predicate applied to the full task list before any user-applied filters. Completed tasks SHALL NOT appear in any view. The Notes view is a separate view that displays markdown notes rather than tasks. Open tasks with a due date in the past (before today) SHALL appear in the Due view at Week/Month/Year/All window levels.

#### Scenario: View variants
- **WHEN** the TUI initializes the view system
- **THEN** the following views SHALL be available: Due, NoDueDate, Recurring, Notes

#### Scenario: Done tasks excluded from all views
- **WHEN** a task has status Done
- **THEN** it SHALL NOT appear in any view

#### Scenario: Overdue tasks shown in Due view at expanded windows
- **WHEN** a task has status Open and a `due_date` before today and `View::Due` is active with window Week/Month/Year/All
- **THEN** the task SHALL appear in the view

### Requirement: View switching keybinding
The user SHALL press `v` in normal mode to cycle to the next view in the order: Due → NoDueDate → Recurring → Notes → Due. The user SHALL press `V` (shift-v) to cycle to the previous view in reverse order. The view change SHALL take effect immediately, re-filtering the displayed task list.

#### Scenario: Cycle forward through views
- **WHEN** the active view is Due and the user presses `v`
- **THEN** the active view SHALL change to NoDueDate and the task list SHALL be re-filtered

#### Scenario: Cycle backward through views
- **WHEN** the active view is Due and the user presses `V`
- **THEN** the active view SHALL change to Notes

#### Scenario: Wrap around forward
- **WHEN** the active view is Notes and the user presses `v`
- **THEN** the active view SHALL change to Due

### Requirement: View name in header
The TUI header SHALL display the name of the active view. For `View::Due`, the header SHALL include the window label (e.g. `Due [This Week]`). Other display names SHALL be: "No Due Date", "Recurring", "Notes".

#### Scenario: Due view header includes window
- **WHEN** `View::Due` is active with window `Month`
- **THEN** the header SHALL show `Due [This Month]`

#### Scenario: Other views show plain name
- **WHEN** the Recurring view is active
- **THEN** the header SHALL show `Recurring`
