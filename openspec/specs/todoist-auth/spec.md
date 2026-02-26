### Requirement: OAuth 2.0 browser authorization flow
The `task auth todoist` subcommand SHALL initiate an OAuth 2.0 authorization code flow by opening the Todoist authorization URL in the system browser, spinning up a local HTTP callback server on `127.0.0.1:7777`, and waiting for the redirect with the authorization code.

#### Scenario: Successful authorization
- **WHEN** the user runs `task auth todoist` and completes the browser authorization
- **THEN** the system SHALL capture the authorization code from the redirect, exchange it for an access token, and store the token locally

#### Scenario: Browser launch
- **WHEN** the user runs `task auth todoist`
- **THEN** the system SHALL open the Todoist OAuth authorization URL in the default system browser

#### Scenario: User cancels or browser not opened
- **WHEN** the authorization callback is not received within a reasonable timeout
- **THEN** the system SHALL exit with code 1 and display an appropriate error message

### Requirement: Token exchange
The system SHALL exchange the authorization code for an access token by calling the Todoist token endpoint with the client ID, client secret, and authorization code.

#### Scenario: Successful token exchange
- **WHEN** the authorization code is received from the callback
- **THEN** the system SHALL POST to the Todoist token endpoint and receive an access token

#### Scenario: Token exchange failure
- **WHEN** the token exchange request fails (e.g., invalid code, network error)
- **THEN** the system SHALL exit with code 1 and display an error describing the failure

### Requirement: Token persistence
The system SHALL store the access token at `{config_dir}/task-manager/todoist_token` (platform config directory via `dirs::config_dir()`). The token file SHALL be created with 0600 permissions (owner read/write only).

#### Scenario: Token stored
- **WHEN** token exchange succeeds
- **THEN** the system SHALL write the token to the configured path and set file permissions to 0600

#### Scenario: Config directory created if absent
- **WHEN** the config directory does not exist
- **THEN** the system SHALL create it before writing the token file

### Requirement: Auth status subcommand
The `task auth status` subcommand SHALL report whether a Todoist token is currently stored.

#### Scenario: Token present
- **WHEN** the user runs `task auth status` and a token file exists
- **THEN** the system SHALL output a message indicating a token is stored (e.g., `Todoist token: present`)

#### Scenario: Token absent
- **WHEN** the user runs `task auth status` and no token file exists
- **THEN** the system SHALL output a message indicating no token is stored (e.g., `Todoist token: not set`)

### Requirement: Auth revoke subcommand
The `task auth revoke` subcommand SHALL delete the stored token file.

#### Scenario: Revoke existing token
- **WHEN** the user runs `task auth revoke` and a token file exists
- **THEN** the system SHALL delete the token file and output a confirmation message

#### Scenario: Revoke when no token
- **WHEN** the user runs `task auth revoke` and no token file exists
- **THEN** the system SHALL output a message indicating no token was found (no error)

### Requirement: OAuth credentials via environment variables
The OAuth client ID and client secret SHALL be read from the environment variables `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET`.

#### Scenario: Credentials present
- **WHEN** `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET` are set
- **THEN** the system SHALL use those values for the OAuth flow

#### Scenario: Missing credentials
- **WHEN** `TODOIST_CLIENT_ID` or `TODOIST_CLIENT_SECRET` is not set
- **THEN** the system SHALL exit with code 1 and display an error describing which variable is missing
