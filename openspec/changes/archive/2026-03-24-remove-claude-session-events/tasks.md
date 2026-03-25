## 1. Remove SessionOutputEvent from claude_session.rs

- [x] 1.1 Delete the `SessionOutputEvent` enum and all its variants from `claude_session.rs`
- [x] 1.2 Revert `ClaudeSession.output` field from `Vec<SessionOutputEvent>` back to `Vec<String>`
- [x] 1.3 Remove `session_id: Option<String>` and `stdin: Option<ChildStdin>` fields from `ClaudeSession`
- [x] 1.4 Revert stream parser to emit `SessionEvent::Line(String)` only — remove `OutputEvent`, `AppendToolResult`, `PermissionRequest`, `SessionIdCaptured` variants
- [x] 1.5 Revert `launch_claude_session` stdin from `Stdio::piped()` back to `Stdio::null()`; remove `ChildStdin` capture
- [x] 1.6 Revert `continue_claude_session` to always use `--continue`; remove `--resume <session_id>` branch; revert stdin to `Stdio::null()`

## 2. Remove PermissionModal from tui.rs

- [x] 2.1 Remove `Mode::PermissionModal` variant from the `Mode` enum
- [x] 2.2 Remove `permission_modal_tool` and `permission_modal_input` fields from `App`
- [x] 2.3 Delete `handle_permission_modal` function
- [x] 2.4 Delete `draw_permission_modal` function
- [x] 2.5 Remove the `PermissionModal` arm from the main event dispatch and draw functions

## 3. Remove collapsible tool call navigation from tui.rs

- [x] 3.1 Remove `session_focused_event: Option<usize>` (or equivalent) from `App`
- [x] 3.2 Remove tool call toggle logic (`Enter`/`Space` in output detail expanding/collapsing `ToolCall` events)
- [x] 3.3 Revert `draw_sessions_panel` output detail branch to iterate `Vec<String>` directly

## 4. Adapt retained UX features to plain strings

- [x] 4.1 Verify ANSI stripping still operates on plain `String` lines (should be a no-op change)
- [x] 4.2 Verify session output follow mode (`session_follow`) still works with `Vec<String>`
- [x] 4.3 Verify markdown rendering (`style_markdown_line`) is called on each `String` line — adapt if it was tied to `Text` event variant
- [x] 4.4 Verify turn-boundary sentinel lines are still inserted on reply and styled correctly

## 5. Update session list preview

- [x] 5.1 Revert "last output" preview in the sessions list to use last non-blank `String` from `session.output`

## 6. Verify and clean up

- [x] 6.1 Ensure the project compiles with no errors (`cargo build`)
- [x] 6.2 Run `cargo test` and fix any test failures caused by the removed types
- [x] 6.3 Remove any dead imports or unused constants left over from the event model
