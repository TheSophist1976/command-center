## 1. App State Changes

- [x] 1.1 Add `status_message: Option<String>` field to `App` struct, initialized to `None` in `App::new`
- [x] 1.2 Add `use crate::auth;` and `use crate::todoist;` imports to `tui.rs`

## 2. Import Key Handler

- [x] 2.1 Add `KeyCode::Char('i')` match arm in `handle_normal` that reads the token via `auth::read_token()`, sets status message to error if `None`, otherwise calls `todoist::run_import`
- [x] 2.2 On successful import, call `app.save()` and `app.clamp_selection()`, then set `status_message` to summary (e.g., "Imported N tasks, skipped M (already exported)")
- [x] 2.3 On import error, set `status_message` to the error string returned by `run_import`

## 3. Status Message Display

- [x] 3.1 Update `draw_footer` to check `app.status_message` in `Mode::Normal` — if `Some`, display the message instead of keybinding hints
- [x] 3.2 Clear `status_message` at the start of `handle_normal` (before processing the current keypress) so any key dismisses the message

## 4. Footer Hint Update

- [x] 4.1 Add `i:import` to the Normal mode keybinding hints string in `draw_footer`

## 5. Tests

- [x] 5.1 Add unit test: status message is cleared after key press (verify `status_message` resets to `None`)
- [x] 5.2 Add unit test: no-token path sets appropriate status message
