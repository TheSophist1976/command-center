## MODIFIED Requirements

### Requirement: Subcommand structure
The CLI SHALL provide the following subcommands: `tui`, `auth`, `config`. Running `task` with no subcommand SHALL launch the TUI (equivalent to `task tui`). Each subcommand SHALL support `--help` for usage information. The CLI SHALL expose `--version` / `-V` flags that print the current version and exit.

#### Scenario: Run with no subcommand
- **WHEN** the user runs `task` with no arguments
- **THEN** the system SHALL launch the TUI

#### Scenario: Explicit tui subcommand
- **WHEN** the user runs `task tui`
- **THEN** the system SHALL launch the TUI

#### Scenario: Help flag
- **WHEN** the user runs `task --help`
- **THEN** the system SHALL display a help summary listing `tui`, `auth`, and `config` subcommands

#### Scenario: Version flag
- **WHEN** the user runs `task --version` or `task -V`
- **THEN** the system SHALL print `task <semver>` (e.g., `task 0.2.0`) and exit with code 0
