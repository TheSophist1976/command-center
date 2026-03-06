## MODIFIED Requirements

### Requirement: Auto-discover channels with unread messages (MODIFIED)
When the user presses `s` in Normal mode, the system SHALL auto-discover member channels that have unread messages, without requiring prior channel selection.

#### Scenario: Sync with no configured channels — auto-discover unread
- **WHEN** the user presses `s` in Normal mode
- **AND** no `slack-channels` config value is set (or it is empty)
- **THEN** the system SHALL fetch all member conversations, check each for unread messages using `conversations.info`, and fetch messages only from channels with unread content

#### Scenario: Sync with configured channels — check only pinned
- **WHEN** the user presses `s` in Normal mode
- **AND** a non-empty `slack-channels` config value exists
- **THEN** the system SHALL check only the configured channels for unread messages using `conversations.info`, and fetch messages from those with unread content

#### Scenario: No unread messages in any channel
- **WHEN** the sync checks all relevant channels and none have unread messages
- **THEN** the system SHALL display "No unread Slack messages" as a status message
