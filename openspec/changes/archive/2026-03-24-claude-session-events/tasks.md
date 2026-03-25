## 1. Data Model

- [x] 1.1 Define `SessionOutputEvent` enum in `claude_session.rs` with variants: `Text(String)`, `ToolCall { name, input_preview, result_lines, collapsed }`, `PermissionRequest { tool, input_preview }`, `TurnSeparator` — derive `Serialize`, `Deserialize`, `Debug`, `Clone`
- [x] 1.2 Add `session_id: Option<String>` field to `ClaudeSession`
- [x] 1.3 Add `stdin: Option<ChildStdin>` field to `ClaudeSession` (annotated `#[serde(skip)]`)
- [x] 1.4 Replace `output: Vec<String>` with `output: Vec<SessionOutputEvent>` on `ClaudeSession`
- [x] 1.5 Update `push_output_line` to `push_output_event(output: &mut Vec<SessionOutputEvent>, event: SessionOutputEvent)` with 500-event ring buffer

## 2. Stream Parser

- [x] 2.1 Add `SessionEvent::OutputEvent(SessionOutputEvent)` variant (replacing `SessionEvent::Line`)
- [x] 2.2 Add `SessionEvent::AppendToolResult { lines: Vec<String> }` variant
- [x] 2.3 Add `SessionEvent::PermissionRequest { tool: String, input_preview: String }` variant
- [x] 2.4 Add `SessionEvent::SessionIdCaptured(String)` variant
- [x] 2.5 Rewrite `parse_stream_json_line` to return `Vec<SessionEvent>` using the new variants: `thinking` → `OutputEvent(Text)`, `tool_use` → `OutputEvent(ToolCall { collapsed: true, result_lines: [] })`, `tool_result` → `AppendToolResult`, `text` → one `OutputEvent(Text)` per line, `permission_request` → `PermissionRequest`, `result.session_id` → `SessionIdCaptured`

## 3. Session Launch & Stdin

- [x] 3.1 In `launch_claude_session`, change stdin to `Stdio::piped()` and store the `ChildStdin` on `session.stdin`
- [x] 3.2 In `continue_claude_session`, do the same for the new subprocess; if `session.session_id` is `Some(id)`, use `--resume <id>` instead of `--continue`
- [x] 3.3 Update the reader thread in both functions to call the new `parse_stream_json_line` and send the resulting `Vec<SessionEvent>`

## 4. Polling Loop (tui.rs)

- [x] 4.1 Update the polling loop to handle `SessionEvent::OutputEvent(e)` → call `push_output_event`
- [x] 4.2 Handle `SessionEvent::AppendToolResult { lines }` → find the last `ToolCall` in `session.output` and append lines to its `result_lines`
- [x] 4.3 Handle `SessionEvent::SessionIdCaptured(id)` → store on `session.session_id`
- [x] 4.4 Handle `SessionEvent::PermissionRequest { tool, input_preview }` → if this is the selected session and `session_viewing_output`, transition to `Mode::PermissionModal`; otherwise update session status to `WaitingForInput`
- [x] 4.5 On child process exit (`clear_rx`), drop `session.stdin` (set to `None`)

## 5. Permission Modal

- [x] 5.1 Add `Mode::PermissionModal` to the `Mode` enum
- [x] 5.2 Add `permission_modal_tool: String` and `permission_modal_input: String` fields to `App`
- [x] 5.3 Add `draw_permission_modal(frame, app, area)` function that renders a centered overlay with tool name, input preview, and key hints `[y] Allow  [n] Deny  [a] Allow session`
- [x] 5.4 Call `draw_permission_modal` from the main draw function when `mode == Mode::PermissionModal`
- [x] 5.5 Add `handle_permission_modal(app, key)` that on `y` writes `"y\n"` to stdin, on `n` writes `"n\n"`, on `a` writes `"y\n"` (allow-all delegated to Claude), then returns to `Mode::Sessions`

## 6. Collapsible Tool Calls in Output Detail

- [x] 6.1 Add `session_focused_event: usize` field to `App` (tracks which event index is focused in output detail view)
- [x] 6.2 Update `draw_sessions_panel` output detail branch to iterate `Vec<SessionOutputEvent>` instead of `Vec<String>`, rendering each event type: `Text` → markdown styled, `ToolCall { collapsed: true }` → single amber `[+]` line, `ToolCall { collapsed: false }` → amber header + green result lines, `PermissionRequest` → yellow warning line, `TurnSeparator` → dimmed
- [x] 6.3 Update scroll math: count total rendered lines by summing event render heights (collapsed ToolCall = 1, expanded = 1 + result_lines.len(), others = 1)
- [x] 6.4 In `handle_sessions` output-detail branch, map `Enter`/`Space` to toggle `collapsed` on the `ToolCall` event at the focused event index
- [x] 6.5 Update `j`/`k` navigation to move by rendered lines and update `session_focused_event` accordingly

## 7. Sessions List Preview

- [x] 7.1 Update `draw_sessions_panel` sessions list to derive the "last output" preview from `SessionOutputEvent` — find the last `Text` event with non-blank content, or fall back to the last `ToolCall.input_preview`
