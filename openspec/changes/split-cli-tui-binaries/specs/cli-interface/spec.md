## MODIFIED Requirements

### Requirement: CLI binary default behaviour
When the `task` binary is invoked with no subcommand, it SHALL print help output (via clap's built-in help) and exit with code 0. It SHALL NOT launch the TUI. The TUI SHALL only be accessible via the `task-tui` binary.

#### Scenario: task with no arguments prints help
- **WHEN** the user runs `task` with no arguments
- **THEN** the CLI SHALL print the help text listing available subcommands and exit with code 0

#### Scenario: task-tui with no arguments launches TUI
- **WHEN** the user runs `task-tui` with no arguments
- **THEN** the TUI SHALL launch as before

### Requirement: CLI binary subcommands
The `task` binary SHALL expose the `auth`, `config`, and `note` subcommands with identical behaviour to the current single binary. The `tui` subcommand SHALL be present only in the `task-tui` binary.

#### Scenario: task auth subcommands work
- **WHEN** the user runs `task auth status`
- **THEN** the authentication status SHALL be printed as before

#### Scenario: task config subcommands work
- **WHEN** the user runs `task config get <key>`
- **THEN** the config value SHALL be printed as before

#### Scenario: task note subcommands work
- **WHEN** the user runs `task note list`
- **THEN** the note list SHALL be printed as before
