## MODIFIED Requirements

### Requirement: Subcommand structure
The CLI SHALL provide the following subcommands: `tui`, `auth`, `config`, `note`. Running `task` with no subcommand SHALL launch the TUI (equivalent to `task tui`). Each subcommand SHALL support `--help` for usage information.

#### Scenario: Run with no subcommand
- **WHEN** the user runs `task` with no arguments
- **THEN** the system SHALL launch the TUI

#### Scenario: Explicit tui subcommand
- **WHEN** the user runs `task tui`
- **THEN** the system SHALL launch the TUI

#### Scenario: Help flag
- **WHEN** the user runs `task --help`
- **THEN** the system SHALL display a help summary listing `tui`, `auth`, `config`, and `note` subcommands
