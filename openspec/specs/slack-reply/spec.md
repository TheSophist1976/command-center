## Requirements

### Requirement: Reply to Slack message from TUI
The user SHALL press `r` in SlackInbox mode to enter `SlackReplying` mode for the selected message. A text input SHALL appear at the bottom of the screen. The user types a reply and presses `Enter` to send or `Esc` to cancel.

#### Scenario: Enter reply mode
- **WHEN** the user presses `r` on a selected message in SlackInbox mode
- **THEN** the TUI SHALL enter `SlackReplying` mode and display a text input prompt with the label "Reply to {channel_name}:"

#### Scenario: Send reply
- **WHEN** the user types "Looks good, I'll review it now" and presses `Enter` in SlackReplying mode
- **THEN** the system SHALL call the Slack `chat.postMessage` API with the channel ID and message text, display a status message "Reply sent", and return to SlackInbox mode

#### Scenario: Cancel reply
- **WHEN** the user presses `Esc` in SlackReplying mode
- **THEN** the input SHALL be discarded and the TUI SHALL return to SlackInbox mode

#### Scenario: Empty reply rejected
- **WHEN** the user presses `Enter` with an empty input buffer in SlackReplying mode
- **THEN** the system SHALL NOT send an API call and SHALL remain in SlackReplying mode

### Requirement: Reply API call
The system SHALL send replies using the Slack `chat.postMessage` API endpoint. The reply SHALL be sent as a new message in the channel (not threaded to the original message).

#### Scenario: Successful reply
- **WHEN** `chat.postMessage` is called with a valid token, channel ID, and message text
- **THEN** the API SHALL return `ok: true` and the system SHALL display "Reply sent"

#### Scenario: API error on reply
- **WHEN** `chat.postMessage` returns an error (e.g., `not_in_channel`, `channel_not_found`)
- **THEN** the system SHALL display the error message in the status bar and return to SlackInbox mode

#### Scenario: Missing chat:write scope
- **WHEN** `chat.postMessage` returns `missing_scope` error
- **THEN** the system SHALL display "Missing chat:write scope. Re-authenticate with `task auth slack`." in the status bar

#### Scenario: Rate limited
- **WHEN** `chat.postMessage` returns HTTP 429
- **THEN** the system SHALL display "Slack API rate limited. Try again later." in the status bar
