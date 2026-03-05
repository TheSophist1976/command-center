## Why

When a user sets a recurrence to "every 3 months" or "every 2 weeks", the NLP parser fails because the `Recurrence` type only supports fixed single-unit intervals (daily, weekly, monthly, yearly). Users need arbitrary interval counts for patterns like quarterly, biweekly, or every N days.

## What Changes

- Extend `Recurrence::Interval` to carry a count (e.g., `count: 3` for "every 3 months")
- Support serialization format `"daily:3"`, `"weekly:2"`, `"monthly:3"` with backward-compatible parsing of existing `"daily"`, `"weekly"`, etc. (count defaults to 1)
- Update `next_due_date` to multiply the interval by count
- Update the TUI display to show "Every 3 Months" instead of "Monthly" when count > 1
- Update the NLP recurrence parser prompt to recognize custom intervals

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `task-recurrence`: Change `Interval(IntervalUnit)` to `Interval { unit: IntervalUnit, count: u32 }`, update Display/FromStr/next_due_date for custom counts
- `tui`: Update `format_recurrence_display` for count > 1

## Impact

- `src/task.rs`: Enum variant change, Display, FromStr, `next_due_date`
- `src/tui.rs`: `format_recurrence_display`, all match arms on `Recurrence::Interval`
- `src/nlp.rs`: `parse_recurrence_nlp` system prompt
- All existing tests using `Recurrence::Interval(unit)` need updating to new struct syntax
