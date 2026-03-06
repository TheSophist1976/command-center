## ADDED Requirements

### Requirement: Slack inbox TUI mode
The TUI SHALL provide a `SlackInbox` mode that displays unread Slack messages in a scrollable list grouped by channel. The user SHALL enter this mode by pressing `s` in Normal mode. The mode SHALL fetch and sync new messages on entry.

#### Scenario: Enter Slack inbox
- **WHEN** the user presses `s` in Normal mode and a valid Slack token exists
- **THEN** the TUI SHALL sync new messages from configured channels, load the inbox, and display all messages with `status:open` in the `SlackInbox` mode

#### Scenario: No Slack token configured
- **WHEN** the user presses `s` in Normal mode and no Slack token is stored
- **THEN** the TUI SHALL display a status message "No Slack token. Run `task auth slack` from the CLI."

#### Scenario: No configured channels
- **WHEN** the user presses `s` and no channels are configured in `slack-channels`
- **THEN** the TUI SHALL open the channel picker to let the user select channels before syncing

### Requirement: Inbox message display
Each message in the inbox SHALL be displayed as a row showing the channel name, sender name, message text (truncated), relative timestamp, and a link indicator. The currently selected message SHALL be visually highlighted.

#### Scenario: Message row rendering
- **WHEN** the SlackInbox mode displays a message from `#general` by `Alice` saying "Can you review the deploy script?" from 2 hours ago
- **THEN** the row SHALL show columns: `#general`, `Alice`, `Can you review the deploy scr...`, `2h ago`

#### Scenario: Selected message highlighting
- **WHEN** the user navigates to a message in the inbox
- **THEN** the selected row SHALL use `theme::HIGHLIGHT_BG` background color

#### Scenario: Empty inbox
- **WHEN** the inbox has no open messages after sync
- **THEN** the TUI SHALL display "No unread Slack messages" and return to Normal mode

### Requirement: Inbox navigation
The user SHALL navigate the inbox using `j`/`k` or arrow keys. The cursor SHALL clamp at list boundaries.

#### Scenario: Navigate down
- **WHEN** the user presses `j` or `Down` with messages below the cursor
- **THEN** the selected message SHALL move down by one

#### Scenario: Navigate up
- **WHEN** the user presses `k` or `Up` with messages above the cursor
- **THEN** the selected message SHALL move up by one

#### Scenario: Cursor clamp at bottom
- **WHEN** the user presses `j` or `Down` on the last message
- **THEN** the cursor SHALL remain on the last message

### Requirement: Mark message as done
The user SHALL press `Enter` or `d` on a selected message to mark it as done. The message SHALL be removed from the visible inbox list and its status updated to `done` in the inbox file.

#### Scenario: Mark message done
- **WHEN** the user presses `Enter` on an open message
- **THEN** the message status SHALL be set to `done` in the inbox file, the message SHALL be removed from the visible list, and the cursor SHALL move to the next message (or previous if it was the last)

#### Scenario: All messages marked done
- **WHEN** the user marks the last open message as done
- **THEN** the TUI SHALL display "All messages handled" and return to Normal mode

### Requirement: Open message in Slack
The user SHALL press `o` on a selected message to open the Slack deep link in the system browser.

#### Scenario: Open deep link
- **WHEN** the user presses `o` on a message with link `https://myteam.slack.com/archives/C0123ABC/p1709654321000100`
- **THEN** the system SHALL execute `open` (macOS) to open the URL in the default browser

### Requirement: Re-sync messages
The user SHALL press `S` (shift-S) to re-fetch new messages from Slack without leaving the inbox mode.

#### Scenario: Re-sync in inbox mode
- **WHEN** the user presses `S` in SlackInbox mode
- **THEN** the system SHALL fetch new messages since the last HWM, append them to the inbox, prune old done messages, and refresh the displayed list

#### Scenario: Sync status message
- **WHEN** a sync completes and finds 5 new messages
- **THEN** the status bar SHALL display "Synced 5 new messages"

### Requirement: Exit inbox mode
The user SHALL press `Esc` to exit SlackInbox mode and return to Normal mode.

#### Scenario: Exit inbox
- **WHEN** the user presses `Esc` in SlackInbox mode
- **THEN** the TUI SHALL return to Normal mode with the task list visible

### Requirement: Inbox layout
The SlackInbox mode SHALL render a table with columns: Channel, Sender, Message, and Time. The layout SHALL include a header showing "Slack Inbox — N messages" and a footer with keybinding hints: `j/k:nav  Enter/d:done  r:reply  o:open  S:sync  Esc:back`.

#### Scenario: Inbox layout rendering
- **WHEN** the SlackInbox mode is active with 12 open messages
- **THEN** the header SHALL show "Slack Inbox — 12 messages" and the footer SHALL show the keybinding hints
