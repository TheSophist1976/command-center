## ADDED Requirements

### Requirement: Claude API key storage
The system SHALL store the Claude API key at `{config_dir}/task-manager/claude_api_key` where `config_dir` is resolved via `dirs::config_dir()`. The file SHALL be created with 0600 permissions (owner read/write only). The system SHALL provide `read_claude_key()`, `write_claude_key()`, and `delete_claude_key()` functions in the `auth` module.

#### Scenario: Write and read key
- **WHEN** `write_claude_key("sk-ant-...")` is called
- **THEN** the key SHALL be written to `{config_dir}/task-manager/claude_api_key` with 0600 permissions and `read_claude_key()` SHALL return `Some("sk-ant-...")`

#### Scenario: Read when no key stored
- **WHEN** no `claude_api_key` file exists
- **THEN** `read_claude_key()` SHALL return `None`

#### Scenario: Delete key
- **WHEN** `delete_claude_key()` is called and a key file exists
- **THEN** the file SHALL be deleted and `read_claude_key()` SHALL return `None`

### Requirement: Environment variable fallback
The system SHALL check the `ANTHROPIC_API_KEY` environment variable as a fallback when no stored key file exists. If both are present, the env var SHALL take precedence.

#### Scenario: Env var present, no file
- **WHEN** `ANTHROPIC_API_KEY` is set and no key file exists
- **THEN** `read_claude_key()` SHALL return the env var value

#### Scenario: Both present, env var wins
- **WHEN** `ANTHROPIC_API_KEY` is set and a key file also exists
- **THEN** `read_claude_key()` SHALL return the env var value

#### Scenario: Neither present
- **WHEN** `ANTHROPIC_API_KEY` is not set and no key file exists
- **THEN** `read_claude_key()` SHALL return `None`

### Requirement: Auth claude CLI subcommand
The `task auth claude` subcommand SHALL prompt the user to paste their Claude API key. An optional `--key` flag SHALL allow passing the key non-interactively. The key SHALL be stored via `write_claude_key()`.

#### Scenario: Interactive key entry
- **WHEN** the user runs `task auth claude` with no flags
- **THEN** the system SHALL prompt "Paste your Claude API key:" and store the entered key

#### Scenario: Non-interactive key entry
- **WHEN** the user runs `task auth claude --key sk-ant-...`
- **THEN** the system SHALL store the provided key without prompting

#### Scenario: Empty key rejected
- **WHEN** the user provides an empty string as the key
- **THEN** the system SHALL exit with code 1 and display "API key cannot be empty."

### Requirement: Auth status includes Claude key
The `task auth status` subcommand SHALL report the status of both the Todoist token and the Claude API key.

#### Scenario: Both present
- **WHEN** both a Todoist token and Claude API key are stored
- **THEN** the output SHALL include "Todoist token: present" and "Claude API key: present"

#### Scenario: Claude key from env var
- **WHEN** the Claude API key comes from `ANTHROPIC_API_KEY` env var
- **THEN** the output SHALL include "Claude API key: present (env)"

#### Scenario: Claude key absent
- **WHEN** no Claude API key is stored and `ANTHROPIC_API_KEY` is not set
- **THEN** the output SHALL include "Claude API key: not set"

### Requirement: Auth revoke includes Claude key
The `task auth revoke` subcommand SHALL delete both the Todoist token and the Claude API key file (if present).

#### Scenario: Revoke deletes Claude key
- **WHEN** the user runs `task auth revoke` and a Claude API key file exists
- **THEN** the Claude API key file SHALL be deleted alongside the Todoist token
