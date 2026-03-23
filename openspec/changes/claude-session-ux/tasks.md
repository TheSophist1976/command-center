## 1. ANSI Stripping

- [x] 1.1 Add `strip_ansi` function to `claude_session.rs` that removes ANSI escape sequences (ESC`[`...cmd byte) from a string
- [x] 1.2 Call `strip_ansi` inside `push_output_line` before appending to the ring buffer
- [x] 1.3 Add unit tests for `strip_ansi`: color codes stripped, plain text unchanged, partial sequences handled

## 2. Turn Separator

- [x] 2.1 Define a `TURN_SEPARATOR` constant string in `claude_session.rs` (e.g., `"──── reply ────"`)
- [x] 2.2 In `continue_claude_session`, append `TURN_SEPARATOR` via `push_output_line` before spawning the new subprocess

## 3. Follow Mode

- [x] 3.1 Add `session_output_follow: bool` field to `App` struct (default `true`)
- [x] 3.2 On opening output detail view (`Enter` in Sessions panel), set `session_output_follow = true` and scroll to bottom
- [x] 3.3 In the polling loop, when new lines are appended to the selected session's output and `session_output_follow` is true, update `session_output_scroll` to the new bottom

## 4. Output Detail Navigation

- [x] 4.1 In `handle_sessions` output-detail branch, map `j`/`Down` to scroll down and disable `session_output_follow`
- [x] 4.2 Map `k`/`Up` to scroll up and disable `session_output_follow`
- [x] 4.3 Map `Home`/`Char('g')` to scroll to top and disable `session_output_follow`
- [x] 4.4 Map `End`/`Char('G')` to scroll to bottom and enable `session_output_follow`

## 5. Markdown Rendering in Output Detail

- [x] 5.1 In `draw_sessions_panel` output detail branch, replace `Line::from(l.as_str())` with `style_markdown_line` (threading `in_code_block` state across lines)
- [x] 5.2 Detect `TURN_SEPARATOR` lines and render them with `Style::default().fg(Color::DarkGray)` instead of running markdown parsing

## 6. Sessions List Preview Fix

- [x] 6.1 In `draw_sessions_panel` list branch, replace `session.output.last()` with an iterator that finds the last non-blank (non-whitespace-only) line for the "last output" cell
