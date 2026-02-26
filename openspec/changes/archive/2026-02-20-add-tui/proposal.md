## Why

The CLI subcommands (`task list`, `task done 3`, etc.) work well for scripting and AI agents, but for humans managing tasks day-to-day, the workflow involves repeated command invocations to view, filter, and act on tasks. A terminal UI provides an interactive dashboard where users can see all their tasks at a glance and take actions with single keystrokes — no need to remember command syntax or retype filters.

## What Changes

- Add a new `task tui` subcommand that launches a full-screen terminal interface
- The TUI displays a scrollable task list with columns matching the existing `task list` output (ID, status, priority, title, tags)
- Users can navigate tasks with arrow keys / vim-style keys, toggle completion, filter by status/priority/tag, and add/edit/delete tasks — all without leaving the interface
- The TUI reads and writes the same `tasks.md` file, sharing the existing parser and storage layer
- Adds `ratatui` and `crossterm` as new dependencies for terminal rendering and input handling

## Capabilities

### New Capabilities
- `tui`: Full-screen terminal interface for viewing and managing tasks interactively. Covers layout, navigation, keybindings, inline editing, filtering, and the `task tui` subcommand entry point.

### Modified Capabilities
- `cli-interface`: Adds the `tui` subcommand to the CLI's subcommand structure. The `--file` global flag and `TASK_FILE` env var apply to TUI mode as well.

## Impact

- **Dependencies**: Adds `ratatui` (~terminal UI framework) and `crossterm` (cross-platform terminal backend). Both are well-established Rust crates with no transitive C dependencies.
- **Code**: New `src/tui.rs` module (or `src/tui/` directory if it grows). Minor addition to `src/cli.rs` for the `Tui` subcommand variant, and a new match arm in `src/main.rs`.
- **Existing behavior**: No changes to existing CLI subcommands or JSON output. The TUI is purely additive — launched only via `task tui`.
- **Binary size**: Will increase moderately due to the terminal rendering library.
