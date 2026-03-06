## ADDED Requirements

### Requirement: Slack review mode
The TUI SHALL support a `Mode::SlackReview` mode that displays AI-suggested action items derived from Slack messages. The mode SHALL be entered by pressing `S` in normal mode and exited by pressing `Esc`.

#### Scenario: Enter Slack review
- **WHEN** the user presses `S` in normal mode and a Slack token is configured
- **THEN** the TUI SHALL enter `Mode::SlackReview` and begin fetching new Slack messages

#### Scenario: No Slack token
- **WHEN** the user presses `S` and no Slack token is configured
- **THEN** the TUI SHALL display a status message: "No Slack token. Run `task auth slack` from the CLI."

#### Scenario: No channels configured
- **WHEN** the user presses `S`, a token exists, but no `slack-channels` config is set
- **THEN** the TUI SHALL enter `Mode::SlackChannelPicker` to let the user select channels first

#### Scenario: Exit Slack review
- **WHEN** the user presses `Esc` in SlackReview mode
- **THEN** the TUI SHALL return to Normal mode

### Requirement: Slack fetch and analysis flow
When entering SlackReview mode with channels configured, the system SHALL: (1) show a spinner with "Fetching Slack messages...", (2) fetch new messages from all configured channels, (3) if messages exist, show a spinner with "Analyzing messages...", (4) send messages to the NLP engine for task suggestion analysis, (5) display the suggestions list. If no new messages are found, it SHALL display "No new Slack messages" and return to Normal mode.

#### Scenario: Messages found and analyzed
- **WHEN** new messages exist in configured channels
- **THEN** the system SHALL fetch them, send to NLP, and display the resulting suggestions

#### Scenario: No new messages
- **WHEN** no new messages exist since the last check
- **THEN** the system SHALL display "No new Slack messages" as a status message and return to Normal mode

#### Scenario: Fetch error
- **WHEN** the Slack API returns an error during fetch
- **THEN** the system SHALL display the error as a status message and return to Normal mode

#### Scenario: NLP analysis error
- **WHEN** the NLP engine fails to analyze messages
- **THEN** the system SHALL display the error as a status message and return to Normal mode

### Requirement: Suggestion list rendering
In SlackReview mode, the TUI SHALL render a list of AI-suggested action items. Each item SHALL display: the suggested task title, suggested priority (color-coded), source channel name, and a truncated snippet of the source Slack message. The list SHALL highlight the currently selected item. A header SHALL show "Slack Review — N suggestions" and a footer SHALL show keybinding hints.

#### Scenario: Render suggestion list
- **WHEN** SlackReview mode is active with 3 suggestions
- **THEN** the header SHALL show "Slack Review — 3 suggestions" and all 3 items SHALL be visible with title, priority, channel, and source snippet

#### Scenario: Empty suggestion list
- **WHEN** NLP analysis returns no actionable suggestions from the messages
- **THEN** the system SHALL display "No action items found in recent messages" and return to Normal mode

### Requirement: Suggestion navigation
The user SHALL navigate the suggestion list using `j`/`Down` to move down and `k`/`Up` to move up. Selection SHALL wrap within bounds (not cycle).

#### Scenario: Navigate down
- **WHEN** the user presses `j` with suggestion 1 of 3 selected
- **THEN** suggestion 2 SHALL become selected

#### Scenario: Navigate at bottom
- **WHEN** the user presses `j` with the last suggestion selected
- **THEN** the selection SHALL remain on the last suggestion

### Requirement: Accept suggestion
The user SHALL press `Enter` on a selected suggestion to accept it. Accepting SHALL create a new task with the suggested title, priority, and due date (if any). The task description SHALL include the source channel and message snippet for context. After accepting, the suggestion SHALL be removed from the list. The high-water-mark for the source channel SHALL be updated to include the accepted message's timestamp.

#### Scenario: Accept creates task
- **WHEN** the user presses `Enter` on a suggestion with title "Review PR #42", priority High
- **THEN** a new task SHALL be created with that title and priority, and the suggestion SHALL be removed from the list

#### Scenario: Accept includes source context in description
- **WHEN** a suggestion is accepted from channel `#engineering`
- **THEN** the task description SHALL include "From Slack #engineering:" followed by the source message text

#### Scenario: Last suggestion accepted
- **WHEN** the user accepts the last remaining suggestion
- **THEN** the system SHALL display "All suggestions reviewed" and return to Normal mode

### Requirement: Skip suggestion
The user SHALL press `s` to skip the currently selected suggestion without creating a task. The suggestion SHALL be removed from the list and the next item SHALL become selected. The high-water-mark SHALL still be updated for skipped messages.

#### Scenario: Skip removes suggestion
- **WHEN** the user presses `s` on suggestion 2 of 3
- **THEN** suggestion 2 SHALL be removed and the new suggestion 2 (previously 3) SHALL be selected

### Requirement: Edit suggestion before accepting
The user SHALL press `e` to edit the suggested title before accepting. The TUI SHALL enter an inline edit mode with the title pre-filled in the input buffer. Pressing `Enter` SHALL accept with the edited title; pressing `Esc` SHALL cancel the edit and return to the suggestion list.

#### Scenario: Edit and accept
- **WHEN** the user presses `e`, changes the title to "Review and merge PR #42", then presses `Enter`
- **THEN** a task SHALL be created with the edited title "Review and merge PR #42"

#### Scenario: Cancel edit
- **WHEN** the user presses `e` and then presses `Esc`
- **THEN** no task SHALL be created and the suggestion list SHALL be shown again

### Requirement: Slack review footer hints
The footer in SlackReview mode SHALL display: `j/k:nav  Enter:accept  e:edit  s:skip  Esc:exit`.

#### Scenario: Footer keybinding hints
- **WHEN** SlackReview mode is active
- **THEN** the footer SHALL display `j/k:nav  Enter:accept  e:edit  s:skip  Esc:exit`

### Requirement: Channel picker mode
The TUI SHALL support a `Mode::SlackChannelPicker` mode that displays available Slack channels and lets the user toggle which ones to monitor. The picker SHALL show channel names with a checkbox indicator. Pressing `Space` SHALL toggle a channel. Pressing `Enter` SHALL save the selection to config and proceed to SlackReview. Pressing `Esc` SHALL cancel and return to Normal mode.

#### Scenario: Display available channels
- **WHEN** SlackChannelPicker mode is entered
- **THEN** the system SHALL fetch the channel list from Slack and display each channel with `[ ]` (unselected) or `[x]` (selected) indicator

#### Scenario: Toggle channel selection
- **WHEN** the user presses `Space` on an unselected channel
- **THEN** the channel indicator SHALL change to `[x]`

#### Scenario: Save channel selection
- **WHEN** the user presses `Enter` with channels C1234 and C5678 selected
- **THEN** the config key `slack-channels` SHALL be set to `C1234,C5678` and the system SHALL proceed to fetch messages

#### Scenario: Cancel channel picker
- **WHEN** the user presses `Esc` in SlackChannelPicker mode
- **THEN** the system SHALL return to Normal mode without changing config

### Requirement: HWM update timing
The system SHALL update the high-water-mark for a channel only after all suggestions from that channel have been reviewed (accepted or skipped). When the user exits SlackReview with `Esc` before reviewing all suggestions, the HWM SHALL be updated only for messages that were reviewed, so unreviewed messages appear again on the next fetch.

#### Scenario: All suggestions reviewed
- **WHEN** the user accepts or skips all suggestions from channel C1234
- **THEN** the HWM for C1234 SHALL be updated to the latest message timestamp from that fetch

#### Scenario: Partial review then exit
- **WHEN** the user reviews 2 of 5 suggestions from C1234 and presses Esc
- **THEN** the HWM for C1234 SHALL be updated to the timestamp of the last reviewed message only
