## ADDED Requirements

### Requirement: claude-code-dir config key
The `claude-code-dir` key SHALL store the root directory whose immediate subdirectories are listed in the Claude session directory picker. If not set, the system SHALL default to `~/code` (tilde expanded using the platform home directory). The value SHALL be read at picker-open time, not at TUI startup, so changes take effect without restarting.

#### Scenario: claude-code-dir is set
- **WHEN** `claude-code-dir` is set to `/home/user/projects` in the config file
- **THEN** the directory picker SHALL list immediate subdirectories of `/home/user/projects`

#### Scenario: claude-code-dir is not set
- **WHEN** `claude-code-dir` is absent from the config file
- **THEN** the directory picker SHALL default to `~/code` (expanded to the user's home directory)

#### Scenario: Set via config subcommand
- **WHEN** the user runs `task config set claude-code-dir /workspace`
- **THEN** the system SHALL write `claude-code-dir: /workspace` to the config file and the picker SHALL use `/workspace` on next open
