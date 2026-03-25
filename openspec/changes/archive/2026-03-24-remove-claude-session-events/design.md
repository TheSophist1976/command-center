## Context

The `claude-session-events` change introduced a typed `SessionOutputEvent` enum to model structured TUI output: collapsible tool calls, a permission modal, piped stdin, and `--resume` support. In practice, these features added significant complexity across `claude_session.rs` and `tui.rs` while remaining hard to test and rarely used. This design documents how to safely strip that complexity and return to a flat `Vec<String>` output model.

## Goals / Non-Goals

**Goals:**
- Remove `SessionOutputEvent` and revert `ClaudeSession.output` to `Vec<String>`
- Remove `session_id`, `stdin`, and `--resume`; restore `--continue`
- Remove `Mode::PermissionModal` and all associated modal logic from `tui.rs`
- Remove collapsible tool call navigation
- Retain UX improvements from `claude-session-ux` that are model-independent (ANSI stripping, auto-scroll, markdown rendering adapted to plain strings)

**Non-Goals:**
- Re-designing the session output model in a new direction
- Removing the sessions panel or any other TUI navigation
- Touching note, task, or Todoist subsystems

## Decisions

**Revert to `Vec<String>` output, not a new intermediate abstraction**
The temptation is to introduce a simpler event type (e.g. `Line` vs `ToolOutput`). Rejected — the plain `Vec<String>` model is already well-understood, requires no parsing, and is trivially testable. Adding even a minimal enum reintroduces the same category of complexity we're removing.

**Keep `SessionEvent::Line(String)` as the channel message type**
The channel between the stream-reading thread and the TUI tick loop already uses `SessionEvent`. Retaining `SessionEvent::Line(String)` as the only active variant avoids touching the channel plumbing while cleaning up all the typed output variants (`OutputEvent`, `AppendToolResult`, `PermissionRequest`, `SessionIdCaptured`).

**Revert stdin to `Stdio::null()`**
The piped stdin was only needed for `--resume`. With that flag removed, there is no need to hold a `ChildStdin` handle. `Stdio::null()` is simpler and avoids potential deadlocks from an unconsumed stdin pipe.

**Retain ANSI stripping and auto-scroll**
These were introduced in `claude-session-ux` and operate purely on `Vec<String>`. They are independent of the event model and improve the experience regardless. Markdown rendering is similarly retained, adapted to operate on plain strings.

## Risks / Trade-offs

- **Loss of tool call visibility** → Accepted. Tool call blocks will appear as raw text lines in the output panel rather than collapsible entries. This is a deliberate trade-off for simplicity.
- **`--resume` flag removed** → Users relying on `--resume` will need to use `--continue`. Document in the CLI help text.
- **Delta spec for `claude-session`** → The spec must accurately reflect the reverted requirements. Risk of drift if the spec is not updated in sync with the code. Mitigation: the delta spec is a required artifact before tasks are created.

## Migration Plan

1. Update `claude_session.rs`: remove `SessionOutputEvent`, revert struct fields, revert stream parser to emit `SessionEvent::Line`
2. Update `tui.rs`: remove `PermissionModal` mode and handlers, revert session output rendering to iterate `Vec<String>`, remove `session_focused_event`
3. Remove `--resume` flag from CLI; restore `--continue` if absent
4. Adapt markdown rendering / ANSI stripping to work on `Vec<String>` (should be a small mechanical change)
5. Update delta spec for `claude-session` capability
6. Run full test suite; verify no compilation errors
