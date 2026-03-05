## Context

The `View::matches` method in `src/tui.rs` controls which tasks appear in each view. Currently, done tasks are allowed in both the All and Recurring views (line 95). The Recurring view filter (line 99-100) only checks `recurrence.is_some()` with no status filter.

## Goals / Non-Goals

**Goals:**
- Exclude done tasks from the Recurring view
- Keep the All view unchanged (still shows everything)

**Non-Goals:**
- Adding a toggle to show/hide done tasks in the Recurring view
- Changing any other view's filtering behavior

## Decisions

### Remove Recurring from the done-task exception
The early return at line 94-95 that allows done tasks in `View::All` and `View::Recurring` should be changed to only allow done tasks in `View::All`. This single change handles everything because the done task is rejected before reaching the Recurring-specific filter.

## Risks / Trade-offs

- Users who want to see completed recurring tasks can still find them in the All view
