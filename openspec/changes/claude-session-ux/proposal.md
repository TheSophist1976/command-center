## Why

The Claude session output view displays raw text lines that are difficult to read — markdown is unrendered, ANSI escape codes appear as literal characters, and there's no visual structure to distinguish tool calls, thinking, and agent responses. The interactive terminal experience (running `claude` directly) is clear and structured; the TUI session viewer needs to match that clarity.

## What Changes

- Strip ANSI escape codes from output lines before storing or displaying them
- Render markdown in the output detail view (reuse existing markdown rendering used in notes)
- Visually separate conversation turns (user prompt vs. agent response) with headers or dividers
- Auto-scroll to the bottom of output when new lines arrive during a Running session
- Add `Home`/`End` key support in the output detail view for faster navigation
- Clean up the "last output" preview column in the sessions list (strip ANSI, skip blank lines)

## Capabilities

### New Capabilities

_(none — all changes are improvements to the existing claude-session capability)_

### Modified Capabilities

- `claude-session`: Output rendering requirements are changing — lines must be ANSI-stripped before storage, the detail view must render markdown, turns must be visually separated, and auto-scroll must engage while a session is Running.

## Impact

- `src/claude_session.rs`: ANSI stripping in `push_output_line`; turn-boundary markers when a new reply is sent
- `src/tui.rs`: `draw_sessions_panel` output detail view — markdown rendering, turn headers, auto-scroll, Home/End; sessions list "last output" cell — skip blank/escape-only lines
- `openspec/specs/claude-session/spec.md`: delta spec with new output rendering requirements
