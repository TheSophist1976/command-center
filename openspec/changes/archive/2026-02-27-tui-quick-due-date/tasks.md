## 1. Due date keybindings in Normal mode

- [x] 1.1 Add `T`, `W`, `M`, `Q`, `X` key handlers in `handle_normal` that set `due_date` on the selected task using chrono date arithmetic (`Local::now().date_naive()`, `checked_add_signed(Duration::days(7))`, `checked_add_months(Months::new(1))`, `checked_add_months(Months::new(3))`, and `None`), set `updated` timestamp, save, and set a status message ("Due: YYYY-MM-DD" or "Due date cleared"). No-op when no task is selected.

## 2. Footer hints

- [x] 2.1 Update the Normal mode footer hint string to include `T/W/M/Q:due  X:clr-due`

## 3. Tests

- [x] 3.1 Add unit tests for each due date keybinding (`T`, `W`, `M`, `Q`, `X`) verifying the task's `due_date` is set correctly and the file is saved
- [x] 3.2 Add a test that due date keys are no-ops when the task list is empty
