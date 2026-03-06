## Why

Several TUI operations make blocking network calls (Slack sync, Todoist import, Slack channel discovery) that freeze the entire UI for seconds or longer. The user sees no visual feedback — the terminal appears hung. The NLP chat already has a background-thread + spinner pattern, but it's not generalized to other operations.

## What Changes

- Generalize the existing background-thread + spinner pattern (used by NLP chat) into a reusable async task system
- Move Slack sync, Todoist import, and Slack channel fetching onto background threads
- Show an animated spinner/status message in the footer while operations are in progress
- Allow the user to cancel long-running operations with Esc
- Keep the TUI responsive (renders, handles quit) during background work

## Capabilities

### New Capabilities
- `tui-background-tasks`: Generic background task runner with spinner feedback and cancellation

### Modified Capabilities
- `tui`: Add Loading mode, integrate background task dispatch for Slack and Todoist operations

## Impact

- **Code modified**: `src/tui.rs` — new background task infrastructure, refactor Slack sync / Todoist import / channel picker to use it
- **Pattern change**: The existing `nlp_pending` / `nlp_spinner_frame` pattern gets generalized into a `BackgroundTask` enum + single receiver field
- **No new dependencies**: Uses existing `std::sync::mpsc` and `std::thread::spawn`
- **No breaking changes**: All keybindings remain the same, operations just become non-blocking
