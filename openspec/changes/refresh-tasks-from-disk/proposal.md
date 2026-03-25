## Why

When an external agent edits `tasks.md` while the TUI is open, the in-memory task list goes stale with no way to pick up those changes short of restarting the application. A manual reload keybinding lets the user resync instantly after an agent finishes writing.

## What Changes

- Add `ctrl+r` keybinding in Normal mode that reloads `tasks.md` from disk into the TUI's in-memory task list
- If the TUI is in any non-Normal mode (i.e., an edit is in progress), block the reload and show a warning status message
- On successful reload, replace the in-memory `TaskFile` and show a confirmation status message

## Capabilities

### New Capabilities

- `tui-disk-refresh`: Manual reload of `tasks.md` from disk while the TUI is running, triggered by `ctrl+r` in Normal mode

### Modified Capabilities

- `tui`: Adds a new `ctrl+r` keybinding and corresponding Normal-mode interaction scenarios to the existing TUI spec

## Impact

- `src/tui.rs` — `handle_key()` updated to handle `ctrl+r`; footer hint string updated to include the new binding
- No new dependencies
- No breaking changes
