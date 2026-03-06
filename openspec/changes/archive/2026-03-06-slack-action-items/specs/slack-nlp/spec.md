## ADDED Requirements

### Requirement: Slack analysis NLP action
The system SHALL add a `SlackSuggestion` struct with fields: `title: String`, `priority: Priority`, `due_date: Option<NaiveDate>`, `source_channel: String`, `source_text: String`. This struct represents a single AI-suggested action item derived from a Slack message.

#### Scenario: Suggestion with all fields
- **WHEN** the NLP engine identifies an actionable message with a deadline
- **THEN** the `SlackSuggestion` SHALL have `title`, `priority`, `due_date`, `source_channel`, and `source_text` all populated

#### Scenario: Suggestion without due date
- **WHEN** the NLP engine identifies an actionable message with no time reference
- **THEN** the `SlackSuggestion` SHALL have `due_date` set to `None`

### Requirement: Slack analysis prompt
The system SHALL provide an `analyze_slack_messages(messages: &[(String, String, String)], api_key: &str, today: &str)` function that sends Slack messages to Claude for analysis. The input tuples contain `(channel_name, user_name, message_text)`. The function SHALL return `Result<Vec<SlackSuggestion>, String>`.

#### Scenario: Successful analysis
- **WHEN** `analyze_slack_messages` is called with 5 messages
- **THEN** it SHALL return a `Vec<SlackSuggestion>` containing only the actionable messages (may be fewer than 5)

#### Scenario: No actionable messages
- **WHEN** all messages are casual conversation with no action items
- **THEN** the function SHALL return an empty `Vec`

#### Scenario: API error
- **WHEN** the Claude API returns an error
- **THEN** the function SHALL return an `Err` with the error message

### Requirement: Slack analysis system prompt
The NLP system prompt for Slack analysis SHALL instruct Claude to: (1) analyze each message for actionable content (requests, deadlines, follow-ups, decisions needing action), (2) ignore casual conversation, greetings, reactions, and status updates, (3) return a JSON array of objects with `title` (imperative task title, max 80 chars), `priority` ("critical", "high", "medium", or "low"), `due_date` (ISO 8601 date string or null), `source_channel`, and `source_text` (the original message). The prompt SHALL include today's date for relative date resolution.

#### Scenario: Message with explicit request
- **WHEN** a message says "Can someone review PR #42 by Friday?"
- **THEN** the suggestion SHALL have a title like "Review PR #42", priority "medium" or "high", and a `due_date` set to the coming Friday

#### Scenario: Message with no action item
- **WHEN** a message says "Good morning everyone!"
- **THEN** it SHALL NOT generate a suggestion

#### Scenario: Message with urgency
- **WHEN** a message says "URGENT: production is down, need hotfix ASAP"
- **THEN** the suggestion SHALL have priority "critical" and no specific due_date (or today)

### Requirement: Response parsing
The system SHALL parse Claude's JSON response into a `Vec<SlackSuggestion>`. If the response is not valid JSON or contains malformed entries, individual malformed entries SHALL be skipped rather than failing the entire parse. Priority strings SHALL be parsed using the existing `Priority::from_str` logic.

#### Scenario: Valid JSON response
- **WHEN** Claude returns a valid JSON array with 3 suggestion objects
- **THEN** all 3 SHALL be parsed into `SlackSuggestion` structs

#### Scenario: Partially malformed response
- **WHEN** Claude returns 3 objects but one has no `title` field
- **THEN** 2 valid suggestions SHALL be returned and the malformed one SHALL be skipped

#### Scenario: Non-JSON response
- **WHEN** Claude returns plain text instead of JSON
- **THEN** the function SHALL return an `Err` indicating a parse failure
