## 1. Add Pattern Column

- [x] 1.1 Add "Pattern" header cell after the "↻" header cell when `show_recur` is true
- [x] 1.2 Add pattern cell in each row: use `format_recurrence_display(r)` for recurring tasks, empty string for non-recurring
- [x] 1.3 Add a `Constraint::Min(8)` width entry for the Pattern column after the "↻" width entry

## 2. Tests

- [x] 2.1 Add test verifying the recurrence pattern column appears when tasks have recurrence set
