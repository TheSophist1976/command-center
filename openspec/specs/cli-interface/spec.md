## ADDED Requirements

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

### Requirement: JSON output mode
Every subcommand SHALL support a `--json` flag that switches output from human-readable text to structured JSON. JSON output SHALL always include a top-level `ok` boolean field.

#### Scenario: Successful operation with --json
- **WHEN** the user runs `task list --json` and the operation succeeds
- **THEN** the output SHALL be valid JSON with `{"ok": true, "tasks": [...]}`

#### Scenario: Failed operation with --json
- **WHEN** the user runs `task show 999 --json` and the task does not exist
- **THEN** the output SHALL be valid JSON with `{"ok": false, "error": "Task 999 not found"}`

#### Scenario: Default output without --json
- **WHEN** the user runs `task list` without `--json`
- **THEN** the output SHALL be human-readable formatted text (table or plain)

### Requirement: Human-readable default output
The default output for `list` SHALL be a columnar table showing ID, status, priority, title, and tags. Other subcommands SHALL output concise confirmation messages.

#### Scenario: List table output
- **WHEN** the user runs `task list` with multiple tasks
- **THEN** the output SHALL display tasks in aligned columns with headers

#### Scenario: Single task confirmation
- **WHEN** the user runs `task add "New task"`
- **THEN** the output SHALL display a confirmation like `Added task 5: New task`

### Requirement: File path configuration
The system SHALL resolve the task file path in this order: (1) `--file <path>` flag, (2) `TASK_FILE` environment variable, (3) `tasks.md` in the current working directory.

#### Scenario: Explicit --file flag
- **WHEN** the user runs `task list --file /path/to/my-tasks.md`
- **THEN** the system SHALL read from `/path/to/my-tasks.md`

#### Scenario: TASK_FILE environment variable
- **WHEN** `TASK_FILE=/tmp/tasks.md` is set and no `--file` flag is provided
- **THEN** the system SHALL read from `/tmp/tasks.md`

#### Scenario: Default file location
- **WHEN** no `--file` flag or `TASK_FILE` env var is set
- **THEN** the system SHALL read from `tasks.md` in the current working directory

#### Scenario: Flag overrides env var
- **WHEN** both `--file /a.md` and `TASK_FILE=/b.md` are set
- **THEN** the system SHALL use `/a.md` (flag takes precedence)

### Requirement: Exit codes
The system SHALL use consistent exit codes: 0 for success, 1 for errors (invalid input, file I/O failure), 2 for "not found" (task ID does not exist).

#### Scenario: Successful operation
- **WHEN** any command completes successfully
- **THEN** the process SHALL exit with code 0

#### Scenario: Invalid input
- **WHEN** the user provides invalid arguments (e.g., non-numeric ID, invalid priority)
- **THEN** the process SHALL exit with code 1

#### Scenario: Task not found
- **WHEN** a command references a task ID that does not exist
- **THEN** the process SHALL exit with code 2

### Requirement: Global --file flag
The `--file` flag SHALL be available as a global option on all subcommands, not per-subcommand.

#### Scenario: Global flag position
- **WHEN** the user runs `task --file custom.md list`
- **THEN** the system SHALL use `custom.md` as the task file for the list command

### Requirement: Init command
The system SHALL provide an `init` subcommand that creates a new task file with the format header and next-id counter.

#### Scenario: Init in empty directory
- **WHEN** the user runs `task init` and no `tasks.md` exists
- **THEN** the system SHALL create `tasks.md` with format version and next-id headers

#### Scenario: Init when file already exists
- **WHEN** the user runs `task init` and `tasks.md` already exists
- **THEN** the system SHALL exit with code 1 and display an error (will not overwrite)

### Requirement: Auto-init on first add
When adding a task, if the task file does not exist, the system SHALL automatically create it with proper headers before adding the task.

#### Scenario: Add task without existing file
- **WHEN** the user runs `task add "First task"` and no task file exists
- **THEN** the system SHALL create the file with headers and add the task in a single operation

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
The CLI SHALL provide an `auth` subcommand with sub-subcommands `todoist`, `claude`, `status`, and `revoke`.

#### Scenario: Auth todoist invocation
- **WHEN** the user runs `task auth todoist`
- **THEN** the system SHALL prompt for a Todoist API token and store it

#### Scenario: Auth claude invocation
- **WHEN** the user runs `task auth claude`
- **THEN** the system SHALL prompt for a Claude API key and store it

#### Scenario: Auth claude with key flag
- **WHEN** the user runs `task auth claude --key sk-ant-...`
- **THEN** the system SHALL store the provided key without prompting

#### Scenario: Auth status invocation
- **WHEN** the user runs `task auth status`
- **THEN** the system SHALL report whether a Todoist token and Claude API key are stored

#### Scenario: Auth revoke invocation
- **WHEN** the user runs `task auth revoke`
- **THEN** the system SHALL delete both the stored Todoist token and Claude API key

### Requirement: Migrate subcommand
The CLI SHALL provide a `migrate` subcommand that upgrades the task file to the latest format version.

#### Scenario: Migrate invocation
- **WHEN** the user runs `task migrate`
- **THEN** the system SHALL load the task file and save it at format version 2 without changing task data
