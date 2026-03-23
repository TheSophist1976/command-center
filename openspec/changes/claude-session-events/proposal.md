## Why

The current Claude session output view stores all output as a flat `Vec<String>`, which makes it impossible to properly support collapsible tool calls, interactive permission requests, or reliable session resumption. The UX is cluttered — tool output floods the display and permission gates stall silently. This change introduces a structured event model that enables a claude-squad-quality TUI experience.

## What Changes

- Replace `output: Vec<String>` on `ClaudeSession` with `output: Vec<SessionOutputEvent>` — a typed enum covering text, tool calls, permission requests, and turn separators
- Tool calls are **collapsed by default** — shown as a single `⚙  Name: input [+]` line; `Enter`/`Space` expands the result inline
- **Permission request modal** — when Claude needs approval to use a tool, a modal overlay appears with `[y] Allow`, `[n] Deny`, `[a] Allow session`; response is written to the process stdin
- **Session ID tracking** — capture `session_id` from stream events and use `--resume <id>` for follow-up replies instead of `--continue`
- Stdin is now piped (not null) on spawned processes to support permission responses
- Stream parser is refactored to emit `SessionOutputEvent` variants instead of raw strings; `tool_result` events mutate the last `ToolCall` in the buffer

## Capabilities

### New Capabilities

_(none — all changes are modifications to the existing claude-session capability)_

### Modified Capabilities

- `claude-session`: Output data model, stream parser, permission handling, session resumption, and tool call rendering requirements are all changing.

## Impact

- `src/claude_session.rs`: New `SessionOutputEvent` enum, updated `ClaudeSession` struct, new stream parser, stdin piped, `--resume` support
- `src/tui.rs`: New `Mode::PermissionModal`, updated `draw_sessions_panel` for structured events and collapsible tool calls, new `handle_permission_modal`, updated `handle_sessions`
- `openspec/specs/claude-session/spec.md`: Delta spec for all new requirements
