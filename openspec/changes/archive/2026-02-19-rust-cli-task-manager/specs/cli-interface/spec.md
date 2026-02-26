## ADDED Requirements

### Requirement: Subcommand structure
The CLI SHALL provide the following subcommands: `add`, `list`, `show`, `edit`, `done`, `undo`, `rm`. Each subcommand SHALL support `--help` for usage information.

#### Scenario: Run with no subcommand
- **WHEN** the user runs `task` with no arguments
- **THEN** the system SHALL display a help summary listing all subcommands

#### Scenario: Help flag on subcommand
- **WHEN** the user runs `task add --help`
- **THEN** the system SHALL display usage, arguments, and options for the `add` subcommand

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
