## Context

The detail panel (`Tab` to toggle) shows all fields of the selected task in a read-only layout. Editing currently requires leaving the panel and using per-field keys (`e` for title, `r` for description, `t` for tags, `p` for priority). The existing editing modes use a shared `input_buffer` in the footer. This change makes the detail panel itself the editing surface.

## Goals / Non-Goals

**Goals:**
- Let users edit task fields inline within the detail panel
- Navigate between editable fields in the panel
- Track dirty state and prompt to save/discard when navigating away
- Keep existing per-field edit keys (`e`, `r`, `t`, `p`) working unchanged

**Non-Goals:**
- Editing ID, Created, or Updated fields (these are system-managed)
- Multi-line text editing for description (single-line input, same as current `r` key)
- Replacing the existing per-field edit shortcuts — those remain as quick alternatives

## Decisions

### 1. New mode: `EditingDetailPanel`

Add a single `EditingDetailPanel` mode that activates when the user presses `Enter` while the detail panel is visible. This mode takes over the panel rendering and input handling. `Esc` exits back to Normal mode (with dirty check).

**Why a single mode instead of per-field modes:** The detail panel is a cohesive editing context. A single mode simplifies state management — one entry point, one exit point, one dirty check.

### 2. Editable fields and field index

The panel presents these fields in order, navigable by `j`/`k` or `Tab`/`Shift-Tab`:

| Index | Field       | Input type         |
|-------|-------------|--------------------|
| 0     | Title       | Text input         |
| 1     | Description | Text input         |
| 2     | Priority    | Cycle (c/h/m/l)    |
| 3     | Status      | Toggle (Enter)     |
| 4     | Due Date    | Text input (YYYY-MM-DD or empty to clear) |
| 5     | Project     | Text input (empty to clear) |
| 6     | Tags        | Text input (space-separated) |

A `detail_field_index: usize` field on `App` tracks which field is focused. Non-text fields (Priority, Status) use special key handling instead of the input buffer.

### 3. Draft state with dirty tracking

Add a `detail_draft: Option<DetailDraft>` field to `App`. The draft struct holds editable copies of all field values:

```
struct DetailDraft {
    title: String,
    description: String,
    priority: Priority,
    status: Status,
    due_date: String,       // raw string, parsed on save
    project: String,
    tags: String,            // space-separated
    original_task_id: u32,   // to detect which task this draft belongs to
}
```

The draft is created from the current task when entering `EditingDetailPanel`. It is `Some` while editing, `None` otherwise. Dirty = draft differs from the original task values.

### 4. Editing UX within a field

When a text field is focused:
- The `input_buffer` is loaded with that field's draft value
- Typing modifies `input_buffer` as usual
- Moving to another field (`j`/`k`/`Tab`) commits the `input_buffer` value back into the draft and loads the next field's value into `input_buffer`
- The currently focused field renders with highlight styling and shows a cursor

For Priority: pressing `c`/`h`/`m`/`l` cycles the value directly in the draft.
For Status: pressing `Enter` or `Space` toggles between open/done in the draft.

### 5. Save-on-navigate prompt

When the user tries to leave the editing context (pressing `Esc`, navigating to a different task with `j`/`k` in Normal mode while panel is open) and the draft is dirty:

Add a `ConfirmingDetailSave` mode that shows a footer prompt: `"Unsaved changes. [s]ave  [d]iscard  [c]ancel"`

- `s` — Apply the draft to the task, save to disk, exit editing
- `d` — Discard the draft, exit editing
- `c` or `Esc` — Cancel, stay in editing mode

If the draft is clean (no changes), exiting is immediate with no prompt.

### 6. Entering edit mode

In Normal mode with `show_detail_panel` true, pressing `Enter` on the selected task enters `EditingDetailPanel` mode instead of toggling completion. This is a context-sensitive override — `Enter` toggles completion when the panel is hidden, enters detail editing when the panel is visible.

`j`/`k` in Normal mode continue to navigate the task table (they do not enter editing). The user must press `Enter` to start editing.

### 7. Panel rendering in edit mode

The detail panel switches from a plain `Paragraph` to a structured layout with one line per field. The focused field is highlighted (e.g., reverse video or a `>>` marker). The currently-editing text field shows the input buffer with a cursor indicator.

```
 Title:       [Buy groceries_]        ← focused, cursor after 's'
 Description: Pick up milk and eggs
 Priority:    medium
 Status:      open
 Due Date:    2026-03-01
 Project:     (none)
 Tags:        shopping errands
```

### 8. Navigation interception when dirty

When `detail_draft` is `Some` and dirty, intercepting `j`/`k` in Normal mode (task navigation) triggers the `ConfirmingDetailSave` prompt before allowing the selection to change. If the user saves or discards, the navigation proceeds. If cancelled, the selection stays.

To implement this: the `j`/`k` handler in Normal mode checks `detail_draft.is_some()` and dirty state before moving `app.selected`. If dirty, it stores the intended direction and enters `ConfirmingDetailSave`. On resolution, the stored direction is applied.

Add `pending_navigation: Option<NavDirection>` to `App` where `NavDirection` is `Up` or `Down`.

## Risks / Trade-offs

- **`Enter` behavior change when panel is open**: Currently `Enter` toggles completion. With the panel open, `Enter` enters editing instead. This is a behavioral change that users must discover. Mitigated by updating the footer hints to show `Enter:edit` when the panel is visible.
- **Complexity in mode transitions**: `EditingDetailPanel` → `ConfirmingDetailSave` → back to `EditingDetailPanel` or `Normal` adds mode transition complexity. Keeping it to just two new modes (edit + confirm) limits the surface.
- **Date parsing**: Users type due dates as raw strings. Invalid dates need graceful handling — show a status message and keep the field focused rather than silently discarding.
- **Input buffer sharing**: The existing `input_buffer` is shared across all input modes. This works because only one mode is active at a time, but field-switching within `EditingDetailPanel` must carefully save/restore the buffer.
