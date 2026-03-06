## ADDED Requirements

### Requirement: Slack API types
The system SHALL define deserialization types for Slack API responses: `SlackMessage` (with fields `ts: String`, `text: String`, `user: Option<String>`, `channel: Option<String>`), `SlackChannel` (with fields `id: String`, `name: String`, `is_member: bool`), and their paginated response wrappers with `response_metadata.next_cursor` for pagination.

#### Scenario: Deserialize message response
- **WHEN** the Slack API returns a JSON response with `ok: true` and a `messages` array
- **THEN** the system SHALL deserialize it into a `Vec<SlackMessage>` with `ts`, `text`, and optional `user` fields

#### Scenario: Deserialize channel list response
- **WHEN** the Slack API returns a JSON response with `ok: true` and a `channels` array
- **THEN** the system SHALL deserialize it into a `Vec<SlackChannel>` with `id`, `name`, and `is_member` fields

### Requirement: Fetch conversations list
The system SHALL provide a `fetch_channels(token: &str)` function that calls Slack's `conversations.list` API with `types=public_channel` and `exclude_archived=true`. It SHALL paginate using `cursor` until no `next_cursor` is returned. It SHALL return `Result<Vec<SlackChannel>, String>`.

#### Scenario: Fetch channels successfully
- **WHEN** `fetch_channels` is called with a valid token
- **THEN** it SHALL return all non-archived public channels the bot can see

#### Scenario: Paginated channel list
- **WHEN** the Slack API returns a `next_cursor` in the first response
- **THEN** the system SHALL make additional requests with the cursor until all channels are fetched

#### Scenario: Unauthorized token
- **WHEN** `fetch_channels` is called with an invalid token
- **THEN** it SHALL return an `Err` with a message indicating authentication failure

### Requirement: Fetch conversation history
The system SHALL provide a `fetch_messages(token: &str, channel_id: &str, oldest: Option<&str>, limit: usize)` function that calls Slack's `conversations.history` API. If `oldest` is provided, it SHALL pass it as the `oldest` parameter to fetch only messages after that timestamp. The `limit` parameter SHALL cap the number of messages returned (max 200). It SHALL return `Result<Vec<SlackMessage>, String>`.

#### Scenario: Fetch all messages from a channel
- **WHEN** `fetch_messages` is called with `oldest: None` and `limit: 100`
- **THEN** it SHALL return up to 100 most recent messages from the channel

#### Scenario: Fetch only new messages
- **WHEN** `fetch_messages` is called with `oldest: Some("1709654321.000100")`
- **THEN** it SHALL return only messages with `ts` greater than `1709654321.000100`

#### Scenario: Rate limited response
- **WHEN** the Slack API returns HTTP 429
- **THEN** the function SHALL return an `Err` with a message indicating rate limiting

#### Scenario: Bot not in channel
- **WHEN** the bot is not a member of the requested channel
- **THEN** the function SHALL return an `Err` with a message indicating the bot needs to be added to the channel

### Requirement: High-water-mark state persistence
The system SHALL store per-channel high-water-mark timestamps in a JSON file at `<config_dir>/task-manager/slack_state.json`. The file SHALL contain a JSON object mapping channel IDs to their latest read message `ts` string. The system SHALL provide `read_hwm(channel_id: &str)` to get the last-read timestamp and `write_hwm(channel_id: &str, ts: &str)` to update it.

#### Scenario: Read HWM for a tracked channel
- **WHEN** `slack_state.json` contains `{"C1234": "1709654321.000100"}` and `read_hwm("C1234")` is called
- **THEN** it SHALL return `Some("1709654321.000100")`

#### Scenario: Read HWM for an untracked channel
- **WHEN** `read_hwm("C9999")` is called and `C9999` is not in the state file
- **THEN** it SHALL return `None`

#### Scenario: Write HWM updates state file
- **WHEN** `write_hwm("C1234", "1709654400.000200")` is called
- **THEN** the state file SHALL be updated with `"C1234": "1709654400.000200"`, preserving other channel entries

#### Scenario: State file does not exist
- **WHEN** `read_hwm` is called and `slack_state.json` does not exist
- **THEN** it SHALL return `None` without error

### Requirement: Fetch new messages across channels
The system SHALL provide a `fetch_new_messages(token: &str, channel_ids: &[String])` function that iterates over each channel, reads its high-water-mark, calls `fetch_messages` with the HWM as `oldest`, and returns all new messages grouped by channel. After fetching, it SHALL NOT update the HWM — the caller updates HWM only after the user has reviewed the messages.

#### Scenario: Fetch from multiple channels
- **WHEN** `fetch_new_messages` is called with channels `["C1234", "C5678"]`
- **THEN** it SHALL return new messages from both channels, each message tagged with its source channel ID

#### Scenario: Channel with no new messages
- **WHEN** a channel has no messages newer than its HWM
- **THEN** the result for that channel SHALL be an empty list

#### Scenario: Channel fetch error is non-fatal
- **WHEN** one channel returns an error (e.g., bot not a member) but others succeed
- **THEN** the system SHALL skip the errored channel, include a warning, and return messages from the successful channels

### Requirement: API base URL override
The system SHALL read `SLACK_API_BASE_URL` environment variable to allow overriding the Slack API base URL (default: `https://slack.com/api`). This enables testing with mock servers.

#### Scenario: Default base URL
- **WHEN** `SLACK_API_BASE_URL` is not set
- **THEN** API calls SHALL use `https://slack.com/api` as the base URL

#### Scenario: Custom base URL for testing
- **WHEN** `SLACK_API_BASE_URL` is set to `http://localhost:1234`
- **THEN** API calls SHALL use `http://localhost:1234` as the base URL
