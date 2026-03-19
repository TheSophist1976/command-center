## Context

The note editor (`NoteEditor` in `src/tui.rs`) is a terminal UI widget for inline markdown editing. It stores content as `Vec<String>` (logical lines) with `cursor_row`/`cursor_col` position and a `viewport_offset` (logical line index for scrolling).

Currently:
- `handle_note_editor` has no `KeyCode::Home` or `KeyCode::End` arms — those keys are silently ignored.
- `draw_note_editor` truncates long lines at `text_width` chars (line 3724–3728), making content past the terminal edge invisible.
- `ensure_cursor_visible` counts logical rows only, so it becomes inaccurate once a line wraps.

All changes are confined to `src/tui.rs`. No storage, CLI, or note model changes are needed.

## Goals / Non-Goals

**Goals:**
- `Home` moves cursor to column 0 on the current logical line
- `End` moves cursor to the last character of the current logical line
- Long lines wrap visually instead of being truncated
- Cursor screen position accounts for wrapped visual rows
- Scrolling stays correct under word wrap

**Non-Goals:**
- Sub-line `viewport_offset` (pixel-level scrolling within a wrapped logical line)
- Soft vs. hard wrap distinction
- Changes to the underlying line storage model
- Wrap-aware `move_up` / `move_down` (they continue navigating logical lines)

## Decisions

### Home / End as methods on `NoteEditor`

Add `move_to_line_start` and `move_to_line_end` as methods on the struct, then wire `KeyCode::Home` and `KeyCode::End` in `handle_note_editor`. This mirrors the existing pattern for `move_left`, `move_right`, etc.

Alternative considered: Inline the logic directly in the match arm. Rejected — keeping logic in `NoteEditor` keeps the struct self-contained and unit-testable.

### Word wrap in the renderer, not in storage

Wrap is applied only at render time. The logical line model (`Vec<String>`) is unchanged. `text_width` (derived from the terminal area) determines how many chars fit per visual row.

Alternative considered: Storing pre-wrapped display lines. Rejected — it would require re-wrapping on every resize and complicates cursor tracking.

### Visual row calculation: `cols_per_row = text_width.max(1)`; visual rows per line = `max(1, ceil(char_count / cols_per_row))`

An empty line always occupies 1 visual row.

### Cursor screen position in `draw_note_editor`

```
visual_row_within_line = cursor_col / text_width (or 0 if text_width == 0)
visual_col_within_row  = cursor_col % text_width (or cursor_col if text_width == 0)
cursor_screen_row      = (sum of visual rows for logical lines [viewport_offset, cursor_row))
                         + visual_row_within_line
cursor_x               = inner_area.x + line_num_width + visual_col_within_row
cursor_y               = inner_area.y + cursor_screen_row
```

Line numbers continue to display on the first visual row of each logical line; continuation rows have a blank number gutter.

### `ensure_cursor_visible` receives `text_width` as a second parameter

Scrolling must stay aware of how many visual rows each logical line occupies. Adding `text_width: usize` to the method signature is the minimal change. The call site already computes `text_width` before calling this method.

`viewport_offset` stays as a logical line index. Sub-line scrolling (scrolling within a wrapped line) is out of scope — the viewport jumps to the logical line boundary.

Scrolling algorithm:
1. If `cursor_row < viewport_offset`, set `viewport_offset = cursor_row` (scroll up).
2. Otherwise, accumulate visual rows from `viewport_offset` forward until reaching the cursor's visual row. If that exceeds `visible_height`, advance `viewport_offset` one logical line at a time until the cursor fits.

This can place the cursor at a non-last row if the cursor's line itself wraps beyond one screen, which is acceptable for typical note lengths.

## Risks / Trade-offs

- **Long single lines (> `visible_height × text_width` chars)**: The cursor may not be scrollable into view using the logical-line viewport. Mitigation: accepted as an edge case for typical notes; a sub-line viewport would add significant complexity.
- **`ensure_cursor_visible` signature change**: Any test that calls the method directly must pass `text_width`. Existing unit tests use `ensure_cursor_visible(3)`; these will need a second argument (e.g., `80`). Mitigation: update tests as part of this change.
- **Markdown styling + word wrap interaction**: `md_style::style_markdown_line` is called per visual row. Continuation rows of a wrapped line pass a slice of the original line, which may land mid-token (e.g., inside `**bold**`). Mitigation: style correctness on wrapped continuations is best-effort; markdown spans that exceed a single visual row are an existing edge case.

## Migration Plan

No data migration. Changes are purely in the TUI rendering and input handling layer. The binary is the only artifact; deployment is a `cargo build --release` and binary swap.

Rollback: revert the `src/tui.rs` changes and rebuild.

## Open Questions

- Should `Home` / `End` in a wrapped visual row move within the visual row, or always jump to the logical line boundary? **Decision**: jump to logical line boundary for simplicity and consistency with most terminal editors.
