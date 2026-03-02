## 1. View matching logic

- [x] 1.1 In `View::matches()`, after the completed-task guard, add: if the task is open, has a `due_date` that is `Some(d)` where `d < today`, and the view is not `NoDueDate`, return `true`.

## 2. Tests

- [x] 2.1 Update `today_view_hides_overdue_task` test to assert overdue open tasks ARE shown (rename to `today_view_shows_overdue_open_task`).
- [x] 2.2 Add test `today_view_hides_overdue_completed_task`: overdue task with `Status::Done` is hidden from Today view.
- [x] 2.3 Add test `weekly_view_shows_overdue_open_task`: open task with due date before this week is shown.
- [x] 2.4 Add test `monthly_view_shows_overdue_open_task`: open task with due date before this month is shown.
- [x] 2.5 Add test `yearly_view_shows_overdue_open_task`: open task with due date before this year is shown.
- [x] 2.6 Add test `no_due_date_view_hides_overdue_task`: overdue task is NOT shown in NoDueDate view (it has a date).
