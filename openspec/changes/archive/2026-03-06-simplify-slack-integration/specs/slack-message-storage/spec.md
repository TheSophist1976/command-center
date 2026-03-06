## ADDED Requirements

### Requirement: Inbox file location and format
The system SHALL store Slack inbox state in a markdown file at `~/.config/task-manager/slack/inbox.md`. The file SHALL use HTML comments for machine-readable metadata. Each message SHALL be represented as a level-2 heading with metadata comments.

#### Scenario: Inbox file created on first sync
- **WHEN** the user syncs Slack messages for the first time and no inbox file exists
- **THEN** the system SHALL create `~/.config/task-manager/slack/inbox.md` with a file header containing workspace and last-sync metadata

#### Scenario: Inbox file header format
- **WHEN** the inbox file is created or updated
- **THEN** the file SHALL begin with `<!-- slack-inbox -->`, followed by `<!-- workspace: {WORKSPACE_ID} -->` and `<!-- last-sync: {ISO8601_TIMESTAMP} -->`

### Requirement: Message entry format
Each Slack message in the inbox file SHALL be stored as a level-2 heading in the format `## [{channel_display_name}] {author}: {message_text}` followed by metadata comments containing `ts`, `channel`, `user`, `status`, and `link` fields.

#### Scenario: Open message entry
- **WHEN** a new message is added to the inbox
- **THEN** the entry SHALL have a heading like `## [#general] Alice: Can you review the deploy script?` followed by `<!-- ts:1709654321.000100 channel:C0123ABC user:U456DEF status:open -->` and `<!-- link: https://workspace.slack.com/archives/C0123ABC/p1709654321000100 -->`

#### Scenario: Done message entry
- **WHEN** a message is marked as done
- **THEN** the metadata comment SHALL be updated to `status:done` and the entry SHALL remain in the file

#### Scenario: Message text with special characters
- **WHEN** a Slack message contains markdown-significant characters (e.g., `#`, `*`, `>`)
- **THEN** the heading text SHALL preserve the original message content without escaping (the metadata comments provide the machine-readable data)

### Requirement: Message deduplication
The system SHALL NOT create duplicate entries for the same message. A message SHALL be identified uniquely by its `channel` + `ts` combination.

#### Scenario: Duplicate message skipped on sync
- **WHEN** a sync fetches a message whose `channel:ts` pair already exists in the inbox file
- **THEN** the system SHALL skip that message and not add a duplicate entry

### Requirement: Deep link construction
Each message entry SHALL include a Slack deep link in the format `https://{workspace}.slack.com/archives/{channel_id}/p{ts_without_dot}`. The workspace name SHALL be stored in the app config as `slack-workspace`.

#### Scenario: Deep link format
- **WHEN** a message with ts `1709654321.000100` in channel `C0123ABC` is stored for workspace `myteam`
- **THEN** the link metadata SHALL be `https://myteam.slack.com/archives/C0123ABC/p1709654321000100`

#### Scenario: Workspace name fetched on first sync
- **WHEN** the `slack-workspace` config value is not set and a sync is initiated
- **THEN** the system SHALL call the Slack `auth.test` API to retrieve the workspace URL, extract the workspace name, and store it in config as `slack-workspace`

### Requirement: High-water-mark tracking
The inbox file SHALL track the latest message timestamp per channel using metadata comments in the format `<!-- hwm:{channel_id}:{timestamp} -->` in the file header section. This replaces the separate `slack_state.json` file.

#### Scenario: HWM updated after sync
- **WHEN** a sync fetches new messages for channel `C0123ABC` with max timestamp `1709654322.000200`
- **THEN** the file header SHALL contain `<!-- hwm:C0123ABC:1709654322.000200 -->`

#### Scenario: HWM used for incremental fetch
- **WHEN** a sync is initiated and the inbox file contains `<!-- hwm:C0123ABC:1709654321.000100 -->`
- **THEN** the system SHALL fetch only messages newer than `1709654321.000100` for that channel

### Requirement: Done message pruning
The system SHALL remove messages with `status:done` that are older than 7 days (based on the message timestamp) during each sync operation.

#### Scenario: Old done messages pruned
- **WHEN** a sync runs and the inbox contains a done message with ts older than 7 days
- **THEN** the entry SHALL be removed from the inbox file

#### Scenario: Recent done messages retained
- **WHEN** a sync runs and the inbox contains a done message with ts within the last 7 days
- **THEN** the entry SHALL remain in the inbox file

#### Scenario: Open messages never pruned
- **WHEN** a sync runs and the inbox contains an open message with ts older than 7 days
- **THEN** the entry SHALL remain in the inbox file regardless of age

### Requirement: Load and save operations
The system SHALL provide `load_inbox` and `save_inbox` functions that parse and serialize the inbox markdown file. The load function SHALL return a structured representation of all messages with their metadata. The save function SHALL write the file atomically (write to temp, then rename).

#### Scenario: Load inbox with messages
- **WHEN** `load_inbox` is called on a valid inbox file containing 5 messages
- **THEN** it SHALL return a list of 5 message structs with all metadata fields populated

#### Scenario: Load inbox from nonexistent file
- **WHEN** `load_inbox` is called and the inbox file does not exist
- **THEN** it SHALL return an empty inbox with no messages and no HWM data

#### Scenario: Save inbox atomically
- **WHEN** `save_inbox` is called with inbox data
- **THEN** the system SHALL write to a temporary file first, then atomically rename it to `inbox.md`
