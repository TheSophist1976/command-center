## Context

The Slack inbox TUI (`draw_slack_inbox`) renders messages as a table with four fixed columns (Channel, Sender, Message, Time). Message text is truncated to 60 characters. Replies are composed in a single-line status bar input with no visible context of the original message. The TUI already supports split-pane layouts in other modes — a chat panel (`draw_chat_panel`) uses `Constraint::Percentage(55)` for the table and `Constraint::Min(3)` for the panel, and a detail panel uses `Constraint::Percentage(70)` / `Constraint::Min(3)`.

Key existing patterns:
- `app.show_detail_panel` boolean toggles the detail pane in task view
- `app.input_buffer` is used for single-line text input (reply, NLP chat, search)
- `Mode::SlackReplying` currently reuses the status bar area for input
- `draw_slack_inbox` is called from the main draw dispatch; layout is self-contained within its allocated `Rect`

## Goals / Non-Goals

**Goals:**
- Show full message text in a preview pane below the message list, updating as the user navigates
- Provide a reply composition panel that displays the original message above a multi-line text input
- Support basic cursor movement in the reply input (left/right, Home/End)
- Allow toggling the preview pane on/off with `p`
- Follow existing split-pane layout patterns (chat panel, detail panel)

**Non-Goals:**
- Threaded reply support (replies post as channel messages, same as today)
- Message search or filtering within the inbox
- Markdown rendering in the preview pane (raw text only)
- Scrollable preview for extremely long messages (truncate to visible area for now)
- Editing previously sent messages

## Decisions

### 1. Preview pane as a vertical split below the message table

**Choice:** Split the `SlackInbox` layout into table (top, ~55%) and preview pane (bottom, remaining space), mirroring the existing chat panel layout pattern.

**Alternatives considered:**
- Horizontal split (table left, preview right): Would reduce table width, making the already constrained message column narrower
- Popup/overlay: More complex, breaks the terminal flow; existing TUI has no overlay pattern

**Rationale:** Vertical split is already proven in the codebase (`draw_chat_panel`, `draw_detail_panel`). Users can toggle it off with `p` if they prefer the full table.

### 2. Toggle state stored as `app.slack_preview_visible: bool`

**Choice:** Add a boolean field to `App`, defaulting to `true`. The `p` key toggles it. When off, the table takes the full area (current behavior).

**Alternatives considered:**
- Always show preview: Some users may prefer more message rows visible
- Config-persisted preference: Over-engineering for a toggle

**Rationale:** Simple, matches the `app.show_detail_panel` pattern. Defaulting to on showcases the feature; users who prefer density can press `p`.

### 3. Reply panel replaces preview pane, not a new mode layout

**Choice:** When entering `SlackReplying`, the preview pane area transforms into a reply composition panel. The top portion shows the original message (quoted), the bottom is the text input. The message table remains visible above.

**Alternatives considered:**
- Full-screen reply editor: Loses inbox context
- Status bar input (current): Too cramped, no context visible

**Rationale:** Reusing the preview pane area means no layout restructuring when switching to reply mode. The user sees the message list, the message they're replying to, and their input simultaneously.

### 4. Multi-line reply input with cursor tracking

**Choice:** Replace `app.input_buffer` (String) usage in SlackReplying with cursor-aware input: track `cursor_pos: usize` as a byte offset into the buffer. Support Left/Right arrow keys and Home/End for cursor movement within the single logical line. The text wraps visually but is sent as a single message.

**Alternatives considered:**
- True multi-line editor with line tracking: Overkill for Slack messages; Slack itself treats messages as single blocks
- Keep single-line input but in the panel: Misses the opportunity for a better editing experience

**Rationale:** Most Slack replies are short. Cursor movement within a single buffer is the minimum viable improvement. Visual word-wrapping in the panel area gives the appearance of multi-line editing without line-tracking complexity.

### 5. Word-wrapping via `Paragraph` widget with `Wrap { trim: false }`

**Choice:** Use ratatui's built-in `Paragraph` widget with `Wrap` for both the preview pane message text and the reply input. This handles word-wrapping automatically based on available width.

**Alternatives considered:**
- Manual line-breaking: Error-prone, duplicates ratatui functionality

**Rationale:** `Paragraph::new(text).wrap(Wrap { trim: false })` is the idiomatic ratatui approach and handles all edge cases (long words, Unicode).

## Risks / Trade-offs

- **[Reduced table rows]** The preview pane cuts visible message rows roughly in half. → Mitigation: `p` toggle lets users reclaim full height when scanning many messages.
- **[Cursor byte offset vs. grapheme]** Tracking cursor as byte offset could split multi-byte characters. → Mitigation: Use `char_indices` for cursor movement to ensure we always land on character boundaries.
- **[Input buffer shared state]** `app.input_buffer` is used by multiple modes (NLP chat, search, reply). → Mitigation: Clear the buffer on mode entry/exit as the code already does. No structural change needed.
