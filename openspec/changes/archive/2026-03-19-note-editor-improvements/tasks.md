## 1. Home / End key navigation

- [x] 1.1 Add `move_to_line_start` method on `NoteEditor`: sets `cursor_col` to 0
- [x] 1.2 Add `move_to_line_end` method on `NoteEditor`: sets `cursor_col` to `lines[cursor_row].len()`
- [x] 1.3 Wire `KeyCode::Home` in `handle_note_editor` to call `move_to_line_start`
- [x] 1.4 Wire `KeyCode::End` in `handle_note_editor` to call `move_to_line_end`

## 2. Word-wrap rendering in `draw_note_editor`

- [x] 2.1 Replace the truncation logic (lines ~3724–3728) with a word-wrap loop that splits each logical line into chunks of `text_width` chars
- [x] 2.2 Render the line number gutter on only the first visual row of each logical line; continuation rows get a blank gutter
- [x] 2.3 Update cursor screen-position calculation: `visual_row_within_line = cursor_col / text_width`, `visual_col_within_row = cursor_col % text_width`, `cursor_screen_row` = sum of visual rows for lines `[viewport_offset, cursor_row)` + `visual_row_within_line`

## 3. Scroll fix for word wrap in `ensure_cursor_visible`

- [x] 3.1 Add `text_width: usize` as a second parameter to `ensure_cursor_visible`
- [x] 3.2 Update the scroll-down branch to accumulate visual rows (not logical rows) from `viewport_offset` forward until the cursor's visual row fits within `visible_height`
- [x] 3.3 Update the call site in `draw_note_editor` to pass the computed `text_width`

## 4. Tests

- [x] 4.1 Add unit tests for `move_to_line_start`: cursor at mid-line moves to 0; cursor already at 0 is a no-op
- [x] 4.2 Add unit tests for `move_to_line_end`: cursor at start moves to line length; empty line stays at 0
- [x] 4.3 Update existing unit tests that call `ensure_cursor_visible` to pass a second `text_width` argument (e.g., `80`)
- [x] 4.4 Add unit test for visual-row counting: a line of 10 chars with `text_width = 4` produces 3 visual rows
