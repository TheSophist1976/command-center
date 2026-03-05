## Context

The `Recurrence` enum currently has two variants: `Interval { unit, count }` for simple repeating intervals and `NthWeekday { n, weekday }` for nth-weekday-of-month patterns. The string format uses colon-separated parts parsed in `FromStr`. The `next_due_date` function dispatches on variant. The TUI's `format_recurrence_display` formats each variant for the UI column.

The parser serializes recurrence as `recur:<value>` in task metadata comments, using the `Display` impl.

## Goals / Non-Goals

**Goals:**
- Add a `WeeklyOn { weekday, every_n_weeks }` variant to `Recurrence`
- Parse `weekly:DAY` (shorthand for every 1 week) and `weekly:N:DAY` formats
- Calculate next due date: find the next occurrence of the weekday after the base date, advancing by N weeks total
- Display as "Weekly (Fri)" or "Every 2 Weeks (Mon)"
- Round-trip through metadata serialization

**Non-Goals:**
- Daily-on-weekday patterns (e.g., "every weekday") — different feature
- Multiple weekdays per recurrence (e.g., "every Mon and Wed")
- Calendar-aware holiday skipping

## Decisions

### 1. New variant `WeeklyOn { weekday: Weekday, every_n_weeks: u32 }`

Add as a third variant on `Recurrence`. This keeps it distinct from `Interval` (which doesn't anchor to a weekday) and `NthWeekday` (which is monthly).

**Alternative**: Extend `Interval` with an optional weekday field. Rejected — makes the common case more complex and muddies the semantics.

### 2. Parse format reuses existing colon convention

`weekly:DAY` → `WeeklyOn { weekday: DAY, every_n_weeks: 1 }`
`weekly:N:DAY` → `WeeklyOn { weekday: DAY, every_n_weeks: N }`

This creates an ambiguity in the 2-part case: `weekly:2` means `Interval { unit: Weekly, count: 2 }` (existing), while `weekly:fri` means `WeeklyOn { weekday: Fri, every_n_weeks: 1 }` (new). Disambiguate by checking if the second part parses as a number vs a weekday name.

For the 3-part case: `weekly:N:DAY` is new (currently only `monthly:N:DAY` exists). Check `parts[0]` to route correctly.

### 3. Next due date calculation

Given base date and `WeeklyOn { weekday, every_n_weeks }`:
1. Find the next occurrence of `weekday` strictly after `base`
2. If `every_n_weeks > 1`, the interval between occurrences is N weeks. Find next occurrence that is a multiple of N weeks from the anchor. For simplicity, use the next occurrence of the weekday, then add `(every_n_weeks - 1) * 7` days. This gives a consistent N-week gap from the previous due date.

Actually simpler: advance base by 7 * every_n_weeks days, then snap to the target weekday within that week. But the simplest correct approach: find next `weekday` after base, then that's it for N=1. For N>1, add (N-1) weeks to that date.

Wait — that's not right either. For N=2, if the last due date was a Friday, the next should be 2 Fridays later, not 1 Friday + 1 week. The simplest: base + (7 * every_n_weeks) days if base is already on the target weekday. If base is not on the target weekday (no due date case), find next occurrence of weekday.

Simplest correct approach:
- If base is already the target weekday: next = base + (7 * N) days
- Otherwise: find next occurrence of weekday after base (for initial scheduling)

### 4. Display format

- `WeeklyOn { weekday: Fri, every_n_weeks: 1 }` → "Weekly (Fri)"
- `WeeklyOn { weekday: Mon, every_n_weeks: 2 }` → "Every 2 Weeks (Mon)"

Consistent with existing "Every N Weeks" for Interval and "Monthly (3rd Thu)" for NthWeekday.

## Risks / Trade-offs

- **[Parse ambiguity]** `weekly:fri` vs `weekly:2` — mitigated by checking if second part is numeric or alphabetic. Clear delimiter.
- **[N-week drift]** If a user completes a task late, the next occurrence uses the completion's due date as base, so it stays anchored to the right weekday. No drift risk.
- **[Serialization backward compat]** Old versions won't recognize `weekly:1:fri` in metadata. They'll fail to parse it. → Acceptable; this is an additive format change.
