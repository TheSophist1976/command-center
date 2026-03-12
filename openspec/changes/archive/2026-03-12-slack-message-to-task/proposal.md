## Why

When reviewing the Slack inbox, users often identify messages that require follow-up action. Currently they must manually switch to the task list and create a task from memory — the message context is lost. A direct "convert to task" action keeps users in the inbox flow and ensures the task captures the original message.

## What Changes

- Add a `t` keybinding in `SlackInbox` mode to create a task from the selected message
- The task title is pre-filled from the message text (user can edit before confirming)
- Once the task is created, the message is automatically marked as done and removed from the open inbox
- The new task appears in the task list immediately (same session state update)
- A status confirmation is shown: "Task created: <title>"

## Capabilities

### New Capabilities
- `slack-task-create`: Converting a Slack inbox message into a task — keybinding, inline title editing, task creation, and automatic inbox message dismissal.

### Modified Capabilities
- `slack-inbox`: Add `t:task` to the footer keybinding hints.

## Impact

- **Modified files**: `src/tui.rs` (new `SlackCreatingTask` mode or inline input handling, keybinding `t`, footer update), `src/task.rs` (task creation called from TUI context)
- **No new dependencies**: Uses existing `Task` struct and task file write infrastructure
- **No API changes**: Task creation is local; no Slack API calls beyond existing mark-read
