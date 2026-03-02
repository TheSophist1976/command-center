## Context

The `View::matches()` method determines whether a task appears in a given view. Each time-based view (Today, Weekly, Monthly, Yearly) checks if the task's due date falls within its window. Open tasks with past due dates fall outside all windows and become invisible.

## Goals / Non-Goals

**Goals:**
- Show overdue open tasks in all time-based views
- Keep completed tasks hidden from non-All views (existing behavior)
- Keep NoDueDate view unchanged

**Non-Goals:**
- Visual distinction for overdue tasks (highlighting, color) — can be added later
- Sorting overdue tasks to the top

## Decisions

### 1. Add overdue check at the top of `View::matches()`

After the existing completed-task filter, add: if the task is open, has a due date, and that date is before today, return true for all views except All (already returns true) and NoDueDate (should not show dated tasks).

This is a single guard clause — minimal change, no restructuring needed.

### 2. Update existing tests

The test `today_view_hides_overdue_task` currently asserts that overdue tasks are hidden. This test needs to be updated to assert the opposite: overdue open tasks are shown. Add a new test confirming overdue *completed* tasks remain hidden.

## Risks / Trade-offs

- **More tasks visible**: Users may see more tasks than expected in narrower views. This is the desired behavior — overdue tasks should not be silently hidden.
- **No visual distinction**: Overdue tasks look the same as current-window tasks. This is acceptable for now and can be enhanced separately.
