## ADDED Requirements

### Requirement: Due date field
The `Task` struct SHALL include an optional `due_date` field of type `Option<NaiveDate>`. When set, it represents the date (without time) by which the task should be completed. When absent, the task has no due date.

#### Scenario: Task with due date
- **WHEN** a task is created or edited with `--due 2025-06-01`
- **THEN** the task's `due_date` SHALL be set to 2025-06-01

#### Scenario: Task without due date
- **WHEN** a task is created without a `--due` flag
- **THEN** the task's `due_date` SHALL be `None`

#### Scenario: Invalid due date format
- **WHEN** the user provides `--due not-a-date`
- **THEN** the system SHALL exit with code 1 and display an error indicating the expected format (YYYY-MM-DD)

### Requirement: Project field
The `Task` struct SHALL include an optional `project` field of type `Option<String>`. When set, it groups the task under a named project. When absent, the task belongs to no project.

#### Scenario: Task with project
- **WHEN** a task is created or edited with `--project "Work"`
- **THEN** the task's `project` field SHALL be set to `"Work"`

#### Scenario: Task without project
- **WHEN** a task is created without a `--project` flag
- **THEN** the task's `project` field SHALL be `None`

## MODIFIED Requirements

### Requirement: Priority values
The system SHALL support four priority levels: `critical`, `high`, `medium`, `low`. Priority order from highest to lowest is: `critical > high > medium > low`. The default priority for new tasks SHALL be `medium`.

#### Scenario: Critical priority accepted
- **WHEN** the user provides `--priority critical`
- **THEN** the task SHALL be created or updated with priority `critical`

#### Scenario: Invalid priority
- **WHEN** the user provides a priority value not in {critical, high, medium, low}
- **THEN** the system SHALL exit with code 1 and display an error listing valid values
