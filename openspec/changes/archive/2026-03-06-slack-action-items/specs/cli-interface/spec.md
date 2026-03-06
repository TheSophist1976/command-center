## MODIFIED Requirements

### Requirement: Auth subcommand group
The CLI SHALL provide an `auth` subcommand with sub-subcommands `todoist`, `claude`, `slack`, `status`, and `revoke`.

#### Scenario: Auth todoist invocation
- **WHEN** the user runs `task auth todoist`
- **THEN** the system SHALL prompt for a Todoist API token and store it

#### Scenario: Auth claude invocation
- **WHEN** the user runs `task auth claude`
- **THEN** the system SHALL prompt for a Claude API key and store it

#### Scenario: Auth claude with key flag
- **WHEN** the user runs `task auth claude --key sk-ant-...`
- **THEN** the system SHALL store the provided key without prompting

#### Scenario: Auth slack invocation
- **WHEN** the user runs `task auth slack`
- **THEN** the system SHALL prompt for a Slack Bot User OAuth Token, validate it starts with `xoxb-`, and store it

#### Scenario: Auth slack with token flag
- **WHEN** the user runs `task auth slack --token xoxb-123-456`
- **THEN** the system SHALL store the provided token without prompting

#### Scenario: Auth status invocation
- **WHEN** the user runs `task auth status`
- **THEN** the system SHALL report whether a Todoist token, Claude API key, and Slack token are stored

#### Scenario: Auth revoke invocation
- **WHEN** the user runs `task auth revoke`
- **THEN** the system SHALL delete the stored Todoist token, Claude API key, and Slack token
