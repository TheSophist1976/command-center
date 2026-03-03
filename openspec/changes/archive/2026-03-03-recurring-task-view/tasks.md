## Tasks

- [x] Add `Recurring` variant to the `View` enum in `src/tui.rs`. Add match arm in `matches()` that returns `task.recurrence.is_some()` (show both open and done). Skip the overdue/done early-return logic for `Recurring` (like `All` does). Add `display_name()` returning `"Recurring"`. Update `next()`/`prev()` cycle: NoDueDate → Recurring → Today.
- [x] Update existing TUI view tests in `src/tui.rs` to account for the new cycle order (NoDueDate → Recurring → Today instead of NoDueDate → Today). Add tests for the Recurring view: recurring open task shown, recurring done task shown, non-recurring task hidden.
