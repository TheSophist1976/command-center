## Why

Users currently must type due dates in `YYYY-MM-DD` format, which is tedious when thinking in terms of days of the week. Accepting weekday names like "Monday" or "fri" as due date input reduces friction and feels more natural.

## What Changes

- The TUI detail edit panel due date field SHALL accept weekday names (e.g., `Monday`, `tuesday`, `fri`) in addition to `YYYY-MM-DD`
- A weekday name SHALL resolve to the **next future occurrence** of that day — if today is Tuesday and the user types `Monday`, the due date is set to the following Monday (7+ days away, never today)
- Full names (`Monday`) and three-letter abbreviations (`mon`) SHALL both be accepted, case-insensitively
- The resolved date SHALL be stored and displayed as `YYYY-MM-DD` (no change to storage format)
- The error message on invalid input SHALL be updated to reflect that weekday names are now valid

## Capabilities

### New Capabilities

- `due-date-weekday-input`: Parsing weekday names to the next occurrence NaiveDate, and integrating that parser into all due date input paths in the TUI

### Modified Capabilities

_(none — storage format and task data model are unchanged)_

## Impact

- `src/tui.rs`: Two due date parse sites updated — detail edit panel save (line ~2620) and NLP set_fields processing (line ~2692)
- `src/parser.rs` or new utility: `parse_due_date_input(s: &str, today: NaiveDate) -> Option<NaiveDate>` function added
- No changes to `tasks.md` file format, serialization, or `AGENTS.md`
