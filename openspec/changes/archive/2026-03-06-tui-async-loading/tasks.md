## 1. Background task infrastructure

- [x] 1.1 Define `BgTaskKind` enum with variants: `TodoistImport`, `SlackSync`, `SlackChannelFetch`
- [x] 1.2 Define `BgTaskResult` enum with result variants for each kind (imported tasks, inbox data, channel list)
- [x] 1.3 Add `bg_task: Option<(BgTaskKind, mpsc::Receiver<Result<BgTaskResult, String>>)>` and `bg_spinner_frame: u8` fields to `App`
- [x] 1.4 Add spinner character sequence constant `SPINNER_CHARS: &[char] = &['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏']`
- [x] 1.5 Add `bg_task_label()` method that returns the display string for each `BgTaskKind`
- [x] 1.6 Add background task check in the event loop: `try_recv()` on each tick, apply result or advance spinner

## 2. Result application handlers

- [x] 2.1 Implement `apply_bg_result()` match arm for `TodoistImport` — update task_file, save, set status message
- [x] 2.2 Implement `apply_bg_result()` match arm for `SlackSync` — update slack_inbox, enter SlackInbox mode or show "no messages"
- [x] 2.3 Implement `apply_bg_result()` match arm for `SlackChannelFetch` — populate channel picker state, enter SlackChannelPicker mode
- [x] 2.4 Handle error results — display error as status message, clear bg_task
- [x] 2.5 Handle disconnected channel — display "Operation failed unexpectedly", clear bg_task

## 3. Move operations to background threads

- [x] 3.1 Refactor Todoist import (`i` key) to spawn a background thread, send result over channel
- [x] 3.2 Refactor Slack sync (`s` key) to spawn a background thread, send result over channel
- [x] 3.3 Refactor Slack channel fetch (`S` key) to spawn a background thread, send result over channel
- [x] 3.4 Guard all three key handlers: if `bg_task.is_some()`, show "Operation in progress" and return

## 4. Spinner display and cancellation

- [x] 4.1 Update footer rendering: when `bg_task` is active, show task label + spinner char instead of keybinding hints
- [x] 4.2 Add Esc handling: if `bg_task` is active in Normal mode, drop the receiver to cancel
- [x] 4.3 Update test helper `make_app_with_tasks` and `make_app_with_tmpfile` with new `bg_task`/`bg_spinner_frame` fields

## 5. Testing and verification

- [x] 5.1 Write test: bg_task_label returns correct strings for each variant
- [x] 5.2 Write test: spinner frame advances on tick
- [x] 5.3 Write test: operation blocked while bg_task is active
- [x] 5.4 Run full test suite (`cargo test`) and fix any breakage
