## MODIFIED Requirements

### Requirement: Auth subcommand group
The CLI SHALL provide an `auth` subcommand with sub-subcommands `todoist`, `claude`, `status`, and `revoke`.

#### Scenario: Auth todoist invocation
- **WHEN** the user runs `task auth todoist`
- **THEN** the system SHALL prompt for a Todoist API token and store it

#### Scenario: Auth claude invocation
- **WHEN** the user runs `task auth claude`
- **THEN** the system SHALL prompt for a Claude API key and store it

#### Scenario: Auth claude with key flag
- **WHEN** the user runs `task auth claude --key sk-ant-...`
- **THEN** the system SHALL store the provided key without prompting

#### Scenario: Auth status invocation
- **WHEN** the user runs `task auth status`
- **THEN** the system SHALL report whether a Todoist token and Claude API key are stored

#### Scenario: Auth revoke invocation
- **WHEN** the user runs `task auth revoke`
- **THEN** the system SHALL delete both the stored Todoist token and Claude API key
