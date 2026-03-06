## ADDED Requirements

### Requirement: Fetch channel read state from Slack
The system SHALL retrieve the `last_read` timestamp for each channel using the Slack `conversations.info` API. This timestamp represents Slack's native read cursor.

#### Scenario: Channel has unread messages
- **WHEN** `conversations.info` returns `last_read` less than `latest.ts` for a channel
- **THEN** the system SHALL consider that channel as having unread messages

#### Scenario: Channel has no unread messages
- **WHEN** `conversations.info` returns `last_read` equal to or greater than `latest.ts`
- **THEN** the system SHALL skip that channel during message fetch

#### Scenario: `last_read` not present in response
- **WHEN** `conversations.info` does not include a `last_read` field for a channel
- **THEN** the system SHALL fall back to fetching the most recent 50 messages from that channel

#### Scenario: `conversations.info` API error
- **WHEN** the `conversations.info` call fails for a channel (rate limit, network error)
- **THEN** the system SHALL skip that channel and continue with remaining channels

### Requirement: Fetch only unread messages
The system SHALL use the `last_read` timestamp as the `oldest` parameter when calling `conversations.history`, so only messages newer than the read cursor are returned.

#### Scenario: Fetch unread messages from channel
- **WHEN** a channel has `last_read` of `1709654321.000100`
- **THEN** the system SHALL call `conversations.history` with `oldest=1709654321.000100` to fetch only newer messages

### Requirement: Sync read state back to Slack
When a user marks a message as "done" in the TUI, the system SHALL call `conversations.mark` to update Slack's read cursor for that channel.

#### Scenario: Mark single message as read
- **WHEN** the user marks a message done with ts `1709654322.000200` in channel `C0123ABC`
- **THEN** the system SHALL call `conversations.mark` with `channel=C0123ABC` and `ts=1709654322.000200`

#### Scenario: Mark message read — API error
- **WHEN** the `conversations.mark` call fails (rate limit, missing scope, network error)
- **THEN** the system SHALL display a status message with the error but still mark the message as done locally

#### Scenario: Mark message read — missing write scope
- **WHEN** `conversations.mark` returns a `missing_scope` error
- **THEN** the system SHALL display "Could not sync read state to Slack — add write scopes to your app" and mark the message done locally

### Requirement: Updated OAuth scope requirements
The auth setup SHALL list write scopes needed for `conversations.mark`.

#### Scenario: Auth prompt shows write scopes
- **WHEN** the user runs `task auth slack` without a `--token` flag
- **THEN** the instructions SHALL include `channels:write`, `groups:write`, `im:write`, `mpim:write` in addition to existing read/history scopes
