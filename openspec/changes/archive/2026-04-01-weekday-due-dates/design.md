## Context

Due dates are currently entered as `YYYY-MM-DD` strings in two TUI code paths:

1. **Detail edit panel save** (`tui.rs` ~2620): validates the `draft.due_date` string before applying it to the task
2. **NLP set_fields processing** (`tui.rs` ~2692): applies a `due_date` string returned from Claude

Both sites call `NaiveDate::parse_from_str(s, "%Y-%m-%d")` directly. If parsing fails, they show an error and abort. There is no shared parsing utility — each site is independent.

The weekday name → date resolution rule: given weekday name W and today T, find the next calendar date after T whose weekday equals W. If today is Tuesday and the user types "Monday", the result is the Monday of the following week (6 days away). The resolved day is **never today** — it always points to a future occurrence.

## Goals / Non-Goals

**Goals:**
- Accept full weekday names (`Monday`…`Sunday`) and three-letter abbreviations (`mon`…`sun`), case-insensitively, in all due date text input fields in the TUI
- Resolve them to the next future occurrence (never today)
- Centralize due date input parsing into one function so both call sites share the logic
- Keep storage format (`YYYY-MM-DD`) and the `Task` struct unchanged

**Non-Goals:**
- Parsing relative phrases like "next Monday", "this Friday", "in 3 days" — that is NLP territory
- Changing how Claude/NLP produces due dates in responses (Claude already outputs `YYYY-MM-DD`)
- CLI subcommand argument parsing (no `--due monday` flag)
- Accepting two-letter abbreviations (`mo`, `tu`, etc.)

## Decisions

### 1. Shared `parse_due_date_input` utility in `parser.rs`

Add `pub fn parse_due_date_input(s: &str, today: NaiveDate) -> Option<NaiveDate>` to `src/parser.rs`. It:
1. Tries `NaiveDate::parse_from_str(s, "%Y-%m-%d")` — returns `Some(date)` on success
2. Maps weekday names/abbreviations to `chrono::Weekday` — returns `Some(next_occurrence)` if matched
3. Returns `None` for anything else

Both TUI call sites replace their inline `NaiveDate::parse_from_str` with this function.

**Alternative considered**: New `src/date_utils.rs` module. Rejected — `parser.rs` already owns date-adjacent string parsing (task metadata) and adding one function there avoids a new file.

### 2. "Next future occurrence" semantics — never today

If today is Monday and the user types "Monday", the result is next Monday (7 days away), not today. This matches the user's stated intent: typing a weekday name means "the upcoming occurrence", and "today" already has its own shorthand in the NLP interface.

**Alternative considered**: Allow resolving to today if today matches. Rejected per the user's explicit requirement ("if it is Tuesday and I say Monday, it should be due next week" — implying the same logic applies when the weekday matches today).

### 3. Error message update

The validation error currently reads `"Invalid date format (use YYYY-MM-DD)"`. It SHALL be updated to `"Invalid date (use YYYY-MM-DD or a weekday name)"` to inform users of the new option.

## Risks / Trade-offs

- **Ambiguity of "mar" etc.**: Three-letter month abbreviations that double as something else are not an issue here since we only match three-letter day abbreviations (mon/tue/wed/thu/fri/sat/sun), which don't collide with anything valid in `YYYY-MM-DD`.
- **Locale**: `chrono::Weekday` uses ISO weekdays (Mon=1…Sun=7). The mapping is hardcoded in English only — acceptable for this app's audience.

## Migration Plan

Pure additive change. No migration needed — existing `YYYY-MM-DD` input continues to work unchanged.
