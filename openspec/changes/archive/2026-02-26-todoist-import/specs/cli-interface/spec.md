## ADDED Requirements

### Requirement: Due date flag on add and edit
The `add` and `edit` subcommands SHALL accept a `--due <date>` flag that sets the task's `due_date` field. The date SHALL be provided in `YYYY-MM-DD` format.

#### Scenario: Add task with due date
- **WHEN** the user runs `task add "Deploy v2" --due 2025-06-01`
- **THEN** the new task SHALL have `due_date` set to 2025-06-01

#### Scenario: Edit task due date
- **WHEN** the user runs `task edit 3 --due 2025-07-15`
- **THEN** task 3's `due_date` SHALL be updated to 2025-07-15

### Requirement: Project flag on add, edit, and list
The `add` and `edit` subcommands SHALL accept a `--project <name>` flag that sets the task's `project` field. The `list` subcommand SHALL accept a `--project <name>` flag that filters tasks to those matching the given project name.

#### Scenario: Add task with project
- **WHEN** the user runs `task add "Write tests" --project "Backend"`
- **THEN** the new task SHALL have `project` set to `"Backend"`

#### Scenario: Edit task project
- **WHEN** the user runs `task edit 3 --project "Frontend"`
- **THEN** task 3's `project` field SHALL be updated to `"Frontend"`

#### Scenario: Filter list by project
- **WHEN** the user runs `task list --project "Backend"`
- **THEN** only tasks with `project` equal to `"Backend"` SHALL be displayed

### Requirement: Import subcommand group
The CLI SHALL provide an `import` subcommand with a `todoist` sub-subcommand that triggers the Todoist import flow.

#### Scenario: Import todoist invocation
- **WHEN** the user runs `task import todoist`
- **THEN** the system SHALL run the Todoist import flow

#### Scenario: Import help
- **WHEN** the user runs `task import --help`
- **THEN** the system SHALL display available import sub-subcommands including `todoist`

### Requirement: Auth subcommand group
The CLI SHALL provide an `auth` subcommand with sub-subcommands `todoist`, `status`, and `revoke`.

#### Scenario: Auth todoist invocation
- **WHEN** the user runs `task auth todoist`
- **THEN** the system SHALL initiate the OAuth 2.0 browser flow

#### Scenario: Auth status invocation
- **WHEN** the user runs `task auth status`
- **THEN** the system SHALL report whether a Todoist token is stored

#### Scenario: Auth revoke invocation
- **WHEN** the user runs `task auth revoke`
- **THEN** the system SHALL delete the stored token

### Requirement: Migrate subcommand
The CLI SHALL provide a `migrate` subcommand that upgrades the task file to the latest format version.

#### Scenario: Migrate invocation
- **WHEN** the user runs `task migrate`
- **THEN** the system SHALL load the task file and save it at format version 2 without changing task data

## MODIFIED Requirements

### Requirement: Subcommand structure
The CLI SHALL provide the following subcommands: `add`, `list`, `show`, `edit`, `done`, `undo`, `rm`, `init`, `tui`, `import`, `auth`, `migrate`. Each subcommand SHALL support `--help` for usage information.

#### Scenario: Run with no subcommand
- **WHEN** the user runs `task` with no arguments
- **THEN** the system SHALL display a help summary listing all subcommands including `tui`, `import`, `auth`, and `migrate`

#### Scenario: Help flag on subcommand
- **WHEN** the user runs `task add --help`
- **THEN** the system SHALL display usage, arguments, and options for the `add` subcommand

#### Scenario: TUI help
- **WHEN** the user runs `task tui --help`
- **THEN** the system SHALL display usage information for the `tui` subcommand
