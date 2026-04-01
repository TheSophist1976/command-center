## MODIFIED Requirements

### Requirement: Three-region layout
The TUI SHALL render a three-region layout: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints.

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL use `theme::BAR_FG` foreground and `theme::BAR_BG` background, and the footer SHALL show keybinding hints with the same theme colors. The header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  f:filter  p:priority  e:edit  t:tags  r:desc  d:due  v:view  i:import  ::command  D:set-dir  S:slack  X:clr-due  R:recur  Tab:details  q:quit`

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
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, EditingDue, EditingDetailPanel, ConfirmingDetailSave, EditingRecurrence, SlackInbox, SlackReplying, SlackChannelPicker, EditingNote, ConfirmingNoteExit, NotePicker, EditingAgent, SessionDirectoryPicker, Sessions, and SessionReply. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. While a background task is active, the event loop SHALL check for task completion on each tick and the footer SHALL show the spinner instead of normal hints. The `i` key in Normal mode SHALL trigger a Todoist import on a background thread. The `s` key SHALL trigger Slack sync on a background thread.

#### Scenario: Mode transitions
- **WHEN** the user presses `a` in Normal mode
- **THEN** the TUI SHALL enter Adding mode and the footer SHALL display a text input prompt

#### Scenario: Recurrence editing mode
- **WHEN** the user presses `R` in Normal mode
- **THEN** the TUI SHALL enter EditingRecurrence mode and the footer SHALL display a text input prompt for recurrence

#### Scenario: Due date editing mode
- **WHEN** the user presses `d` in Normal mode
- **THEN** the TUI SHALL enter EditingDue mode and the footer SHALL display a text input prompt pre-filled with the task's current due date

## REMOVED Requirements

### Requirement: Delete task with confirmation
**Reason**: `d` key is repurposed for inline due date editing. Deletion is available via the CLI.
**Migration**: Use `task delete <id>` from the terminal to delete tasks.
