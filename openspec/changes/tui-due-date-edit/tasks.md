## 1. Remove Delete Command

- [x] 1.1 Remove the `d` key handler (`Mode::Confirming`) from `handle_normal` in `src/tui.rs`
- [x] 1.2 Remove the `Mode::Confirming` arm from the main event dispatch
- [x] 1.3 Remove the `handle_confirming` function (or equivalent delete confirmation logic)
- [x] 1.4 Remove `Mode::Confirming` variant from the `Mode` enum

## 2. Add Mode and Input Action

- [x] 2.1 Add `EditingDue` variant to the `Mode` enum in `src/tui.rs`
- [x] 2.2 Add `EditDue` variant to the `InputAction` enum in `src/tui.rs`

## 3. Normal Mode Key Binding

- [x] 3.1 Add `d` key handler in `handle_normal`: set `input_buffer` to current due date string (or empty), switch to `Mode::EditingDue`
- [x] 3.2 Guard the `d` key so it does nothing when the task list is empty

## 4. Edit Confirmation Logic

- [x] 4.1 Route `Mode::EditingDue` in the main event dispatch to `handle_input(app, key, InputAction::EditDue)`
- [x] 4.2 Implement `InputAction::EditDue` in `handle_input`: parse empty input as `None` (clear date), parse valid `YYYY-MM-DD` via `chrono::NaiveDate::parse_from_str`, save task and file on success
- [x] 4.3 On invalid date string: set `app.status_message` to `"Invalid date — use YYYY-MM-DD"` and keep mode as `EditingDue`

## 5. Footer Hint Update

- [x] 5.1 Replace `d:delete` with `d:due` in the Normal mode footer hint string
- [x] 5.2 Remove any confirming-mode footer hint references

## 6. Tests

- [x] 6.1 Unit test: pressing `d` on a task with a due date pre-fills input buffer with `YYYY-MM-DD` and enters `EditingDue` mode
- [x] 6.2 Unit test: confirming a valid date string updates the task's `due_date` and saves
- [x] 6.3 Unit test: confirming an empty string clears the task's `due_date`
- [x] 6.4 Unit test: confirming an invalid string keeps mode as `EditingDue` and sets status message
- [x] 6.5 Unit test: pressing `Esc` in `EditingDue` returns to Normal without modifying the task
- [x] 6.6 Update or remove any existing tests that relied on the delete (`d`) flow
