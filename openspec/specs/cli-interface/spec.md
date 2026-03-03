## ADDED Requirements

### Requirement: Subcommand structure
The CLI SHALL provide the following subcommands: `tui`, `auth`, `config`. Running `task` with no subcommand SHALL launch the TUI (equivalent to `task tui`). Each subcommand SHALL support `--help` for usage information.

#### Scenario: Run with no subcommand
- **WHEN** the user runs `task` with no arguments
- **THEN** the system SHALL launch the TUI

#### Scenario: Explicit tui subcommand
- **WHEN** the user runs `task tui`
- **THEN** the system SHALL launch the TUI

#### Scenario: Help flag
- **WHEN** the user runs `task --help`
- **THEN** the system SHALL display a help summary listing `tui`, `auth`, and `config` subcommands

### Requirement: File path configuration
The system SHALL resolve the task file path in this order: (1) `--file <path>` flag, (2) `TASK_FILE` environment variable, (3) `tasks.md` in the configured default directory or current working directory.

#### Scenario: Explicit --file flag
- **WHEN** the user runs `task --file /path/to/my-tasks.md`
- **THEN** the TUI SHALL use `/path/to/my-tasks.md` as the task file

#### Scenario: Default file location
- **WHEN** no `--file` flag or `TASK_FILE` env var is set
- **THEN** the TUI SHALL use `tasks.md` in the configured default directory or current working directory

### Requirement: Exit codes
The system SHALL use consistent exit codes: 0 for success, 1 for errors.

#### Scenario: Successful operation
- **WHEN** any command completes successfully
- **THEN** the process SHALL exit with code 0

#### Scenario: Error
- **WHEN** an error occurs (invalid input, file I/O failure)
- **THEN** the process SHALL exit with code 1

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
