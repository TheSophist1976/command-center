## Context

The TUI's `SlackInbox` mode lets users read unread Slack messages and either dismiss them (Enter/d) or reply (r). There is currently no way to act on a message as a task without leaving the inbox and manually adding one. The existing task-creation pattern in the TUI uses `Mode::Adding` with a text input buffer and `InputAction::Add` to push a new `Task` struct onto `app.task_file`. The `SlackInbox` message struct carries the channel name, sender name, and full message text — all useful as task context.

## Goals / Non-Goals

**Goals:**
- Let the user press `t` on a selected inbox message to enter task-title editing mode
- Pre-fill the input buffer with the message text (truncated to a useful length)
- On Enter, create the task and mark the inbox message as done (dismissing it)
- Return to `SlackInbox` mode after creation

**Non-Goals:**
- Linking the created task back to the Slack message (deep link) — that belongs in a future change
- NLP-powered title extraction — the raw message text is sufficient as a starting point
- Bulk conversion of multiple messages at once

## Decisions

### 1. New `SlackCreatingTask` mode rather than reusing `Mode::Adding`

**Decision:** Add a new `Mode::SlackCreatingTask` enum variant and a dedicated `handle_slack_creating_task` function.

**Rationale:** `Mode::Adding` is tightly coupled to the task list view and `InputAction::Add` path in `handle_input`. Reusing it would require threading Slack-specific context (which inbox message to dismiss) through that function. A dedicated mode keeps the slack logic self-contained and avoids coupling the generic add flow to inbox state.

**Alternative considered:** Store a "pending task source" field on `App` and reuse `Mode::Adding` — rejected because it adds ambient state that complicates unrelated code paths.

### 2. Pre-fill title from message text, user edits before confirming

**Decision:** Populate `app.input_buffer` with the message text (first 120 chars, stripped of newlines) when entering `SlackCreatingTask` mode. User edits it inline before pressing Enter.

**Rationale:** Message text is almost always the best starting point for a task title. 120 chars covers most short messages and gives the user a sensible default without overwhelming the input field. The user can clear it completely if they want a custom title.

### 3. Mark message done immediately on task creation

**Decision:** When the task is saved, the inbox message at `slack_inbox_selected` is marked `InboxMessageStatus::Done` and the inbox is saved. This is the same behavior as pressing Enter/d directly.

**Rationale:** The message has been acted on — it would be redundant to keep it in the open inbox after converting it to a task. Consistency with the existing dismiss behavior (Enter/d) avoids a two-step flow.

### 4. Footer hint added to `SlackInbox` mode

**Decision:** Append `t:task` to the `SlackInbox` footer hints string.

**Rationale:** Discoverability — the keybinding should be visible in the UI alongside the other inbox actions.

## Risks / Trade-offs

- **[Input conflict]** `t` is already used in `Normal` mode for EditingTags — no conflict since `handle_slack_inbox` has its own key dispatch separate from `handle_normal`. ✓
- **[Empty title]** If the user clears the input and confirms with Enter on empty text, silently return to `SlackInbox` without creating a task (same guard as existing `InputAction::Add`).
- **[Cursor position]** `SlackCreatingTask` reuses the simple append-only input model (no cursor navigation) for now — consistent with the current `Mode::Adding` UX. Full cursor editing (like `SlackReplying`) can be added later if needed.
