## Context

The TUI has a `View` enum with 6 variants (Today, All, Weekly, Monthly, Yearly, NoDueDate) cycled via `v`/`V`. Now that recurring tasks exist (`task.recurrence: Option<Recurrence>`), users need a view to see all of them at once.

## Goals / Non-Goals

**Goals:**
- Add a Recurring view that shows only tasks with a recurrence pattern set
- Integrate into the existing view cycle naturally

**Non-Goals:**
- No calendar/forecast of future occurrences
- No recurring-specific sorting or grouping

## Decisions

### 1. Filter predicate: `task.recurrence.is_some()`
Show any task (open or done) that has a recurrence set. This matches the All view's "show everything" philosophy but scoped to recurring tasks. Including done tasks lets users see their completed recurring patterns.

### 2. Cycle position: after NoDueDate
Order: Today → All → Weekly → Monthly → Yearly → NoDueDate → Recurring → (back to Today). Placing Recurring at the end keeps it out of the main time-based sequence — it's a secondary filter, not a time window.

### 3. Display name: "Recurring"
Short, clear, consistent with the other view names.

## Risks / Trade-offs

- [Overdue handling] Recurring view does not use the overdue-in-all-views logic since it filters by recurrence, not time. This is intentional — the view answers "what repeats?" not "what's overdue?" → No mitigation needed.
