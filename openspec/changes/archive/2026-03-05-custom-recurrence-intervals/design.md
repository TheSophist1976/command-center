## Context

The `Recurrence` enum in `src/task.rs` has `Interval(IntervalUnit)` as a tuple variant. Every place that matches on it uses `Recurrence::Interval(unit)`. The serialized format is plain strings like `"daily"`, `"weekly"`. The `next_due_date` function hardcodes +1 unit. The NLP prompt in `src/nlp.rs` only lists the four fixed intervals.

## Goals / Non-Goals

**Goals:**
- Support arbitrary positive integer counts for interval recurrence (e.g., every 3 months)
- Backward-compatible parsing: existing `"daily"` still works (count = 1)
- Correct next occurrence calculation with count multiplier

**Non-Goals:**
- Supporting fractional intervals (every 1.5 weeks)
- Supporting count on NthWeekday (every 2nd month's 3rd Thursday)

## Decisions

### Change `Interval(IntervalUnit)` to `Interval { unit: IntervalUnit, count: u32 }`

Named fields are clearer than a tuple when two values are involved. `count` is `u32` — always >= 1.

### Serialization format: `"unit:count"` with count optional

- `"daily"` → `Interval { unit: Daily, count: 1 }` (backward compat)
- `"monthly:3"` → `Interval { unit: Monthly, count: 3 }`

Disambiguation with `"monthly:N:DAY"` (NthWeekday): if 2 parts and second part is numeric → Interval with count. If 3 parts → try NthWeekday first. No ambiguity since NthWeekday always has 3 parts.

### Display: count 1 omits count, count > 1 appends `":count"`

This preserves backward compat for serialized files.

### TUI display: "Every N Units" for count > 1

- count == 1: `"Daily"`, `"Weekly"`, `"Monthly"`, `"Yearly"` (unchanged)
- count > 1: `"Every 3 Months"`, `"Every 2 Weeks"`, `"Every 5 Days"`, `"Every 2 Years"`

## Risks / Trade-offs

- Every `Recurrence::Interval(unit)` match arm in the codebase needs updating — mechanical but widespread
- Existing task files remain compatible since count-1 serialization is identical
