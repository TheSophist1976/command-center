## Why

The TUI currently lets users add, toggle, delete, and filter tasks — but editing a task's content requires leaving the TUI and using the CLI (`task edit <id> --priority high`, `task edit <id> --title "..."`, etc.). This breaks the workflow for users who want to triage and refine tasks interactively without dropping out of the TUI.

## What Changes

- Add TUI editing modes for four task fields: **priority**, **title**, **tags**, and **description**
- Priority: new `p` keybinding enters a single-keystroke picker (h/m/l) shown in the footer
- Title, tags, description: new keybindings (`e`, `t`, `r`) enter text-input modes pre-populated with the current value, consistent with the existing `Adding` and `Filtering` modes
- Persist each change to disk immediately on confirmation, consistent with other TUI mutations
- Update the footer help text to include all new keybindings

## Capabilities

### New Capabilities

_(none — this extends an existing capability)_

### Modified Capabilities

- `tui`: Add interactive editing modes for priority, title, tags, and description, with keybindings, inline input/picker UI in the footer, and immediate persistence

## Impact

- **Code**: `src/tui.rs` — new `Mode` variants, new key handlers, updated footer rendering and help text
- **No new dependencies** — uses existing ratatui widgets and the shared storage layer
- **No breaking changes** — all existing keybindings and behavior are preserved
