## Why

When a task or note requires AI assistance, the user must context-switch to a separate terminal, manually copy the task content, and spawn Claude Code there. This breaks the workflow. Launching and monitoring Claude Code sessions directly from the task manager keeps context and conversation in one place.

## What Changes

- New keybinding (`C`) on a selected task or note in the TUI to open a directory picker, then spawn a Claude Code CLI session in the selected directory with that item's content pre-loaded as context
- New Sessions panel (overlay mode) in the TUI that lists all active Claude sessions, their context source (task/note title), running status, and last output line
- When a session is waiting for user input, the user can navigate to it in the Sessions panel and type a reply inline

## Capabilities

### New Capabilities
- `claude-session`: All session management within the TUI — a directory picker that lists subdirectories of a configurable root (default `~/code`), spawning a `claude` subprocess in the selected directory with task or note content as context, maintaining a list of active sessions, rendering the Sessions panel, and handling inline reply input when Claude is waiting for a response.

### Modified Capabilities
- `tui`: New keybinding (`C`) in Normal mode to open the directory picker; new `SessionDirectoryPicker`, `Sessions`, and `SessionReply` modes; updated footer hints.
- `note-tui`: New keybinding (`C`) in the Notes view to open the directory picker and launch a session with the selected note's content as context.
- `app-config`: New `claude-code-dir` config key (default `~/code`) specifying the root directory whose subdirectories are listed in the directory picker.

## Impact

- `src/tui.rs`:
  - New `ClaudeSession` struct: context label, child process handle, stdout reader, accumulated output buffer, status (Running / WaitingForInput / Done)
  - `App.claude_sessions: Vec<ClaudeSession>` to track active sessions
  - New `Mode::Sessions` and `Mode::SessionReply` variants
  - New `handle_sessions` and `draw_sessions_panel` functions
  - `C` keybinding wired in Normal mode (task list) and Notes view
  - Non-blocking stdout reads integrated into the TUI event loop
- `src/config.rs` / `app-config` spec: new `claude-code-dir` config key (default `~/code`); the directory picker reads this to enumerate project directories
- External dependency: `claude` binary must be installed and in PATH; feature degrades gracefully if not found
- No changes to `note.rs`, `storage.rs`, task model, or CLI subcommands
