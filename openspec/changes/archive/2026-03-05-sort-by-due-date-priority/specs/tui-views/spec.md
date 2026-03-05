## ADDED Requirements

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
