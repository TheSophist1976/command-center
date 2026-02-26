## MODIFIED Requirements

### Requirement: TUI entry point
The system SHALL provide a `task tui` subcommand that launches a full-screen terminal interface. The TUI SHALL take ownership of the terminal using crossterm's alternate screen and raw mode. On exit, the terminal SHALL be restored to its original state. When no `--file` flag or `TASK_FILE` env var is given, the TUI SHALL load tasks from the path resolved by `storage::resolve_file_path`, which includes the `default-dir` app config value.

#### Scenario: Launch TUI
- **WHEN** the user runs `task tui`
- **THEN** the system SHALL enter alternate screen, enable raw mode, and display the TUI dashboard

#### Scenario: Quit TUI
- **WHEN** the user presses `q` in normal mode
- **THEN** the system SHALL restore the terminal to its original state and exit with code 0

#### Scenario: Panic recovery
- **WHEN** the TUI encounters a panic during execution
- **THEN** the system SHALL restore raw mode and alternate screen before printing the panic message

#### Scenario: Launch with configured default directory
- **WHEN** `default-dir` is set in the app config and the user runs `task tui` with no `--file` flag
- **THEN** the TUI SHALL load tasks from `<default-dir>/tasks.md`

## ADDED Requirements

### Requirement: Set default directory from TUI
The user SHALL press `D` in normal mode to enter directory-setting mode. The footer SHALL display a text input prompt pre-populated with the current default directory (empty if not set). The user SHALL type a directory path and press `Enter` to confirm or `Esc` to cancel. On confirm, the system SHALL write the new value to the app config file and reload tasks from `<new-dir>/tasks.md`. Any unsaved in-memory state SHALL be saved before reloading. The `D` key SHALL be a no-op when the TUI is in any non-Normal mode.

#### Scenario: Set default directory
- **WHEN** the user presses `D`, types `/home/user/notes`, and presses `Enter`
- **THEN** the system SHALL save the current task state, write `default-dir: /home/user/notes` to the config file, and reload tasks from `/home/user/notes/tasks.md`

#### Scenario: Cancel setting default directory
- **WHEN** the user presses `D` and then presses `Esc`
- **THEN** the config SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: Footer hint updated
- **WHEN** the TUI is in normal mode
- **THEN** the footer SHALL include `D:set-dir` in its keybinding hints
