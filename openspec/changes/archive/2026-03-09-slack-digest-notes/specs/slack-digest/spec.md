## ADDED Requirements

### Requirement: Slack workspace domain configuration
The system SHALL support a `slack-workspace` configuration key that stores the Slack workspace domain (e.g., `myteam`). This value SHALL be used for constructing message permalinks. The value SHALL be set via `task config set slack-workspace <domain>`.

#### Scenario: Workspace domain configured
- **WHEN** the user runs `task config set slack-workspace myteam`
- **THEN** the system SHALL store `myteam` as the Slack workspace domain

#### Scenario: Workspace domain not configured
- **WHEN** the Slack workspace domain is not set and a digest is generated
- **THEN** the system SHALL omit permalinks from the digest note and display a hint suggesting the user configure it

### Requirement: Slack message permalink construction
The system SHALL construct Slack message permalinks from a channel ID and message timestamp. The permalink format SHALL be `https://<workspace>.slack.com/archives/<channel_id>/p<timestamp_without_dot>` where the dot in the Slack timestamp is removed.

#### Scenario: Permalink from channel and timestamp
- **WHEN** the workspace is `myteam`, channel ID is `C123ABC`, and timestamp is `1709654321.000100`
- **THEN** the permalink SHALL be `https://myteam.slack.com/archives/C123ABC/p1709654321000100`

#### Scenario: Permalink with no workspace configured
- **WHEN** the workspace domain is not configured
- **THEN** no permalink SHALL be generated for that message

### Requirement: Digest analysis via single Claude API call
The system SHALL provide an `analyze_slack_digest` function that sends all fetched Slack messages to Claude and requests both a markdown summary and action items in a single API call. The response SHALL be a JSON object with `summary` (string) and `action_items` (array) fields.

#### Scenario: Successful digest analysis
- **WHEN** messages from channels `#general` and `#engineering` are sent for analysis
- **THEN** the response SHALL contain a `summary` field with markdown text organized by channel and an `action_items` field with extracted tasks

#### Scenario: No messages to analyze
- **WHEN** there are no new messages across any configured channels
- **THEN** the system SHALL skip the digest analysis and inform the user there are no new messages

### Requirement: Digest summary content format
The AI-generated summary SHALL be organized by channel with `## #channel-name` headings. Each channel section SHALL contain a concise summary of key discussions, decisions, and notable messages. Message permalinks SHALL be included as inline markdown links where the workspace domain is configured.

#### Scenario: Multi-channel summary with permalinks
- **WHEN** messages from `#general` and `#engineering` are analyzed with workspace configured
- **THEN** the summary SHALL contain `## #general` and `## #engineering` sections with discussion summaries and `[message](permalink)` links

#### Scenario: Summary without permalinks
- **WHEN** messages are analyzed without workspace domain configured
- **THEN** the summary SHALL contain channel sections with discussion summaries but no permalink links

### Requirement: Digest prompt includes pre-built permalinks
The system SHALL construct permalinks for each message before sending to the AI. Each message in the prompt context SHALL include its permalink (if available) so the AI can reference messages as markdown links in the summary output.

#### Scenario: Messages with permalinks in prompt
- **WHEN** messages are prepared for the AI prompt with workspace configured
- **THEN** each message SHALL include its permalink URL alongside the sender, timestamp, and text

### Requirement: Digest note creation
After a successful digest analysis, the system SHALL create a markdown note using the existing `write_note` infrastructure. The note slug SHALL follow the pattern `slack-digest-YYYY-MM-DD` where the date is the current date. If a note with that slug already exists, `unique_slug` SHALL generate a unique variant.

#### Scenario: Digest note saved with date slug
- **WHEN** a digest is generated on 2026-03-05
- **THEN** a note SHALL be created with slug `slack-digest-2026-03-05` containing the AI-generated summary as the body and "Slack Digest 2026-03-05" as the title

#### Scenario: Multiple digests on same day
- **WHEN** a second digest is generated on 2026-03-05 and `slack-digest-2026-03-05.md` already exists
- **THEN** the system SHALL create the note with slug `slack-digest-2026-03-05-2`

### Requirement: Digest flow integration
The Slack review flow SHALL first save the digest note automatically, display a confirmation message with the note slug, and then proceed to the existing `SlackReview` mode for task-by-task review of extracted action items.

#### Scenario: Complete digest flow
- **WHEN** the user initiates Slack review and messages are found
- **THEN** the system SHALL analyze messages, save the digest note, display "Saved digest note: slack-digest-YYYY-MM-DD", and enter `SlackReview` mode with the extracted action items

#### Scenario: Digest saved before task review
- **WHEN** the user exits the task review early (before reviewing all action items)
- **THEN** the digest note SHALL already be saved and accessible

### Requirement: Backward compatibility with existing Slack analysis
The existing `analyze_slack_messages` function SHALL remain unchanged. The new `analyze_slack_digest` function SHALL be a separate function that returns a `SlackDigestResult` containing both the summary and action items.

#### Scenario: Existing function preserved
- **WHEN** code calls `analyze_slack_messages`
- **THEN** it SHALL continue to return `Vec<SlackSuggestion>` as before

#### Scenario: New function returns combined result
- **WHEN** code calls `analyze_slack_digest`
- **THEN** it SHALL return a `SlackDigestResult` with `summary: String` and `suggestions: Vec<SlackSuggestion>` fields
