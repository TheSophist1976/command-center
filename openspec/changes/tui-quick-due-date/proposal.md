## Why

Setting a task's due date in the TUI currently requires leaving the TUI and using the CLI. Users need a fast way to assign common due dates (today, next week, next month, next quarter) to the selected task with a single keypress, matching the quick-edit patterns already established for priority (`p`) and status (`Enter`).

## What Changes

- Add Shift-letter keybindings directly in Normal mode for due date assignment:
  - `T` — set due date to today
  - `W` — set due date to next week (+7 days)
  - `M` — set due date to next month
  - `Q` — set due date to next quarter (+3 months)
  - `X` — clear due date (set to None)
- The selected task's `due_date` field is updated and persisted immediately
- A status message confirms the change (e.g., "Due: 2026-03-06")

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `tui`: Adding Shift-letter keybindings in Normal mode for single-keypress due date assignment

## Impact

- `src/tui.rs` — New keybindings in `handle_normal`, due date update logic, footer hints
- Uses existing `chrono` dependency for date arithmetic
