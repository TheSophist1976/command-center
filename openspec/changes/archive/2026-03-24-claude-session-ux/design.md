## Context

The Claude session output detail view (`draw_sessions_panel` with `session_viewing_output = true`) renders raw lines from `claude --print` stdout as plain `Line::from(l.as_str())`. These lines contain ANSI escape sequences (color codes, cursor movement) and markdown formatting that appear as unreadable garbage or unformatted text in the TUI.

The codebase already has `style_markdown_line` in `tui.rs` (used for note rendering) that handles headings, bold, italic, inline code, code blocks, and blockquotes. This is the primary reuse point.

## Goals / Non-Goals

**Goals:**
- Strip ANSI escape codes from output lines before they are stored in the ring buffer
- Apply `style_markdown_line` to render markdown in the output detail view
- Inject visible turn-boundary separators when a new reply is sent
- Auto-scroll to the latest line when new output arrives (with manual scroll override)
- Add `Home`/`End` key navigation in the output detail view
- Fix the sessions list "last output" preview to skip blank/escape-only lines

**Non-Goals:**
- Switching from `claude --print` to a streaming JSON mode (separate change)
- Syntax highlighting within code blocks beyond what `style_markdown_line` provides
- Persistent scroll position across TUI mode switches

## Decisions

### 1. Strip ANSI at storage time (in `push_output_line`)

**Decision**: Strip ANSI escape codes inside `push_output_line` in `claude_session.rs` before the line is appended to the ring buffer.

**Rationale**: Stripping at storage time means all consumers (detail view, sessions list preview, persistence) see clean text. The alternative — stripping at render time — would require every render site to handle escape codes and would persist unreadable data to disk.

**Implementation**: Use a simple regex-free state machine to strip `ESC[...m` sequences (SGR codes) and other common escape sequences. The standard pattern `\x1b\[[0-9;]*[A-Za-z]` covers virtually all terminal color/style codes. A hand-written byte scanner avoids a regex dependency.

### 2. Reuse `style_markdown_line` for output detail rendering

**Decision**: Replace the plain `Line::from(l.as_str())` mapping in the output detail view with `style_markdown_line`, threading `in_code_block` state across lines.

**Rationale**: The function already exists and is tested. No new rendering logic needed.

### 3. Turn separators via injected sentinel lines

**Decision**: When `continue_claude_session` is called, inject a styled separator line (e.g., `"──── reply ────"`) into the session's output buffer before spawning the new subprocess.

**Rationale**: This keeps the data model simple (output stays `Vec<String>`) and gives a clear visual break between turns without a new data type. The sentinel is stored as-is; the renderer detects lines matching the pattern and styles them with a dimmed/accent color instead of running markdown parsing.

**Alternative considered**: A dedicated `Turn` enum wrapping `Vec<String>` per turn. Rejected — adds significant complexity to serialization, rendering, and scroll math for modest UX gain.

### 4. Auto-scroll with follow mode

**Decision**: Add a `session_output_follow: bool` field to `App`. When `true`, after new lines are appended to the selected session's output, `session_output_scroll` is updated to `total.saturating_sub(visible_height)`. Follow mode activates on Enter (open detail view) and deactivates when the user manually scrolls up (`j` or `Up`). Pressing `End` or `G` re-enables follow mode.

**Rationale**: Mirrors standard terminal pager behavior (`tail -f`). Without follow mode, new output from a Running session is invisible until the user scrolls down manually.

### 5. Home/End key bindings

**Decision**: In `handle_sessions` (output detail branch), map `KeyCode::Home` / `Char('g')` → scroll to top (disable follow), `KeyCode::End` / `Char('G')` → scroll to bottom (enable follow).

**Rationale**: Consistent with the note editor which already has Home/End support.

### 6. Sessions list "last output" preview

**Decision**: When building the `last_line` cell in `draw_sessions_panel`, iterate from the end of `output` and pick the first line that is non-empty after trimming whitespace.

**Rationale**: `claude --print` often ends output with blank lines; showing the last non-blank line is more informative.

## Risks / Trade-offs

- **ANSI stripper coverage**: The hand-written stripper covers SGR codes (`ESC[...m`) and simple escape sequences. Unusual sequences (e.g., OSC, DCS) might leave partial garbage. Mitigation: strip anything starting with `ESC[` up to the first ASCII letter (inclusive), which covers >99% of terminal output.
- **Sentinel line collision**: A real Claude response that happens to start with `────` would be styled as a separator. Mitigation: use a sentinel that includes a zero-width or bracketed marker unlikely to appear in prose, e.g., `"\x00──── reply ────"` with the null byte stripped before display.
- **Follow mode and scroll math**: The visible height used during polling differs from the height at render time (terminal resize). Mitigation: clamp scroll at render time (already done), so the worst case is a one-frame offset.

## Migration Plan

No data migration needed — stored session JSON contains the raw (pre-strip) output for sessions saved before this change. Those sessions will display with any residual ANSI codes intact, but new sessions will be clean. Acceptable for a UX improvement; no rollback needed.

## Open Questions

_(none — all decisions are resolved above)_
