## ADDED Requirements

### Requirement: Recur flag on add command
The `task add` command SHALL accept an optional `--recur <pattern>` flag. The value SHALL be parsed as a `Recurrence` (e.g., `daily`, `weekly`, `monthly`, `yearly`, `monthly:3:thu`). If provided, the new task SHALL be created with the specified recurrence.

#### Scenario: Add task with weekly recurrence
- **WHEN** the user runs `task add "Weekly review" --recur weekly`
- **THEN** a new task SHALL be created with title "Weekly review" and `recurrence: Some(Interval(Weekly))`

#### Scenario: Add task with nth weekday recurrence
- **WHEN** the user runs `task add "Team meeting" --recur monthly:3:thu`
- **THEN** a new task SHALL be created with `recurrence: Some(NthWeekday { n: 3, weekday: Thu })`

#### Scenario: Invalid recurrence pattern
- **WHEN** the user runs `task add "Task" --recur biweekly`
- **THEN** the system SHALL print an error indicating the invalid recurrence pattern

### Requirement: Recur flag on edit command
The `task edit` command SHALL accept an optional `--recur <pattern>` flag. The value `none` SHALL clear the recurrence. Any other valid pattern SHALL set the recurrence.

#### Scenario: Set recurrence on existing task
- **WHEN** the user runs `task edit 5 --recur monthly`
- **THEN** task 5's recurrence SHALL be set to `Interval(Monthly)` and saved

#### Scenario: Clear recurrence on existing task
- **WHEN** the user runs `task edit 5 --recur none`
- **THEN** task 5's recurrence SHALL be set to `None` and saved
