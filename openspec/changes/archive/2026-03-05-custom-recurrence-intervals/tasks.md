## 1. Data Model (`src/task.rs`)

- [x] 1.1 Change `Recurrence::Interval(IntervalUnit)` to `Recurrence::Interval { unit: IntervalUnit, count: u32 }`
- [x] 1.2 Update `Display` impl: count == 1 outputs `"daily"` etc., count > 1 outputs `"daily:3"` etc.
- [x] 1.3 Update `FromStr` impl: parse `"daily"` as count 1, `"daily:3"` as count 3, reject count 0. Disambiguate from NthWeekday `"monthly:N:DAY"` format.
- [x] 1.4 Update `next_due_date`: multiply interval by count (days * count, weeks * count, add_months(count), add_months(count * 12))

## 2. Fix All Match Arms

- [x] 2.1 Update all `Recurrence::Interval(unit)` match arms across `src/task.rs`, `src/tui.rs`, and `src/nlp.rs` to use `Recurrence::Interval { unit, .. }` or `Recurrence::Interval { unit, count }`

## 3. TUI Display (`src/tui.rs`)

- [x] 3.1 Update `format_recurrence_display`: count == 1 unchanged, count > 1 shows "Every N Days/Weeks/Months/Years"

## 4. NLP Prompt (`src/nlp.rs`)

- [x] 4.1 Update `parse_recurrence_nlp` system prompt to include custom interval patterns (`"daily:N"`, `"weekly:N"`, `"monthly:N"`, `"yearly:N"`) and rules for "every 3 months", "quarterly", "biweekly", "every other day", etc.

## 5. Tests

- [x] 5.1 Update all existing tests that use `Recurrence::Interval(unit)` to use new struct syntax
- [x] 5.2 Add test: parse `"monthly:3"` → `Interval { unit: Monthly, count: 3 }`
- [x] 5.3 Add test: display `Interval { unit: Monthly, count: 3 }` → `"monthly:3"`
- [x] 5.4 Add test: `next_due_date` with `Interval { unit: Monthly, count: 3 }` from `2026-01-15` → `2026-04-15`
- [x] 5.5 Add test: `format_recurrence_display` with count > 1 shows "Every 3 Months"
- [x] 5.6 Add test: parse `"daily:0"` returns error
