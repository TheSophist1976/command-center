## ADDED Requirements

### Requirement: Slack token file path
The system SHALL store the Slack Bot User OAuth Token at `<config_dir>/task-manager/slack_token`, where `<config_dir>` is resolved by `dirs::config_dir()`. The file path function SHALL be named `slack_token_path()` and SHALL return a `PathBuf`.

#### Scenario: Token path on macOS
- **WHEN** the system resolves the Slack token path on macOS
- **THEN** the path SHALL be `~/Library/Application Support/task-manager/slack_token`

#### Scenario: Token path on Linux
- **WHEN** the system resolves the Slack token path on Linux with no `XDG_CONFIG_HOME`
- **THEN** the path SHALL be `~/.config/task-manager/slack_token`

### Requirement: Read Slack token
The system SHALL provide a `read_slack_token()` function that returns `Option<String>`. It SHALL first check the `SLACK_BOT_TOKEN` environment variable; if set and non-empty, return that value. Otherwise, it SHALL read the token file at `slack_token_path()`, trim whitespace, and return the value if the file exists and is non-empty.

#### Scenario: Token from environment variable
- **WHEN** `SLACK_BOT_TOKEN` is set to `xoxb-test-token`
- **THEN** `read_slack_token()` SHALL return `Some("xoxb-test-token")`

#### Scenario: Token from file
- **WHEN** `SLACK_BOT_TOKEN` is not set and the token file contains `xoxb-file-token\n`
- **THEN** `read_slack_token()` SHALL return `Some("xoxb-file-token")`

#### Scenario: No token configured
- **WHEN** `SLACK_BOT_TOKEN` is not set and no token file exists
- **THEN** `read_slack_token()` SHALL return `None`

### Requirement: Write Slack token
The system SHALL provide a `write_slack_token(token: &str)` function that creates the parent directory if needed, writes the token to `slack_token_path()`, and sets file permissions to `0600` (owner read/write only).

#### Scenario: Write creates parent directory
- **WHEN** the config directory does not exist and `write_slack_token("xoxb-new")` is called
- **THEN** the parent directory SHALL be created and the token file SHALL contain `xoxb-new` with `0600` permissions

### Requirement: Prompt for Slack token
The system SHALL provide a `prompt_for_slack_token(token_flag: Option<String>)` function. If `token_flag` is `Some`, it SHALL validate that the value is non-empty and starts with `xoxb-`, then return it. If `None`, it SHALL print setup instructions including a link to `https://api.slack.com/apps` and the required scopes (`channels:history`, `channels:read`), then read a line from stdin.

#### Scenario: Token provided via flag
- **WHEN** `prompt_for_slack_token(Some("xoxb-abc123"))` is called
- **THEN** it SHALL return `Ok("xoxb-abc123")` without prompting

#### Scenario: Invalid token prefix
- **WHEN** `prompt_for_slack_token(Some("not-a-bot-token"))` is called
- **THEN** it SHALL return an error indicating the token must start with `xoxb-`

#### Scenario: Empty token rejected
- **WHEN** `prompt_for_slack_token(Some(""))` is called
- **THEN** it SHALL return an error indicating the token cannot be empty

### Requirement: Delete Slack token
The system SHALL provide a `delete_slack_token()` function that removes the token file at `slack_token_path()` if it exists. It SHALL return `Ok(true)` if a file was deleted, `Ok(false)` if no file existed.

#### Scenario: Delete existing token
- **WHEN** a Slack token file exists and `delete_slack_token()` is called
- **THEN** the file SHALL be removed and the function SHALL return `Ok(true)`

#### Scenario: Delete non-existent token
- **WHEN** no Slack token file exists and `delete_slack_token()` is called
- **THEN** the function SHALL return `Ok(false)`
