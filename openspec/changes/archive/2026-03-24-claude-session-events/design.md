## Context

The current `ClaudeSession.output: Vec<String>` model was sufficient for displaying raw text but cannot support the features needed for a structured UX: collapsible tool calls require knowing where tool input ends and result begins; permission requests require pausing the stream and awaiting a typed response via stdin; session resumption requires knowing the session ID from Claude's own output.

The stream-json parser currently converts JSON events to display strings immediately. This change moves that logic earlier (into a typed event model) and makes the renderer responsible for presentation decisions (collapsed/expanded, permission modal).

## Goals / Non-Goals

**Goals:**
- Replace `Vec<String>` with `Vec<SessionOutputEvent>` — a typed enum per event
- Tool calls collapsed by default; `Enter`/`Space` to expand in the output detail view
- Permission request modal (overlay) with `y`/`n`/`a` keys, writing response to process stdin
- Capture `session_id` from stream events; use `--resume <id>` for follow-ups
- Pipe stdin on spawned processes

**Non-Goals:**
- Migration of existing persisted session JSON files (old files fail gracefully on load)
- Syntax highlighting inside code blocks
- Multi-level tool call nesting

## Decisions

### 1. `SessionOutputEvent` enum

```rust
pub enum SessionOutputEvent {
    Text(String),
    ToolCall {
        name: String,
        input_preview: String,   // first line of command/input
        result_lines: Vec<String>,
        collapsed: bool,
    },
    PermissionRequest {
        tool: String,
        input_preview: String,
    },
    TurnSeparator,
}
```

`ToolCall.collapsed` defaults to `true`. `result_lines` is populated when the corresponding `tool_result` event arrives — the parser mutates the last `ToolCall` in the session buffer rather than emitting a new event. This keeps tool input and output as one unit.

### 2. Stream parser emits `SessionEvent::OutputEvent`

`SessionEvent::Line(String)` is replaced by `SessionEvent::OutputEvent(SessionOutputEvent)`. A new variant `SessionEvent::PermissionRequest { tool, input_preview }` signals the TUI to enter permission modal mode. `SessionEvent::SessionIdCaptured(String)` passes the session ID to the main loop for storage on `ClaudeSession`.

The parser handles `tool_result` specially: instead of emitting an event, it sends a `SessionEvent::AppendToolResult { lines: Vec<String> }` that the polling loop applies to the last `ToolCall` in the buffer.

### 3. Permission flow

Stdin is opened as `Stdio::piped()` on both `launch_claude_session` and `continue_claude_session`. The `ClaudeSession` struct holds `stdin: Option<ChildStdin>` (skipped in serialization).

When `SessionEvent::PermissionRequest` arrives, the TUI transitions to `Mode::PermissionModal`. The modal renders as a centered overlay. `y`, `n`, `a` write `"y\n"`, `"n\n"`, or `"y\n"` (allow-all is handled by Claude itself) to `session.stdin`. After responding, the TUI returns to `Mode::Sessions` with `session_viewing_output = true`.

### 4. Session ID and `--resume`

`ClaudeSession` gains `session_id: Option<String>`. The polling loop stores it when `SessionEvent::SessionIdCaptured` arrives. `continue_claude_session` checks for a stored `session_id`: if present, uses `--resume <id>`; otherwise falls back to `--continue` for backward compatibility with sessions started before this change.

### 5. Rendering collapsible tool calls

`draw_sessions_panel` output detail view iterates `Vec<SessionOutputEvent>` instead of `Vec<String>`. For each event:
- `Text(s)` → `style_markdown_line` as before
- `ToolCall { collapsed: true }` → single amber line `⚙  Name: input [+]`
- `ToolCall { collapsed: false }` → amber header line + green result lines
- `PermissionRequest` → yellow warning line (modal handles interaction)
- `TurnSeparator` → dimmed separator

A `selected_tool_call: Option<usize>` field on `App` tracks which event index is focused when the user navigates inside the output view. `Enter`/`Space` toggles `collapsed` on the focused `ToolCall`.

### 6. Scroll math with structured events

`session_output_scroll` now counts **visible rendered lines** rather than event indices, since a collapsed tool call renders as 1 line but an expanded one renders as N. The scroll offset is recalculated at render time by iterating events and summing rendered heights.

## Risks / Trade-offs

- **Persistent session format break**: Old `Vec<String>` JSON won't deserialize into `Vec<SessionOutputEvent>`. Sessions from before this change will fail to load and be silently skipped. Acceptable given 30-file retention.
- **Scroll math complexity**: Counting rendered lines rather than events adds render-time iteration. At 500 events max this is negligible.
- **Permission stdin lifetime**: `ChildStdin` is held in `ClaudeSession` but the child process may exit before the user responds. Writing to a closed stdin will error — this is caught and ignored.

## Open Questions

_(none)_
