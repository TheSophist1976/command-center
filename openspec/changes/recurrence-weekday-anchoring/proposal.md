## Why

Users want to express recurrences like "every week on Friday" or "every 2 weeks on Monday". The current `Interval` variant repeats from the last due date without anchoring to a specific weekday, and the `NthWeekday` variant only supports nth-weekday-of-month patterns. There is no way to create a weekly (or multi-week) recurrence pinned to a specific day of the week.

## What Changes

- Add a new `Recurrence` variant `WeeklyOn { weekday, every_n_weeks }` that represents recurring on a specific weekday every N weeks
- Parse format: `weekly:N:DAY` (e.g., `weekly:1:fri` for "every week on Friday", `weekly:2:mon` for "every 2 weeks on Monday")
- Also accept shorthand `weekly:DAY` as equivalent to `weekly:1:DAY`
- Calculate next due date by finding the next occurrence of the target weekday, then advancing by (N-1) additional weeks if N > 1
- Display as "Weekly (Fri)", "Every 2 Weeks (Mon)", etc.
- NLP system should understand phrases like "every Friday" or "every 2 weeks on Monday"

## Capabilities

### New Capabilities

- `recurrence-weekday-anchor`: Defines the weekday-anchored weekly recurrence variant, its parsing, serialization, next-date calculation, and display formatting

### Modified Capabilities

- `task-recurrence`: The `Recurrence` enum gains a new variant; `FromStr`, `Display`, and `next_due_date` are extended

## Impact

- `src/task.rs`: New `Recurrence::WeeklyOn` variant, updated `FromStr`/`Display`/`next_due_date`
- `src/tui.rs`: Updated `format_recurrence_display` for the new variant
- `src/parser.rs`: Serialization/deserialization of `weekly:N:DAY` metadata format
- `src/nlp.rs`: System prompt updated to recognize weekday-anchored recurrence phrases
