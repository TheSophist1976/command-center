## 1. Recurrence Variant

- [x] 1.1 Add `WeeklyOn { weekday: Weekday, every_n_weeks: u32 }` variant to `Recurrence` enum in `src/task.rs`
- [x] 1.2 Update `Display` impl: `weekly:fri` for count=1, `weekly:2:mon` for count>1
- [x] 1.3 Update `FromStr` impl: parse `weekly:DAY` and `weekly:N:DAY`, disambiguate `weekly:X` (number vs weekday)
- [x] 1.4 Add unit tests for parsing and display of `WeeklyOn`

## 2. Next Due Date

- [x] 2.1 Add `WeeklyOn` arm to `next_due_date`: if base is target weekday, add 7*N days; otherwise find next occurrence of weekday
- [x] 2.2 Add unit tests for next due date calculation (same weekday base, different weekday base, biweekly)

## 3. TUI Display

- [x] 3.1 Update `format_recurrence_display` in `src/tui.rs` for `WeeklyOn`: "Weekly (Fri)" or "Every 2 Weeks (Mon)"
- [x] 3.2 Add unit tests for TUI display formatting

## 4. NLP Integration

- [x] 4.1 Update NLP system prompt in `src/nlp.rs` to recognize "every week on Friday", "every 2 weeks on Monday" and map to `weekly:fri`, `weekly:2:mon`
