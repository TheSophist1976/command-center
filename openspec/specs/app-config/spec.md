### Requirement: Config file location
The system SHALL store application configuration at `{config_dir}/task-manager/config.md`, where `{config_dir}` is the platform config directory returned by `dirs::config_dir()`. If `config_dir()` returns `None`, the system SHALL behave as if no config file exists.

#### Scenario: Config path on macOS
- **WHEN** the system resolves the config file path on macOS
- **THEN** the path SHALL be `~/Library/Application Support/task-manager/config.md`

#### Scenario: No config dir available
- **WHEN** `dirs::config_dir()` returns `None`
- **THEN** the system SHALL treat the config as unset and fall through to the next file path resolution tier

### Requirement: Config file format
The config file SHALL be a plain markdown file. Each configuration key SHALL appear on its own line in the format `key: value`. Lines that do not match this format SHALL be ignored. The file is human-readable and editable by hand.

#### Scenario: Read a valid config file
- **WHEN** the config file contains `default-dir: /Users/alice/notes`
- **THEN** the system SHALL return `/Users/alice/notes` as the value for `default-dir`

#### Scenario: Ignore unrecognised lines
- **WHEN** the config file contains comment lines (e.g., `# task-manager config`) or blank lines
- **THEN** the system SHALL ignore those lines without error

#### Scenario: Key not present
- **WHEN** the config file exists but does not contain a line for the requested key
- **THEN** the system SHALL return no value for that key

### Requirement: default-dir config key
The `default-dir` key SHALL store the directory path from which `tasks.md` is loaded when no explicit path is provided. The value SHALL be used as the directory; `tasks.md` is appended to form the full file path.

#### Scenario: default-dir is set
- **WHEN** `default-dir` is set to `/home/user/notes` and no `--file` flag or `TASK_FILE` env var is given
- **THEN** the system SHALL resolve the task file path as `/home/user/notes/tasks.md`

#### Scenario: default-dir is not set
- **WHEN** `default-dir` is not set in the config file
- **THEN** the task file path SHALL fall through to `"tasks.md"` (current directory)

### Requirement: Config CLI subcommand
The system SHALL provide a `task config` subcommand with `set` and `get` operations for managing configuration values.

#### Scenario: Set a config value
- **WHEN** the user runs `task config set default-dir /home/user/notes`
- **THEN** the system SHALL write `default-dir: /home/user/notes` to the config file and print a confirmation message

#### Scenario: Get a config value that is set
- **WHEN** the user runs `task config get default-dir` and the key exists
- **THEN** the system SHALL print the stored value

#### Scenario: Get a config value that is not set
- **WHEN** the user runs `task config get default-dir` and the key is not present in the config file
- **THEN** the system SHALL print a message indicating the value is not set and exit with code 0

#### Scenario: Set creates config directory if absent
- **WHEN** the config directory does not exist and the user runs `task config set default-dir <path>`
- **THEN** the system SHALL create the config directory before writing the config file

### Requirement: File path resolution order
The system SHALL resolve the task file path using the following priority order: (1) `--file` CLI flag, (2) `TASK_FILE` environment variable, (3) `default-dir` from config file, (4) `"tasks.md"` in the current directory.

#### Scenario: CLI flag takes highest priority
- **WHEN** `--file custom.md` is provided and `default-dir` is also set
- **THEN** the system SHALL use `custom.md` and ignore the config value

#### Scenario: Env var takes priority over config
- **WHEN** `TASK_FILE=/tmp/tasks.md` is set and `default-dir` is also configured
- **THEN** the system SHALL use `/tmp/tasks.md` and ignore the config value

#### Scenario: Config value used when no flag or env var
- **WHEN** no `--file` flag and no `TASK_FILE` env var are set, and `default-dir` is configured
- **THEN** the system SHALL use `<default-dir>/tasks.md`

#### Scenario: Hardcoded fallback when nothing is configured
- **WHEN** no flag, no env var, and no config value are present
- **THEN** the system SHALL use `"tasks.md"` in the current directory
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

### Requirement: agent-* config key family
The config system SHALL support keys with the prefix `agent-` (e.g., `agent-command-center`) for storing agent profile working directories. These keys SHALL follow the same `key: value` format as all other config entries and SHALL be readable and writable via the existing `read_config_value` / `write_config_value` interface.

#### Scenario: Write and read an agent profile
- **WHEN** `write_config_value("agent-command-center", "~/code/command-center")` is called
- **THEN** the config file SHALL contain `agent-command-center: ~/code/command-center` and `read_config_value("agent-command-center")` SHALL return `~/code/command-center`

### Requirement: Enumerate all agent profiles from config
The config module SHALL expose a function `list_agent_profiles()` that reads the config file and returns all entries whose key starts with `agent-`, as a `Vec<(String, String)>` of `(profile_name, dir)` pairs where `profile_name` is the key with the `agent-` prefix stripped.

#### Scenario: Two profiles in config
- **WHEN** config contains `agent-alpha: /code/alpha` and `agent-beta: /code/beta`
- **THEN** `list_agent_profiles()` SHALL return `[("alpha", "/code/alpha"), ("beta", "/code/beta")]` (order may vary)

#### Scenario: No agent profiles in config
- **WHEN** no `agent-*` keys exist in config
- **THEN** `list_agent_profiles()` SHALL return an empty vec
