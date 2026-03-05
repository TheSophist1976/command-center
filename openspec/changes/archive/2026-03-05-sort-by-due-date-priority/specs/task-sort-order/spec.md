## ADDED Requirements

### Requirement: Default task sort order
The system SHALL sort task lists by due date ascending then priority descending. Tasks with a `due_date` value SHALL appear before tasks with `due_date` of `None`. Among tasks with due dates, earlier dates SHALL appear first. Among tasks with the same due date (or both `None`), tasks SHALL be ordered by priority: Critical first, then High, Medium, Low. This sort order SHALL be applied after view filtering and user-applied filters, before display.

#### Scenario: Tasks sorted by due date ascending
- **WHEN** the task list contains tasks with due dates 2026-03-10, 2026-03-05, and 2026-03-08
- **THEN** the tasks SHALL be displayed in order: 2026-03-05, 2026-03-08, 2026-03-10

#### Scenario: Tasks with no due date sorted last
- **WHEN** the task list contains tasks with due dates 2026-03-10, None, and 2026-03-05
- **THEN** the tasks SHALL be displayed in order: 2026-03-05, 2026-03-10, None

#### Scenario: Same due date sorted by priority descending
- **WHEN** the task list contains three tasks all due 2026-03-10 with priorities Low, Critical, and Medium
- **THEN** the tasks SHALL be displayed in order: Critical, Medium, Low

#### Scenario: No due date tasks sorted by priority descending
- **WHEN** the task list contains tasks with no due date and priorities Low, High, and Medium
- **THEN** the tasks SHALL be displayed in order: High, Medium, Low

#### Scenario: Sort applied after filtering
- **WHEN** a view filter and user filter are applied, and three tasks pass both filters with due dates 2026-03-15, 2026-03-01, and 2026-03-10
- **THEN** the filtered results SHALL be displayed in order: 2026-03-01, 2026-03-10, 2026-03-15

### Requirement: Priority ordering
The `Priority` enum SHALL support ordered comparison. The ordering from highest to lowest priority SHALL be: Critical, High, Medium, Low. This ordering SHALL be used for sort comparisons.

#### Scenario: Critical is highest priority
- **WHEN** comparing Priority::Critical with Priority::Low
- **THEN** Critical SHALL be considered higher priority than Low

#### Scenario: Priority ordering is total
- **WHEN** all four priority values are sorted in descending order
- **THEN** the order SHALL be Critical, High, Medium, Low
