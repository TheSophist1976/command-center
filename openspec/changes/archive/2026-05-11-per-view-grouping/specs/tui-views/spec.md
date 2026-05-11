## MODIFIED Requirements

### Requirement: View switching keybinding
The user SHALL press `v` in normal mode to cycle to the next view in the order: Due → NoDueDate → Recurring → Notes → Due. The user SHALL press `V` (shift-v) to cycle to the previous view in reverse order. The view change SHALL take effect immediately, re-filtering the displayed task list. When switching views, the TUI SHALL restore that view's saved grouping.

#### Scenario: Cycle forward through views
- **WHEN** the active view is Due and the user presses `v`
- **THEN** the active view SHALL change to NoDueDate and the task list SHALL be re-filtered

#### Scenario: Cycle backward through views
- **WHEN** the active view is Due and the user presses `V`
- **THEN** the active view SHALL change to Notes

#### Scenario: Wrap around forward
- **WHEN** the active view is Notes and the user presses `v`
- **THEN** the active view SHALL change to Due

#### Scenario: View switch restores saved grouping
- **WHEN** the user switches from view A (with grouping `agent`) to view B (with saved grouping `priority`)
- **THEN** the active grouping SHALL immediately change to `priority`

#### Scenario: View switch with no saved grouping defaults to none
- **WHEN** the user switches to a view that has no saved grouping
- **THEN** the active grouping SHALL be `none`
