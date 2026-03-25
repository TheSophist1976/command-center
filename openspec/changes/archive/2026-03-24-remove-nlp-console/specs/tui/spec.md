## REMOVED Requirements

### Requirement: NLP loading indicator
**Reason**: The NLP console is removed. There is no NLP background thread to indicate.
**Migration**: No replacement. The `i` key no longer triggers NLP.

### Requirement: NLP message responses
**Reason**: The NLP console is removed. `NlpAction::Message` no longer exists.
**Migration**: No replacement.

### Requirement: Chat panel display
**Reason**: The NLP console is removed. The chat panel rendered in NlpChat mode no longer exists.
**Migration**: No replacement.

### Requirement: ShowTasks display in chat panel
**Reason**: The NLP console is removed. `NlpAction::ShowTasks` no longer exists.
**Migration**: No replacement.

### Requirement: NlpChat conversation lifecycle
**Reason**: The NLP console is removed. `Mode::NlpChat` and `Mode::ConfirmingNlp` are deleted.
**Migration**: No replacement.

## MODIFIED Requirements

### Requirement: Three-region layout
The TUI SHALL render a three-region layout: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints.

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL use `theme::BAR_FG` foreground and `theme::BAR_BG` background, and the footer SHALL show keybinding hints with the same theme colors. The header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  S:slack  T/W/M/Q:due  X:clr-due  R:recur  Tab:details  q:quit`

#### Scenario: Footer hints with detail panel visible
- **WHEN** the TUI is in Normal mode with the detail panel open
- **THEN** the footer SHALL show `j/k:nav  Enter:edit  s:save  d:discard  c:cancel  Tab:close`

#### Scenario: Footer hints in detail editing mode
- **WHEN** the TUI is in `EditingDetailPanel` mode
- **THEN** the footer SHALL show hints for editing: `j/k:field  Enter:save  Esc:cancel` (or equivalent context-sensitive hints)

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, EditingDetailPanel, ConfirmingDetailSave, EditingRecurrence, SlackInbox, SlackReplying, SlackChannelPicker, EditingNote, ConfirmingNoteExit, NotePicker, EditingAgent, SessionDirectoryPicker, Sessions, and SessionReply. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. While a background task is active, the event loop SHALL check for task completion on each tick and the footer SHALL show the spinner instead of normal hints. The `i` key in Normal mode SHALL trigger a Todoist import on a background thread. The `s` key SHALL trigger Slack sync on a background thread.

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import on a background thread with spinner feedback

#### Scenario: R key enters EditingRecurrence mode
- **WHEN** the TUI is in Normal mode and the user presses `R` with a task selected
- **THEN** the TUI SHALL enter EditingRecurrence mode and the footer SHALL display a text input prompt for recurrence

#### Scenario: s key triggers background Slack sync
- **WHEN** the TUI is in Normal mode and the user presses `s`
- **THEN** the TUI SHALL start a background Slack sync with spinner feedback (or show an error if no Slack token is configured)
