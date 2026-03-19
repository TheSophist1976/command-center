## Context

The TUI already manages background tasks using a spawn-thread + `mpsc::channel` pattern (see `tui-background-tasks`). Claude Code (`claude`) is an interactive CLI that owns the terminal when run normally — it cannot share the terminal with the task-manager TUI. All interaction must go through pipes or non-interactive invocation modes.

Config is a plain `key: value` markdown file (see `app-config`). The existing `config::read_config_value` / `config::write_config_value` functions handle reads and writes.

## Goals / Non-Goals

**Goals:**
- Launch a Claude Code session in a chosen project directory, seeded with a task or note as context
- Display a scrollable list of active sessions in the TUI with live output
- Allow the user to send follow-up messages and see Claude's replies inline
- Directory picker sourced from a configurable root (default `~/code`)
- Graceful degradation when `claude` binary is not found
- Persist session history to disk so prior sessions are visible after TUI restart

**Non-Goals:**
- True streaming output (line-buffered polling is sufficient)
- Multiple simultaneous sessions sharing one screen region
- PTY / raw-mode passthrough to `claude`'s interactive TUI

## Decisions

### Claude invocation: `--print` + `--continue`, not PTY

Claude Code supports `claude --print -p "<message>"` for non-interactive, piped output. It also supports `--continue` to resume the most recent conversation stored in its local session cache. This avoids PTY allocation entirely, which would conflict with the task-manager's own raw-mode terminal ownership.

**Protocol per turn:**
1. First turn: `claude --print -p "<context>"` in the session's working directory. The initial `<context>` is the task title + description, or note title + body, formatted as plain text.
2. Subsequent turns: `claude --print --continue -p "<user_message>"` in the same directory. `--continue` picks up the conversation from claude's local `.claude/` session store.

Each turn spawns a new child process. The process exits when the response is complete. The TUI reads stdout until EOF, then marks the session as `WaitingForInput`.

Alternative considered: PTY passthrough (allocate a pseudo-terminal, hand it to `claude`, relay keystrokes). Rejected — it would require `nix::pty` or similar, significantly increasing complexity and coupling the TUI render loop to a raw byte stream with ANSI escape codes.

### Session representation

```rust
struct ClaudeSession {
    id: usize,
    label: String,                    // task/note title shown in panel
    working_dir: PathBuf,
    status: ClaudeSessionStatus,      // Running | WaitingForInput | Done | Failed
    output: Vec<String>,              // accumulated response lines (ring buffer, max 500)
    rx: Option<mpsc::Receiver<SessionEvent>>,
}

enum ClaudeSessionEvent {
    Line(String),   // stdout line from the current turn
    Done,           // child process exited cleanly
    Error(String),  // child exited with non-zero or failed to spawn
}
```

`App.claude_sessions: Vec<ClaudeSession>` stores all sessions. The event loop polls each session's `rx` channel on every tick, appending lines to `output` and updating `status`.

Alternative considered: Reuse the single `bg_task` slot. Rejected — sessions are persistent and multiple may coexist; `bg_task` is designed for one-at-a-time fire-and-forget operations.

### Session persistence: one JSON file per session in `<task-dir>/claude-sessions/`

Each session is persisted as `<task-dir>/claude-sessions/<id>-<slug>.json` containing: label, working directory, status, and full output lines. The directory is created on first session save.

**Write policy:** append-on-turn-complete — after each subprocess exits (status transitions to `WaitingForInput`, `Done`, or `Failed`), the session file is written. This avoids partial writes mid-stream and keeps I/O off the hot event loop path.

**Load policy:** on TUI startup, read all `*.json` files from `<task-dir>/claude-sessions/`, deserialize, and load them as `Done`/`Failed`/`WaitingForInput` sessions (never `Running` — a persisted session can't have an active subprocess). Loaded sessions appear in the Sessions panel with their full prior output.

**Retention:** keep the most recent 30 session files by modification time. On save, delete the oldest if count exceeds 30. This bounds disk usage without requiring user action.

**Format:** JSON (using `serde_json`). The `ClaudeSession` struct gets `#[derive(Serialize, Deserialize)]` with `child` and `rx` skipped (`#[serde(skip)]`).

Alternative considered: Storing sessions as markdown (like notes). Rejected — structured fields (status enum, output array) map naturally to JSON and avoid building a custom parser.

### Directory picker

On `C`, the TUI reads `claude-code-dir` from config (default `~/code`), calls `fs::read_dir`, filters to directories only, and sorts alphabetically. This list is stored in `App.session_dir_picker: Vec<PathBuf>` and `App.session_dir_picker_selected: usize`.

The picker is rendered as a modal overlay (same pattern as `draw_note_picker`). Esc cancels; Enter selects the directory and transitions to spawning the first turn.

If `~/code` does not exist and no config override is set, the picker shows a message: "No projects found — set `claude-code-dir` in config."

### Mode state machine

```
Normal / Notes view
    │  user presses C
    ▼
SessionDirectoryPicker
    │  Enter (directory selected)
    ▼
Sessions (status = Running, first turn in-flight)
    │  turn completes
    ▼
Sessions (status = WaitingForInput)
    │  user presses Enter
    ▼
SessionReply (text input for next message)
    │  user presses Enter (send) or Esc (cancel)
    ▼
Sessions (status = Running again, or WaitingForInput)
```

`Mode::Sessions` is the default view for the sessions panel. The user can press `q` or `Esc` to return to Normal mode; sessions continue running in the background. `S` (or a dedicated keybinding TBD) reopens the Sessions panel.

### Output ring buffer

Each session keeps the last 500 lines of output. Lines beyond 500 are dropped from the front. This bounds memory usage for long-running sessions.

### `claude-code-dir` config key

Stored in the existing config file as `claude-code-dir: /path/to/projects`. Read at picker open time (not at startup), so changes take effect immediately without restarting the TUI.

Default: `~/code` (tilde expanded at read time using `dirs::home_dir()`).

### Graceful degradation

Before spawning, the TUI checks for the `claude` binary using `which::which("claude")` (or equivalent `Command::new("claude").arg("--version")`). If not found, display a status message: "claude binary not found — install Claude Code to use sessions."

## Risks / Trade-offs

- **`--continue` picks up the wrong session**: `claude --continue` resumes the most recently touched session in `.claude/` within the working directory. If the user has other claude activity in that directory between turns, the wrong conversation may be resumed. Mitigation: document this limitation; a future iteration could use `--session-id` if/when claude exposes it.
- **Stdout buffering**: If `claude` buffers its stdout, lines may not arrive until the process exits. Mitigation: read stdout line-by-line on a dedicated thread; user sees output as it flushes. Most CLIs flush on newline.
- **Long responses block the output ring**: Very long responses (e.g., large code blocks) may fill 500 lines quickly. Mitigation: 500 lines covers typical responses; the full output is still visible by scrolling within the session detail.
- **Session orphaning on TUI exit**: Child processes spawned for in-flight turns will continue running after the TUI exits. Mitigation: on TUI exit, iterate `claude_sessions` and call `child.kill()` on any active process handle. The session `struct` holds the `Option<Child>` for exactly this purpose.

## Migration Plan

No data migration. New config key is optional with a default. New code is additive (`claude_sessions` starts empty, new modes are only entered via `C`). Existing behaviour is unchanged if the user never presses `C`.

Rollback: remove the `claude-session` code and revert the `tui`/`note-tui`/`app-config` deltas.

## Open Questions

- What keybinding reopens the Sessions panel after the user navigates away? (`S` is already used for Slack sync.) Candidate: `Alt-C` or a dedicated view in the view cycle. Deferred to spec.
- Should the session panel be a new entry in the view cycle (alongside Notes), or a floating overlay? Overlay feels lighter but a view-cycle entry would be more discoverable. Deferred to spec.
- Does `claude --continue` work correctly when invoked in a subdirectory that has no prior `.claude/` session? Need to verify behaviour: likely starts a new session, which is acceptable.
