## Why

Open tasks with past due dates are invisible in the Today, Weekly, Monthly, and Yearly views because each view only matches tasks whose due date falls within its time window. An overdue task silently disappears from every view except All, making it easy to lose track of tasks that need urgent attention.

## What Changes

- Modify the view matching logic so that open tasks with a due date in the past (before today) are included in all time-based views (Today, Weekly, Monthly, Yearly)
- Completed tasks with past due dates remain hidden (only visible in All view, as before)
- NoDueDate view is unaffected (it only shows tasks with no due date)

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `tui-views`: Overdue open tasks shown in all time-based views

## Impact

- `src/tui.rs` — Modify `View::matches()` to include overdue open tasks in time-based views
