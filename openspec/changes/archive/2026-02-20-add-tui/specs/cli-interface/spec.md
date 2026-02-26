## MODIFIED Requirements

### Requirement: Subcommand structure
The CLI SHALL provide the following subcommands: `add`, `list`, `show`, `edit`, `done`, `undo`, `rm`, `tui`. Each subcommand SHALL support `--help` for usage information.

#### Scenario: Run with no subcommand
- **WHEN** the user runs `task` with no arguments
- **THEN** the system SHALL display a help summary listing all subcommands including `tui`

#### Scenario: Help flag on subcommand
- **WHEN** the user runs `task add --help`
- **THEN** the system SHALL display usage, arguments, and options for the `add` subcommand

#### Scenario: TUI help
- **WHEN** the user runs `task tui --help`
- **THEN** the system SHALL display usage information for the `tui` subcommand
