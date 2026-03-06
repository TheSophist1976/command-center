## ADDED Requirements

### Requirement: Background task infrastructure
The TUI SHALL support running one background task at a time using a spawned thread and an `mpsc` channel. While a background task is active, the TUI SHALL remain responsive to rendering and quit commands.

#### Scenario: Background task spawned
- **WHEN** a blocking operation (Todoist import, Slack sync, Slack channel fetch) is initiated
- **THEN** the operation SHALL run on a spawned thread and the TUI event loop SHALL continue processing events

#### Scenario: Only one background task at a time
- **WHEN** the user initiates a blocking operation while another is already running
- **THEN** the TUI SHALL display a status message "Operation in progress, please wait" and SHALL NOT start the second operation

### Requirement: Animated spinner during background tasks
While a background task is running, the TUI SHALL display an animated spinner in the footer with a description of the operation. The spinner SHALL cycle through braille characters at approximately 200ms intervals (one frame per event loop tick).

#### Scenario: Spinner displays for Slack sync
- **WHEN** a Slack sync is running in the background
- **THEN** the footer SHALL display "Syncing Slack" followed by an animated spinner character

#### Scenario: Spinner displays for Todoist import
- **WHEN** a Todoist import is running in the background
- **THEN** the footer SHALL display "Importing from Todoist" followed by an animated spinner character

#### Scenario: Spinner displays for channel fetch
- **WHEN** a Slack channel fetch is running in the background
- **THEN** the footer SHALL display "Loading channels" followed by an animated spinner character

#### Scenario: Spinner animation cycles
- **WHEN** a background task is active and the event loop ticks
- **THEN** the spinner character SHALL advance to the next frame in the sequence `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`

### Requirement: Background task result handling
When a background task completes, the TUI SHALL apply the result in the main event loop thread. On success, the result SHALL update the App state. On error, the TUI SHALL display the error as a status message.

#### Scenario: Successful Todoist import result
- **WHEN** a background Todoist import completes with imported/skipped counts
- **THEN** the App task_file SHALL be updated with imported tasks, the file SHALL be saved, and a status message "Imported N tasks, skipped M" SHALL be displayed

#### Scenario: Successful Slack sync result
- **WHEN** a background Slack sync completes with a new inbox
- **THEN** the App slack_inbox SHALL be updated and the TUI SHALL either enter SlackInbox mode or display "No unread Slack messages"

#### Scenario: Successful channel fetch result
- **WHEN** a background channel fetch completes with channels
- **THEN** the App SHALL populate the channel picker state and enter SlackChannelPicker mode

#### Scenario: Error result
- **WHEN** a background task completes with an error
- **THEN** the TUI SHALL display the error text as a status message and clear the background task

#### Scenario: Thread disconnected
- **WHEN** the background thread panics or disconnects
- **THEN** the TUI SHALL display "Operation failed unexpectedly" as a status message and clear the background task

### Requirement: Cancel background task with Esc
The user SHALL press Esc while a background task is running to cancel/dismiss it. Cancellation drops the receiver; the spawned thread's result is discarded.

#### Scenario: Cancel background task
- **WHEN** the user presses Esc while a background task is running
- **THEN** the background task receiver SHALL be dropped, the spinner SHALL stop, and the TUI SHALL return to its previous state

#### Scenario: Esc with no background task
- **WHEN** the user presses Esc with no background task running (in Normal mode)
- **THEN** the existing Esc behavior SHALL apply (clear filter, etc.)
