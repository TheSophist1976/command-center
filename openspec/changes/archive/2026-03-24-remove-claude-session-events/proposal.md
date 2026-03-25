## Why

The structured event model introduced in `claude-session-events` (typed `SessionOutputEvent` enum, collapsible tool calls, permission modal, piped stdin, `--resume` support) adds significant complexity to `claude_session.rs` and `tui.rs` without delivering enough value to justify the maintenance burden. Reverting to a simpler flat `Vec<String>` output model reduces code surface area and removes hard-to-test interactive behaviour.

## What Changes

- **BREAKING**: Remove `SessionOutputEvent` enum and revert `ClaudeSession.output` from `Vec<SessionOutputEvent>` back to `Vec<String>`
- **BREAKING**: Remove `session_id: Option<String>` and `stdin: Option<ChildStdin>` from `ClaudeSession`
- Revert stream parser back to emitting plain text lines (`SessionEvent::Line(String)` replaces `OutputEvent`, `AppendToolResult`, `PermissionRequest`, `SessionIdCaptured`)
- Revert subprocess stdin back to `Stdio::null()`; remove `--resume` flag, restore `--continue`
- Remove `Mode::PermissionModal`, `handle_permission_modal`, `draw_permission_modal`, `permission_modal_tool`, `permission_modal_input` from `tui.rs`
- Remove `session_focused_event` from `App`; revert `draw_sessions_panel` output detail branch to iterate `Vec<String>` directly
- Remove collapsible tool call navigation (`Enter`/`Space` toggle in output detail)
- Revert session list "last output" preview to use last non-blank `String`
- Parts of `claude-session-ux` that depend on `SessionOutputEvent` (ANSI stripping, auto-scroll) are retained as they are independent of the event model; markdown rendering is adapted to work on plain strings

## Capabilities

### New Capabilities

### Modified Capabilities
- `claude-session`: Revert output model, stream parser, process management, and permission handling to pre-events state

## Impact

- `src/claude_session.rs` — remove `SessionOutputEvent`, revert `ClaudeSession` struct and stream parser
- `src/tui.rs` — remove `PermissionModal` mode, revert session output rendering, remove tool call collapse logic
- `openspec/specs/claude-session/spec.md` — delta spec reverting the changed requirements
