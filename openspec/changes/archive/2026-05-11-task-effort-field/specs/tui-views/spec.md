## ADDED Requirements

### Requirement: GroupBy::Effort
The TUI SHALL support `GroupBy::Effort` as a grouping option. When active, tasks SHALL be grouped by their `effort` field value. Tasks with `effort: None` SHALL appear in a group labeled `(none)`.

#### Scenario: Effort grouping groups by value
- **WHEN** `GroupBy::Effort` is active and tasks include high, medium, low, and unset effort values
- **THEN** four groups SHALL appear: `High`, `Medium`, `Low`, and `(none)`

#### Scenario: Effort none group label
- **WHEN** `GroupBy::Effort` is active and a task has `effort: None`
- **THEN** that task SHALL appear under the `(none)` group

### Requirement: GroupBy cycle includes Effort
The `G` key cycle SHALL include `GroupBy::Effort` inserted after `GroupBy::Priority`: `None → Project → Agent → Priority → Effort → DueDate → None`.

#### Scenario: G key cycles through Effort
- **WHEN** the active grouping is `GroupBy::Priority` and the user presses `G`
- **THEN** the active grouping SHALL change to `GroupBy::Effort`

#### Scenario: G key cycles from Effort to DueDate
- **WHEN** the active grouping is `GroupBy::Effort` and the user presses `G`
- **THEN** the active grouping SHALL change to `GroupBy::DueDate`

### Requirement: Effort displayed in task list
The TUI task list SHALL display each task's effort level as a short label in a dedicated column. Tasks with `effort: None` SHALL show an empty cell in that column.

#### Scenario: High effort label
- **WHEN** a task has `effort: Some(Effort::High)` and is displayed in the list
- **THEN** the effort column SHALL show `H` or equivalent short label

#### Scenario: Medium effort label
- **WHEN** a task has `effort: Some(Effort::Medium)`
- **THEN** the effort column SHALL show `M`

#### Scenario: Low effort label
- **WHEN** a task has `effort: Some(Effort::Low)`
- **THEN** the effort column SHALL show `L`

#### Scenario: No effort shows blank
- **WHEN** a task has `effort: None`
- **THEN** the effort column SHALL be empty

### Requirement: Effort displayed in task detail panel
The task detail panel SHALL show the effort value as a full label when set. When `effort: None`, the field SHALL be shown as `—` or omitted.

#### Scenario: Detail shows full effort label
- **WHEN** the detail panel is open for a task with `effort: Some(Effort::High)`
- **THEN** the panel SHALL display `Effort: High`

### Requirement: Effort editing via picker
The user SHALL press `E` in task detail/edit mode to open an effort picker overlay. The picker SHALL list `High`, `Medium`, `Low`, and `(clear)` options. Selecting an option SHALL set the task's effort accordingly and close the picker. Pressing `Esc` SHALL cancel without change.

#### Scenario: Open effort picker
- **WHEN** a task is selected and the user presses `E`
- **THEN** an effort picker overlay SHALL appear with options: High, Medium, Low, (clear)

#### Scenario: Select effort value
- **WHEN** the effort picker is open and the user selects `Medium`
- **THEN** the task's `effort` SHALL be set to `Some(Effort::Medium)` and the picker SHALL close

#### Scenario: Clear effort
- **WHEN** the effort picker is open and the user selects `(clear)`
- **THEN** the task's `effort` SHALL be set to `None` and the picker SHALL close

#### Scenario: Cancel effort picker
- **WHEN** the effort picker is open and the user presses `Esc`
- **THEN** the task's `effort` SHALL be unchanged and the picker SHALL close
