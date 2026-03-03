## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: JSON output mode
**Reason**: No CLI commands remain that produce output requiring JSON formatting. The TUI has its own display.
**Migration**: Use the TUI for all task operations.

### Requirement: Human-readable default output
**Reason**: CLI list/show commands removed. The TUI renders its own table.
**Migration**: Use the TUI.

### Requirement: Global --file flag
**Reason**: Merged into the simplified "File path configuration" requirement. The `--file` flag remains but `--json` and `--strict` are removed.
**Migration**: `--file` still works. `--json` and `--strict` are no longer available.

### Requirement: Init command
**Reason**: The TUI auto-creates the task file on first launch via `storage::load`.
**Migration**: Just run `task` to start.

### Requirement: Auto-init on first add
**Reason**: CLI `add` command removed.
**Migration**: The TUI handles file creation.

### Requirement: Due date flag on add and edit
**Reason**: CLI `add`/`edit` commands removed. Due dates set via TUI keybindings (T/Y/W/M/Q) or NLP.
**Migration**: Use TUI.

### Requirement: Project flag on add, edit, and list
**Reason**: CLI `add`/`edit`/`list` commands removed. Projects managed via TUI detail panel or NLP.
**Migration**: Use TUI.

### Requirement: Import subcommand group
**Reason**: Import available via `i` keybinding in TUI.
**Migration**: Use TUI `i` key or NLP chat.

### Requirement: Migrate subcommand
**Reason**: Format migration can be handled internally by the TUI/storage layer if needed.
**Migration**: Not needed for normal operation.

### Requirement: Recur flag on add command
**Reason**: CLI `add` command removed. Recurrence set via TUI `R` keybinding or NLP.
**Migration**: Use TUI.

### Requirement: Recur flag on edit command
**Reason**: CLI `edit` command removed. Recurrence managed via TUI `R` keybinding or NLP.
**Migration**: Use TUI.
