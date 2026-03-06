## MODIFIED Requirements

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, EditingDetailPanel, ConfirmingDetailSave, EditingRecurrence, NlpChat, ConfirmingNlp, SlackInbox, SlackReplying, and SlackChannelPicker. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. The `i` key in Normal mode SHALL trigger a Todoist import (handled outside the mode system via the status message pattern). The `s` key in Normal mode SHALL enter the Slack inbox. The `S` key (shift) in Normal mode SHALL open the Slack channel picker.

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import as specified in the tui-todoist-import capability

#### Scenario: Colon key enters NlpChat mode
- **WHEN** the TUI is in Normal mode and the user presses `:`
- **THEN** the TUI SHALL enter NlpChat mode with an empty conversation history and display the split layout

#### Scenario: R key enters EditingRecurrence mode
- **WHEN** the TUI is in Normal mode and the user presses `R` with a task selected
- **THEN** the TUI SHALL enter EditingRecurrence mode and the footer SHALL display a text input prompt for recurrence (e.g., "Recurrence: _")

#### Scenario: s key enters SlackInbox mode
- **WHEN** the TUI is in Normal mode and the user presses `s`
- **THEN** the TUI SHALL sync Slack messages and enter SlackInbox mode (or show an error if no Slack token is configured)

#### Scenario: S key opens Slack channel picker
- **WHEN** the TUI is in Normal mode and the user presses `S` (shift-s)
- **THEN** the TUI SHALL open the Slack channel picker for configuring which channels to sync

## REMOVED Requirements

### Requirement: SlackReview mode
**Reason**: Replaced by SlackInbox mode. The NLP-based suggestion review flow is removed in favor of direct message browsing.
**Migration**: Users now press `s` to open the Slack inbox directly. Messages are displayed as-is without AI analysis. The `SlackReview` and `SlackReviewEditing` modes are removed.
