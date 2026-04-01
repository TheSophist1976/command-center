## 1. Key Handler

- [x] 1.1 In `handle_key()` in `src/tui.rs`, add a `ctrl+r` branch in Normal mode: if `app.mode != Mode::Normal`, set status message `"Cannot reload: finish editing first"` and return; otherwise call `storage::load(&app.file_path, false)`, replace `app.task_file`, call `app.clamp_selection()`, and set status message `"Reloaded N tasks from disk"`

## 2. Footer Hint

- [x] 2.1 Update the Normal-mode footer hint string in `src/tui.rs` to include `^r:reload` alongside the existing hints

## 3. Tests

- [x] 3.1 Add a unit/integration test verifying that `ctrl+r` in Normal mode reloads a modified `tasks.md` and updates the task count
- [x] 3.2 Add a test verifying that `ctrl+r` in a non-Normal mode (e.g., `Adding`) does not reload and sets the correct warning status message
