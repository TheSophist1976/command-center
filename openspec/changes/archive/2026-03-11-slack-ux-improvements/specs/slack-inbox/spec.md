## MODIFIED Requirements

### Requirement: Inbox message display
Each message in the inbox SHALL be displayed as a row showing the channel name, sender name, message text (truncated), relative timestamp, and a link indicator. The currently selected message SHALL be visually highlighted. When the preview pane is visible, the message text column MAY show fewer characters to accommodate the reduced table height.

#### Scenario: Message row rendering
- **WHEN** the SlackInbox mode displays a message from `#general` by `Alice` saying "Can you review the deploy script?" from 2 hours ago
- **THEN** the row SHALL show columns: `#general`, `Alice`, `Can you review the deploy scr...`, `2h ago`

#### Scenario: Selected message highlighting
- **WHEN** the user navigates to a message in the inbox
- **THEN** the selected row SHALL use `theme::HIGHLIGHT_BG` background color

#### Scenario: Empty inbox
- **WHEN** the inbox has no open messages after sync
- **THEN** the TUI SHALL display "No unread Slack messages" and return to Normal mode

### Requirement: Inbox layout
The SlackInbox mode SHALL render a table with columns: Channel, Sender, Message, and Time. The layout SHALL include a header showing "Slack Inbox -- N messages" and a footer with keybinding hints. When the preview pane is visible, the footer SHALL show `j/k:nav  Enter/d:done  r:reply  o:open  p:preview  S:sync  Esc:back`. When the preview pane is hidden, the footer SHALL show `j/k:nav  Enter/d:done  r:reply  o:open  p:preview  S:sync  Esc:back`. During SlackReplying mode, the footer SHALL show `Enter:send  Esc:cancel  ←→:move cursor  Home/End:jump`.

#### Scenario: Inbox layout with preview pane
- **WHEN** the SlackInbox mode is active with 12 open messages and preview pane visible
- **THEN** the header SHALL show "Slack Inbox -- 12 messages", the table SHALL occupy the upper portion, the preview pane SHALL occupy the lower portion, and the footer SHALL include the `p:preview` keybinding hint

#### Scenario: Inbox layout during reply
- **WHEN** the user is composing a reply in SlackReplying mode
- **THEN** the footer SHALL show reply-specific keybinding hints: `Enter:send  Esc:cancel  ←→:move cursor  Home/End:jump`
