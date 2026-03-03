## Why

The recurring-tasks feature is fully implemented, but there's no quick way to see all recurring tasks at a glance. Users need a dedicated view to manage and review their recurring tasks without manually scanning through all tasks.

## What Changes

- Add a `Recurring` variant to the `View` enum in `src/tui.rs` that filters to only tasks with a `recurrence` field set
- Insert `Recurring` into the view cycle (after NoDueDate, before Today)
- Show both open and completed recurring tasks (like the All view, but filtered to recurrence)

## Capabilities

### New Capabilities

(none — this extends an existing capability)

### Modified Capabilities

- `tui-views`: Adding a new Recurring view variant to the existing view system, updating the cycle order and display name

## Impact

- `src/tui.rs`: View enum, matches(), next(), prev(), display_name(), and view-related tests
