## Requirements

### Requirement: Message preview pane display
The SlackInbox mode SHALL display a preview pane below the message table that shows the full text of the currently selected message. The preview pane SHALL include the sender name, channel name, and relative timestamp as a header line, followed by the complete message text.

#### Scenario: Preview pane shows selected message
- **WHEN** the user navigates to a message from `Alice` in `#general` saying "Can you review the deploy script? I made changes to the rollback logic and added error handling for the new edge cases we discussed yesterday."
- **THEN** the preview pane SHALL display a header line with `Alice · #general · 2h ago` and the full untruncated message text below

#### Scenario: Preview updates on navigation
- **WHEN** the user presses `j` to move to the next message
- **THEN** the preview pane SHALL immediately update to show the full content of the newly selected message

#### Scenario: Empty inbox hides preview
- **WHEN** the inbox has no open messages
- **THEN** the preview pane SHALL NOT be rendered

### Requirement: Preview pane word wrapping
The preview pane SHALL wrap long message text to fit the available width using the ratatui `Paragraph` widget with `Wrap { trim: false }`. Lines SHALL NOT be truncated.

#### Scenario: Long message wraps
- **WHEN** a message contains text wider than the preview pane width
- **THEN** the text SHALL wrap at word boundaries to fit within the pane

#### Scenario: Short message displays inline
- **WHEN** a message fits within a single line of the preview pane
- **THEN** the text SHALL be displayed on one line with no wrapping artifacts

### Requirement: Preview pane toggle
The user SHALL press `p` in SlackInbox mode to toggle the preview pane visibility. When hidden, the message table SHALL expand to fill the full area. The preview pane SHALL default to visible.

#### Scenario: Toggle preview off
- **WHEN** the user presses `p` with the preview pane visible
- **THEN** the preview pane SHALL be hidden and the message table SHALL expand to fill the available space

#### Scenario: Toggle preview on
- **WHEN** the user presses `p` with the preview pane hidden
- **THEN** the preview pane SHALL appear below the message table showing the currently selected message

### Requirement: Preview pane layout proportions
When visible, the preview pane SHALL occupy approximately 40% of the SlackInbox area, with the message table taking the remaining 60%. The preview pane SHALL have a minimum height of 3 rows.

#### Scenario: Layout split with preview
- **WHEN** the SlackInbox mode is active with the preview pane visible
- **THEN** the layout SHALL split vertically with the table at ~60% and the preview pane at ~40% (minimum 3 rows)

#### Scenario: Layout without preview
- **WHEN** the preview pane is toggled off
- **THEN** the message table SHALL occupy the full content area between header and footer
