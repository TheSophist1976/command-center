## ADDED Requirements

### Requirement: Fetch all conversation types
The system SHALL request conversations of all types (`public_channel`, `private_channel`, `mpim`, `im`) from the Slack `conversations.list` API, excluding archived conversations.

#### Scenario: All conversation types returned
- **WHEN** the system fetches conversations from Slack
- **THEN** the API request SHALL include `types=public_channel,private_channel,mpim,im` and `exclude_archived=true`

#### Scenario: Pagination across multiple pages
- **WHEN** the API returns a non-empty `next_cursor` in response metadata
- **THEN** the system SHALL fetch subsequent pages until the cursor is empty or absent

### Requirement: Auto-discover member conversations for Slack review
When the user presses `S` (Slack review) and no `slack-channels` config is set, the system SHALL automatically fetch all conversations where the user is a member and use them for the review — without requiring the channel picker.

#### Scenario: No configured channels — auto-discover
- **WHEN** the user presses `S` in Normal mode
- **AND** no `slack-channels` config value is set (or it is empty)
- **THEN** the system SHALL call `fetch_channels` with all conversation types, filter results to only conversations where `is_member` is true, and proceed directly to the Slack review with all member conversation IDs

#### Scenario: Configured channels — use existing config
- **WHEN** the user presses `S` in Normal mode
- **AND** a non-empty `slack-channels` config value exists
- **THEN** the system SHALL use the configured channel IDs for the review (existing behavior preserved)

#### Scenario: Channel picker override remains available
- **WHEN** the user presses `C` in Normal mode
- **THEN** the system SHALL open the channel picker showing all conversation types, allowing the user to manually select which conversations to monitor

### Requirement: Conversation display names
Each conversation SHALL have a human-readable display name derived from its type, used in the channel picker and review UI.

#### Scenario: Public channel display name
- **WHEN** a conversation has type `public_channel`
- **THEN** its display name SHALL be `#<channel_name>` (e.g., `#general`)

#### Scenario: Private channel display name
- **WHEN** a conversation has type `private_channel`
- **THEN** its display name SHALL be `#<channel_name>` with a lock indicator (e.g., `🔒 #secret-project`)

#### Scenario: Direct message display name
- **WHEN** a conversation has type `im`
- **THEN** its display name SHALL be `DM with <user_display_name>` where the user's display name is resolved from their Slack user ID

#### Scenario: Group DM display name
- **WHEN** a conversation has type `mpim`
- **THEN** its display name SHALL be `Group: <name1>, <name2>, ...` listing participant display names resolved from their Slack user IDs

### Requirement: Expanded SlackChannel struct
The `SlackChannel` struct SHALL include fields for conversation type, display name, and optional user ID.

#### Scenario: Struct fields present
- **WHEN** a `SlackChannel` is constructed from an API response
- **THEN** it SHALL contain: `id`, `name`, `is_member`, `display_name` (human-readable label), `conversation_type` (one of `"channel"`, `"private"`, `"im"`, `"mpim"`), and `user` (the other participant's ID for IMs, `None` otherwise)

### Requirement: Graceful handling of missing OAuth scopes
The system SHALL handle `missing_scope` errors from the Slack API without blocking the entire review flow.

#### Scenario: Some conversation types fail due to missing scopes
- **WHEN** the Slack API returns a `missing_scope` error for a conversation type
- **THEN** the system SHALL skip that conversation type, continue fetching from other types, and display a status message informing the user which scopes are needed

#### Scenario: All conversation types fail
- **WHEN** all API requests fail (e.g., invalid token)
- **THEN** the system SHALL display an error message and return to Normal mode

### Requirement: Updated auth setup instructions
The `prompt_for_slack_token` interactive setup SHALL list all required OAuth scopes for full functionality.

#### Scenario: Auth prompt shows expanded scopes
- **WHEN** the user runs `task auth slack` without a `--token` flag
- **THEN** the printed instructions SHALL list the required scopes: `channels:history`, `channels:read`, `groups:history`, `groups:read`, `im:history`, `im:read`, `mpim:history`, `mpim:read`, `users:read`
