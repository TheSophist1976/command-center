## Context

The task table dynamically shows/hides columns based on whether any filtered task has the relevant data. The existing `show_recur` flag controls the "↻" column. A new `show_recur` check (already true) will also control a "Pattern" column that uses the existing `format_recurrence_display()` function.

## Goals / Non-Goals

**Goals:**
- Show the recurrence pattern as a human-readable string in a dedicated column
- Only show the column when at least one filtered task has a recurrence

**Non-Goals:**
- Changing the "↻" indicator column behavior
- Making the pattern column editable

## Decisions

### Reuse `show_recur` flag for both columns
Both the "↻" and "Pattern" columns appear/hide together since they're both about recurrence. No new flag needed.

### Use `format_recurrence_display()` for cell text
The function already produces clean output ("Daily", "Weekly", "3rd Thu", etc.). No new formatting logic needed.

## Risks / Trade-offs

- Extra column width — mitigated by using `Constraint::Min(8)` which is compact enough for patterns like "Weekly" or "3rd Thu"
