## MODIFIED Requirements

### Requirement: Inbox file header format (MODIFIED)
The inbox file header SHALL no longer contain `<!-- hwm:CHANNEL:TS -->` lines. The `SlackInbox` struct SHALL NOT have a `hwm` field. Slack's native `last_read` is the source of truth for read position.

#### Scenario: Inbox file header without HWM
- **WHEN** the inbox file is saved
- **THEN** the header SHALL contain `<!-- slack-inbox -->`, `<!-- workspace: {WORKSPACE_ID} -->`, and `<!-- last-sync: {ISO8601_TIMESTAMP} -->` but NO `<!-- hwm:... -->` lines

#### Scenario: Load inbox with legacy HWM lines
- **WHEN** `load_inbox` encounters `<!-- hwm:... -->` lines in an existing file
- **THEN** the parser SHALL silently ignore those lines (backward compatible)

### Requirement: SlackInbox struct without HWM (MODIFIED)
The `SlackInbox` struct SHALL NOT contain a `hwm` field. The struct SHALL have `workspace`, `last_sync`, and `messages` fields only.

#### Scenario: SlackInbox struct fields
- **WHEN** a `SlackInbox` is constructed
- **THEN** it SHALL contain `workspace: String`, `last_sync: Option<DateTime<Utc>>`, and `messages: Vec<SlackInboxMessage>`
