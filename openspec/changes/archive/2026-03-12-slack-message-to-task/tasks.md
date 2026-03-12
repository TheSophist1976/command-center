## 1. New Mode

- [x] 1.1 Add `SlackCreatingTask` variant to the `Mode` enum in `tui.rs`
- [x] 1.2 Add `SlackCreatingTask` to the `App` input-routing match in the event loop (route to new handler)
- [x] 1.3 Add `SlackCreatingTask` to the background-task blocking check (should block sync like other Slack modes)

## 2. Key Handler

- [x] 2.1 Add `KeyCode::Char('t')` branch in `handle_slack_inbox`: if open messages exist, pre-fill `input_buffer` with first 120 chars of selected message text (newlines → spaces) and set `mode = SlackCreatingTask`
- [x] 2.2 Create `fn handle_slack_creating_task(app: &mut App, key: KeyCode) -> Result<(), String>` with:
  - Esc → clear buffer, return to `SlackInbox`
  - Enter with empty input → return to `SlackInbox` without creating
  - Enter with non-empty input → create task, dismiss message, show status, return to `SlackInbox`
  - Char(c) → append to `input_buffer`
  - Backspace → pop from `input_buffer`

## 3. Task Creation Logic

- [x] 3.1 In the Enter branch of `handle_slack_creating_task`: create a `Task` with `next_id`, title from `input_buffer.trim()`, medium priority, no tags, `created: Utc::now()`, push to `app.task_file.tasks`, increment `next_id`, call `app.save()`
- [x] 3.2 After saving the task, mark the open message at `slack_inbox_selected` as `InboxMessageStatus::Done` (reuse the same open-index lookup as the Enter/d handler) and call `slack::save_inbox`
- [x] 3.3 Set `app.status_message = Some(format!("Task created: {}", title))`
- [x] 3.4 Clamp `slack_inbox_selected` if the dismissed message was the last one (mirror the Enter/d boundary check)

## 4. Footer & Display

- [x] 4.1 Update the `Mode::SlackInbox` footer string in `draw_footer` to include `t:task`
- [x] 4.2 Add `Mode::SlackCreatingTask` footer in `draw_footer`: `" Enter:confirm  Esc:cancel "`
- [x] 4.3 In `draw_slack_inbox`, render the `SlackCreatingTask` input panel (reuse or mirror `draw_slack_reply_panel` layout — show a titled block with the current `input_buffer` and a cursor indicator)

## 5. Tests & Verification

- [x] 5.1 Add unit test `slack_t_enters_creating_task_mode`: press `t` in `SlackInbox` with a message selected, assert mode is `SlackCreatingTask` and `input_buffer` is pre-filled
- [x] 5.2 Add unit test `slack_creating_task_confirm`: in `SlackCreatingTask` mode with a non-empty buffer, press Enter, assert a task was added to `task_file`, the inbox message is done, and mode returns to `SlackInbox`
- [x] 5.3 Add unit test `slack_creating_task_cancel`: press Esc, assert no task added and mode returns to `SlackInbox`
- [x] 5.4 Add unit test `slack_creating_task_empty_confirm`: press Enter with empty buffer, assert no task added and mode returns to `SlackInbox`
- [x] 5.5 Verify `cargo build --release` compiles cleanly with no warnings
