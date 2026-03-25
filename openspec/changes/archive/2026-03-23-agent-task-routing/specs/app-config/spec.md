## ADDED Requirements

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
