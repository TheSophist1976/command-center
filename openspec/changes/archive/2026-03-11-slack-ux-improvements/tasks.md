## 1. App State

- [x] 1.1 Add `slack_preview_visible: bool` field to `App` struct, defaulting to `true`
- [x] 1.2 Add `slack_reply_cursor: usize` field to `App` struct for cursor position tracking in reply input

## 2. Preview Pane

- [x] 2.1 Create `draw_slack_preview` function that renders the full message text with sender/channel/time header using `Paragraph` with `Wrap { trim: false }`
- [x] 2.2 Update `draw_slack_inbox` layout to split vertically (~60% table, ~40% preview with `Min(3)`) when `slack_preview_visible` is true
- [x] 2.3 When `slack_preview_visible` is false, render the table in the full content area (current behavior)
- [x] 2.4 Add `p` keybinding in `handle_slack_inbox` to toggle `slack_preview_visible`

## 3. Reply Panel

- [x] 3.1 Create `draw_slack_reply_panel` function that shows the original message as a `>` quote block above a bordered text input area labeled "Reply to #channel-name:"
- [x] 3.2 Update the `SlackReplying` draw path to render the reply panel in the preview pane area instead of using the status bar
- [x] 3.3 Render a cursor indicator (highlighted character or underscore) at `slack_reply_cursor` position in the input text

## 4. Cursor Movement

- [x] 4.1 Add Left/Right arrow key handling in `SlackReplying` mode to move `slack_reply_cursor` by character boundaries using `char_indices`
- [x] 4.2 Add Home/End key handling to jump cursor to beginning/end of input
- [x] 4.3 Update character insertion to insert at `slack_reply_cursor` position instead of appending, then advance cursor
- [x] 4.4 Update Backspace to delete character before `slack_reply_cursor` position and move cursor back

## 5. Footer Updates

- [x] 5.1 Update the SlackInbox footer to include `p:preview` in the keybinding hints
- [x] 5.2 Add a separate footer for SlackReplying mode showing `Enter:send  Esc:cancel  ←→:move cursor  Home/End:jump`

## 6. Mode Transitions

- [x] 6.1 On entering `SlackReplying` (pressing `r`), reset `slack_reply_cursor` to 0 and clear input buffer
- [x] 6.2 On exiting `SlackReplying` (Enter send or Esc cancel), restore preview pane display
- [x] 6.3 Ensure `slack_preview_visible` state persists across mode transitions (inbox → reply → inbox)

## 7. Testing

- [x] 7.1 Add integration test verifying preview pane renders full message text for selected inbox message
- [x] 7.2 Add integration test verifying cursor movement: insert at cursor, backspace at cursor, Home/End
- [x] 7.3 Add integration test verifying `p` toggle hides/shows the preview pane
- [x] 7.4 Verify existing Slack inbox keybindings (j/k, Enter/d, o, S, Esc) still work unchanged
