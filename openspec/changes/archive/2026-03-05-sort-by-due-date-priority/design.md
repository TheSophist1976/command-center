## Context

The TUI's `filtered_indices()` method returns task indices in storage order (the order they appear in the task file). There is no sorting applied after filtering by view and user filters. The `Priority` enum variants are ordered Critical, High, Medium, Low in source but do not derive `Ord`. There is no CLI list command — all task display goes through the TUI.

## Goals / Non-Goals

**Goals:**
- Sort all view results by due date ascending (earliest first), `None` last
- Break ties by priority descending (Critical > High > Medium > Low)
- Apply sorting uniformly across all views
- Keep sorting logic in a single reusable function

**Non-Goals:**
- User-configurable sort order or sort direction
- Sort-by column headers or interactive sort toggling
- CLI list command (doesn't exist)

## Decisions

### 1. Add `Ord`/`PartialOrd` to `Priority` enum

Derive `PartialOrd` and `Ord` on `Priority`. The enum variants are already declared in descending importance order (Critical first, Low last), so the derived `Ord` gives Critical < High < Medium < Low by discriminant. For sorting priority descending, we reverse the comparison.

**Alternative**: Manual `Ord` impl or a `priority_rank()` method. Rejected — deriving is simpler and the variant order is stable.

### 2. Sort inside `filtered_indices()`

Add a sort step at the end of `filtered_indices()` before returning. This is the single chokepoint where all views get their displayable task list, so sorting here covers every view automatically.

The sort comparator:
1. Tasks with `due_date: Some(d)` come before `due_date: None`
2. Among tasks with due dates, sort ascending by date
3. Within the same due date (or both `None`), sort by priority descending (Critical first)

**Alternative**: Sort in the rendering function. Rejected — `filtered_indices` is used for selection tracking too, so sorting must happen there to keep indices consistent.

### 3. Use `sort_by` with a composite key

Use `Vec::sort_by` on the collected indices with a closure that compares `(due_date, priority)`. For due date: map `Some(d)` to `(0, d)` and `None` to `(1, NaiveDate::MAX)` to push no-due-date tasks to the end. For priority: use `Reverse(priority)` to get descending order.

## Risks / Trade-offs

- **[Selection stability]** Sorting changes which index maps to which row. The existing `app.selected` is an offset into the filtered list, not a task ID, so selection will naturally follow the new order. No risk of pointing to the wrong task. → No mitigation needed.
- **[Test updates]** Tests that create multiple tasks and assert on position may break if they assumed insertion order. → Update affected tests to match the new sort order.
- **[Performance]** Sorting happens on every render frame via `filtered_indices()`. For typical task counts (< 1000), this is negligible. → No mitigation needed.
