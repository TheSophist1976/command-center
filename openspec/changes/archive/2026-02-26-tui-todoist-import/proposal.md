## Why

Todoist import currently requires exiting the TUI, running `task import todoist` from the CLI, and re-launching the TUI. Users who live in the TUI should be able to trigger an import directly without switching contexts.

## What Changes

- Add a new TUI mode (`Importing`) and keybinding (`i`) to trigger a Todoist import from within the TUI
- Display import progress and results in the TUI footer (fetching, importing N tasks, errors)
- After a successful import, reload the task list to show newly imported tasks
- Handle missing/invalid token gracefully with an in-TUI error message (no crash)

## Capabilities

### New Capabilities

- `tui-todoist-import`: TUI-initiated Todoist import flow — keybinding, import mode, progress display, error handling, and task list reload after import

### Modified Capabilities

- `tui`: Add `Importing` mode variant and `i` keybinding to mode-based input handling; update footer hints to include `i:import`

## Impact

- **Code**: `src/tui.rs` (new mode, keybinding handler, rendering), reuses existing `src/todoist.rs` import logic
- **Dependencies**: No new crates — uses existing `reqwest` and Todoist API client code
- **UX**: New `i` key in normal mode; non-blocking or blocking import with footer status feedback
