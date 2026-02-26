## ADDED Requirements

### Requirement: Add a task
The system SHALL create a new task with a title, optional priority, and optional tags. The task SHALL be assigned the next available integer ID and appended to the task file.

#### Scenario: Add task with title only
- **WHEN** the user runs `task add "Build the login page"`
- **THEN** a new open task with that title SHALL be appended to the file with default priority ("medium"), no tags, and a created timestamp

#### Scenario: Add task with priority and tags
- **WHEN** the user runs `task add "Build the login page" --priority high --tags frontend,auth`
- **THEN** the task SHALL be created with priority "high" and tags ["frontend", "auth"]

#### Scenario: Add task outputs the new task
- **WHEN** a task is successfully added
- **THEN** the system SHALL output the new task's ID and title to confirm creation

### Requirement: List tasks
The system SHALL display all tasks, with the ability to filter by status, priority, and tags.

#### Scenario: List all tasks
- **WHEN** the user runs `task list`
- **THEN** the system SHALL display all tasks showing ID, status, priority, title, and tags

#### Scenario: Filter by status
- **WHEN** the user runs `task list --status open`
- **THEN** only tasks with status "open" SHALL be displayed

#### Scenario: Filter by priority
- **WHEN** the user runs `task list --priority high`
- **THEN** only tasks with priority "high" SHALL be displayed

#### Scenario: Filter by tag
- **WHEN** the user runs `task list --tag frontend`
- **THEN** only tasks that include the tag "frontend" SHALL be displayed

#### Scenario: Combined filters
- **WHEN** multiple filter flags are provided
- **THEN** the system SHALL apply all filters (AND logic) and display only matching tasks

#### Scenario: No matching tasks
- **WHEN** filters match no tasks
- **THEN** the system SHALL display an empty result without error

### Requirement: Show task detail
The system SHALL display full details of a single task by its ID, including description body if present.

#### Scenario: Show existing task
- **WHEN** the user runs `task show 3`
- **THEN** the system SHALL display task 3's title, status, priority, tags, timestamps, and description

#### Scenario: Show nonexistent task
- **WHEN** the user runs `task show 999` and no task with that ID exists
- **THEN** the system SHALL exit with code 2 and display an error message

### Requirement: Edit a task
The system SHALL allow updating a task's title, priority, and tags by ID.

#### Scenario: Edit task title
- **WHEN** the user runs `task edit 3 --title "Redesign the login page"`
- **THEN** task 3's title SHALL be updated and the updated timestamp SHALL be set

#### Scenario: Edit task priority
- **WHEN** the user runs `task edit 3 --priority low`
- **THEN** task 3's priority SHALL be updated to "low"

#### Scenario: Edit task tags
- **WHEN** the user runs `task edit 3 --tags backend,api`
- **THEN** task 3's tags SHALL be replaced with ["backend", "api"]

#### Scenario: Edit nonexistent task
- **WHEN** the user runs `task edit 999 --title "foo"` and no task with that ID exists
- **THEN** the system SHALL exit with code 2 and display an error message

### Requirement: Complete a task
The system SHALL mark a task as done by its ID, changing its checkbox from `[ ]` to `[x]`.

#### Scenario: Complete an open task
- **WHEN** the user runs `task done 3` and task 3 is open
- **THEN** task 3's status SHALL change to "done" and the updated timestamp SHALL be set

#### Scenario: Complete an already-done task
- **WHEN** the user runs `task done 3` and task 3 is already done
- **THEN** the system SHALL output a message indicating the task is already complete (no error)

### Requirement: Reopen a task
The system SHALL mark a completed task as open by its ID, changing its checkbox from `[x]` to `[ ]`.

#### Scenario: Reopen a done task
- **WHEN** the user runs `task undo 3` and task 3 is done
- **THEN** task 3's status SHALL change to "open" and the updated timestamp SHALL be set

#### Scenario: Reopen an already-open task
- **WHEN** the user runs `task undo 3` and task 3 is already open
- **THEN** the system SHALL output a message indicating the task is already open (no error)

### Requirement: Delete a task
The system SHALL remove a task from the file by its ID.

#### Scenario: Delete existing task
- **WHEN** the user runs `task rm 3` and task 3 exists
- **THEN** task 3 SHALL be removed from the file entirely

#### Scenario: Delete nonexistent task
- **WHEN** the user runs `task rm 999` and no task with that ID exists
- **THEN** the system SHALL exit with code 2 and display an error message

### Requirement: Priority values
The system SHALL support four priority levels: `critical`, `high`, `medium`, `low`. Priority order from highest to lowest is: `critical > high > medium > low`. The default priority for new tasks SHALL be `medium`.

#### Scenario: Critical priority accepted
- **WHEN** the user provides `--priority critical`
- **THEN** the task SHALL be created or updated with priority `critical`

#### Scenario: Invalid priority
- **WHEN** the user provides a priority value not in {critical, high, medium, low}
- **THEN** the system SHALL exit with code 1 and display an error listing valid values

### Requirement: Tag format
Tags SHALL be comma-separated strings with no spaces. Each tag SHALL consist of lowercase alphanumeric characters and hyphens.

#### Scenario: Valid tags
- **WHEN** the user provides `--tags frontend,auth,api-v2`
- **THEN** the system SHALL accept and store tags ["frontend", "auth", "api-v2"]

#### Scenario: Invalid tag characters
- **WHEN** the user provides a tag containing spaces or special characters
- **THEN** the system SHALL exit with code 1 and display an error describing valid tag format

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
