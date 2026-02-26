## REMOVED Requirements

### Requirement: OAuth 2.0 browser authorization flow
Removed. The browser-based OAuth flow is replaced by a personal API token prompt.

### Requirement: Token exchange
Removed. There is no authorization code to exchange; the user provides the token directly.

### Requirement: OAuth credentials via environment variables
Removed. `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET` are no longer required.

## MODIFIED Requirements

### Requirement: Personal API token prompt
The `task auth todoist` subcommand SHALL accept the user's Todoist personal API token either interactively via stdin or non-interactively via the `--token <value>` CLI flag.

#### Scenario: Interactive prompt
- **WHEN** the user runs `task auth todoist` without `--token`
- **THEN** the system SHALL print the URL to the Todoist developer settings page and prompt the user to paste their token, then read a line from stdin

#### Scenario: Non-interactive flag
- **WHEN** the user runs `task auth todoist --token <value>`
- **THEN** the system SHALL use that value directly without prompting

#### Scenario: Empty token rejected
- **WHEN** the provided token (from stdin or `--token`) is empty or whitespace-only
- **THEN** the system SHALL exit with code 1 and display an error

#### Scenario: Token stored on success
- **WHEN** a non-empty token is provided
- **THEN** the system SHALL write it to the token file and print a confirmation message

## UNCHANGED Requirements

### Requirement: Token persistence
Unchanged — token stored at `{config_dir}/task-manager/todoist_token` with 0600 permissions.

### Requirement: Auth status subcommand
Unchanged.

### Requirement: Auth revoke subcommand
Unchanged.
