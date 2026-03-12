## ADDED Requirements

### Requirement: Convert Slack message to task
The TUI SHALL allow the user to create a task directly from a selected Slack inbox message by pressing `t` in `SlackInbox` mode. The system SHALL enter `SlackCreatingTask` mode, pre-fill the input with the message text, and create the task on confirmation.

#### Scenario: Enter task-creation mode from inbox
- **WHEN** the user presses `t` in `SlackInbox` mode with at least one open message selected
- **THEN** the TUI SHALL enter `SlackCreatingTask` mode with `input_buffer` pre-filled with the first 120 characters of the selected message's text (newlines replaced with spaces)

#### Scenario: No open messages
- **WHEN** the user presses `t` in `SlackInbox` mode and the inbox has no open messages
- **THEN** the TUI SHALL do nothing (no mode change)

### Requirement: Task creation input
In `SlackCreatingTask` mode the user SHALL be able to edit the pre-filled title using character input and Backspace, confirm with Enter, or cancel with Esc.

#### Scenario: Confirm task creation
- **WHEN** the user presses Enter in `SlackCreatingTask` mode with a non-empty input
- **THEN** the TUI SHALL create a new task with that title (medium priority, no tags), save the task file, mark the source inbox message as done, save the inbox, and return to `SlackInbox` mode
- **AND** the TUI SHALL display a status message "Task created: <title>"

#### Scenario: Confirm with empty input
- **WHEN** the user presses Enter in `SlackCreatingTask` mode with an empty input
- **THEN** the TUI SHALL return to `SlackInbox` mode without creating a task

#### Scenario: Cancel task creation
- **WHEN** the user presses Esc in `SlackCreatingTask` mode
- **THEN** the TUI SHALL return to `SlackInbox` mode without creating a task or modifying the inbox

### Requirement: Inbox message dismissed on task creation
When a task is created from an inbox message, the source message SHALL be automatically dismissed (marked done) and removed from the open inbox list, exactly as if the user had pressed Enter/d directly.

#### Scenario: Message removed after task creation
- **WHEN** a task is successfully created from inbox message M in channel #general
- **THEN** message M SHALL no longer appear in the open inbox list
- **AND** the inbox file SHALL reflect message M as status:done
