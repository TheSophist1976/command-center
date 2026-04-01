## 1. Core Parsing Utility

- [x] 1.1 Add `parse_due_date_input(s: &str, today: NaiveDate) -> Option<NaiveDate>` to `src/parser.rs` — tries `%Y-%m-%d` first, then maps weekday names/abbreviations to the next future occurrence (never today)
- [x] 1.2 Write unit tests for `parse_due_date_input` covering: ISO passthrough, full weekday names, three-letter abbreviations, case-insensitivity, same-weekday-as-today → next week, unrecognized input → None, empty string → None

## 2. TUI Integration

- [x] 2.1 Replace the inline `NaiveDate::parse_from_str` at `tui.rs` ~2620 (detail edit panel validation) with `parse_due_date_input(draft.due_date.trim(), today)`; update the error message to `"Invalid date (use YYYY-MM-DD or a weekday name)"`
- [x] 2.2 Replace the inline `NaiveDate::parse_from_str` at `tui.rs` ~2641 (detail edit panel save apply) with `parse_due_date_input` so the resolved date is stored

## 3. Verification

- [x] 3.1 Run `cargo test` and confirm all tests pass
- [x] 3.2 Manual smoke test: open TUI, edit a task's due date, type a weekday name, save — verify the due date column shows the correct next occurrence
