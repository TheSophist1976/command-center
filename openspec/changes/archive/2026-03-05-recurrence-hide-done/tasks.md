## 1. Filter Change

- [x] 1.1 Remove `View::Recurring` from the done-task exception in `View::matches` so done tasks are only allowed in `View::All`

## 2. Tests

- [x] 2.1 Add test verifying that a done recurring task does NOT appear in the Recurring view
- [x] 2.2 Add test verifying that an open recurring task still appears in the Recurring view
