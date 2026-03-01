## Why

The detail panel (toggled with `Tab`) currently only displays task fields in a read-only format. Users must close the panel and use single-key edit modes (`e`, `r`, `t`, `p`) to change individual fields. This breaks the workflow — the panel is the natural place to inspect *and* edit a task. Inline editing in the detail panel would let users update multiple fields without leaving the panel, and a save-on-navigate prompt would prevent accidental data loss.

## What Changes

- Make the detail panel fields editable: the user can navigate between fields and edit them inline
- Add field-level navigation within the detail panel (e.g., `j`/`k` or `Tab` to move between fields)
- Enter edit mode on a field by pressing `Enter` or just start typing
- Dirty tracking: detect when field values have been modified from the original task state
- Save prompt when navigating to a different task (via `j`/`k` in the table) while the panel has unsaved changes — user can save, discard, or cancel navigation
- Persist changes to disk when the user explicitly saves

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `tui`: Adding inline editing in the detail panel with field navigation, dirty tracking, and a save-on-navigate confirmation prompt

## Impact

- `src/tui.rs` — Modify detail panel rendering to support editable fields, add new mode(s) for detail panel editing, add dirty tracking state to `App`, add save/discard/cancel confirmation dialog, modify navigation handlers to check for unsaved changes
