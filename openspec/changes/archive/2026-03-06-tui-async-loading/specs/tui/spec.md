## MODIFIED Requirements

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, EditingDetailPanel, ConfirmingDetailSave, EditingRecurrence, NlpChat, ConfirmingNlp, SlackInbox, SlackReplying, SlackChannelPicker, EditingNote, ConfirmingNoteExit, and NotePicker. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. While a background task is active, the event loop SHALL check for task completion on each tick and the footer SHALL show the spinner instead of normal hints. The `i` key in Normal mode SHALL trigger a Todoist import on a background thread. The `s` key SHALL trigger Slack sync on a background thread.

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import on a background thread with spinner feedback

#### Scenario: Colon key enters NlpChat mode
- **WHEN** the TUI is in Normal mode and the user presses `:`
- **THEN** the TUI SHALL enter NlpChat mode with an empty conversation history and display the split layout

#### Scenario: R key enters EditingRecurrence mode
- **WHEN** the TUI is in Normal mode and the user presses `R` with a task selected
- **THEN** the TUI SHALL enter EditingRecurrence mode and the footer SHALL display a text input prompt for recurrence

#### Scenario: s key triggers background Slack sync
- **WHEN** the TUI is in Normal mode and the user presses `s`
- **THEN** the TUI SHALL start a background Slack sync with spinner feedback (or show an error if no Slack token is configured)

#### Scenario: S key triggers background channel fetch
- **WHEN** the TUI is in Normal mode and the user presses `S` (shift-s)
- **THEN** the TUI SHALL start a background channel fetch with spinner feedback

#### Scenario: Background task blocks new operations
- **WHEN** a background task is running and the user presses `i`, `s`, or `S`
- **THEN** the TUI SHALL display "Operation in progress, please wait" and SHALL NOT start a new operation

#### Scenario: Footer shows spinner during background task
- **WHEN** a background task is active in Normal mode
- **THEN** the footer SHALL display the task description and animated spinner instead of keybinding hints
