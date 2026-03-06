## Requirements

### Requirement: Fetch all conversation types
The system SHALL request conversations of all types (`public_channel`, `private_channel`, `mpim`, `im`) from the Slack `conversations.list` API, excluding archived conversations.

#### Scenario: All conversation types returned
- **WHEN** the system fetches conversations from Slack
- **THEN** the API request SHALL include `types=public_channel,private_channel,mpim,im` and `exclude_archived=true`

#### Scenario: Pagination across multiple pages
- **WHEN** the API returns a non-empty `next_cursor` in response metadata
- **THEN** the system SHALL fetch subsequent pages until the cursor is empty or absent

### Requirement: Auto-discover channels with unread messages
When the user presses `s` in Normal mode, the system SHALL auto-discover member channels that have unread messages, without requiring prior channel selection.

#### Scenario: Sync with no configured channels -- auto-discover unread
- **WHEN** the user presses `s` in Normal mode
- **AND** no `slack-channels` config value is set (or it is empty)
- **THEN** the system SHALL fetch all member conversations, check each for unread messages using `conversations.info`, and fetch messages only from channels with unread content

#### Scenario: Sync with configured channels -- check only pinned
- **WHEN** the user presses `s` in Normal mode
- **AND** a non-empty `slack-channels` config value exists
- **THEN** the system SHALL check only the configured channels for unread messages using `conversations.info`, and fetch messages from those with unread content

#### Scenario: No unread messages in any channel
- **WHEN** the sync checks all relevant channels and none have unread messages
- **THEN** the system SHALL display "No unread Slack messages" as a status message

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
