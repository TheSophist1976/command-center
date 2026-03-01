## 1. Data structures and state

- [x] 1.1 Add `DetailDraft` struct with fields: `title: String`, `description: String`, `priority: Priority`, `status: Status`, `due_date: String`, `project: String`, `tags: String`, `original_task_id: u32`. Add `DetailDraft::from_task(&Task) -> Self` constructor and `DetailDraft::is_dirty(&self, task: &Task) -> bool` method.
- [x] 1.2 Add `NavDirection` enum (`Up`, `Down`). Add fields to `App`: `detail_draft: Option<DetailDraft>`, `detail_field_index: usize`, `pending_navigation: Option<NavDirection>`.
- [x] 1.3 Add `EditingDetailPanel` and `ConfirmingDetailSave` variants to the `Mode` enum.

## 2. Enter/exit editing mode

- [x] 2.1 In `handle_normal`, when `show_detail_panel` is true and `Enter` is pressed, create a `DetailDraft` from the selected task, set `detail_field_index` to 0, load the title into `input_buffer`, and enter `EditingDetailPanel` mode. Keep `Space` as the completion toggle regardless of panel state.
- [x] 2.2 In `handle_key` dispatch, add `Mode::EditingDetailPanel` branch that calls a new `handle_detail_edit(app, key)` function, and `Mode::ConfirmingDetailSave` branch that calls `handle_detail_confirm(app, key)`.

## 3. Field navigation and editing

- [x] 3.1 Implement `handle_detail_edit`: on `j`/`Down`/`Tab`, commit current `input_buffer` to draft, advance `detail_field_index` (wrap at 7), load next field value into `input_buffer`. On `k`/`Up`/`Shift-Tab`, same but decrement. Skip buffer load/commit for Priority and Status fields.
- [x] 3.2 In `handle_detail_edit`, for Priority field: handle `c`/`h`/`m`/`l` keys to set draft priority. For Status field: handle `Enter`/`Space` to toggle draft status. For text fields: delegate to existing character/backspace input handling on `input_buffer`.
- [x] 3.3 In `handle_detail_edit`, on `Esc`: commit current buffer to draft, check `is_dirty`. If clean, clear `detail_draft` and return to Normal. If dirty, enter `ConfirmingDetailSave` mode.

## 4. Save/discard/cancel confirmation

- [x] 4.1 Implement `handle_detail_confirm`: on `s`, apply draft fields to the task (parse `due_date` string as `NaiveDate`, empty clears to None; empty description clears to None; split tags on whitespace), set `updated` timestamp, save to disk, clear `detail_draft`, return to Normal. If `pending_navigation` is set, apply it and clear.
- [x] 4.2 In `handle_detail_confirm`: on `d`, clear `detail_draft`, return to Normal, apply and clear `pending_navigation` if set. On `c` or `Esc`, return to `EditingDetailPanel` mode.
- [x] 4.3 In `handle_detail_confirm` save path: if due date string is non-empty and fails to parse as `NaiveDate` (YYYY-MM-DD), set a status message "Invalid date format (use YYYY-MM-DD)", return to `EditingDetailPanel` with `detail_field_index` set to the Due Date field (index 4).

## 5. Navigation interception

- [x] 5.1 In `handle_normal`, when `j`/`k` is pressed and `detail_draft` is `Some` and dirty: store `NavDirection` in `pending_navigation`, enter `ConfirmingDetailSave` mode instead of moving selection. If draft is clean or `detail_draft` is None, navigate normally.

## 6. Panel rendering

- [x] 6.1 Update `draw_detail_panel` to check if `detail_draft` is `Some`. If so, render in edit layout: one line per field with label, draft value, and highlight on the focused field (`detail_field_index`). Show cursor indicator on text fields. If `detail_draft` is None, render read-only as before.
- [x] 6.2 For the focused text field in edit mode, display `input_buffer` with a `_` cursor suffix instead of the draft value.

## 7. Footer hints

- [x] 7.1 Update Normal mode footer: when `show_detail_panel` is true, show `Enter:edit` instead of `Enter:toggle` in the hint string.
- [x] 7.2 Add footer for `EditingDetailPanel` mode: `j/k:field  c/h/m/l:priority  Enter/Space:status  Esc:done`.
- [x] 7.3 Add footer for `ConfirmingDetailSave` mode: `Unsaved changes. [s]ave  [d]iscard  [c]ancel`.

## 8. Tests

- [x] 8.1 Test `DetailDraft::from_task` creates correct draft and `is_dirty` returns false for unchanged, true for changed.
- [x] 8.2 Test `Enter` with panel visible enters `EditingDetailPanel`, sets draft and field index. Test `Space` still toggles completion.
- [x] 8.3 Test field navigation wraps correctly (0→6→0 forward, 0→6 backward).
- [x] 8.4 Test `Esc` from clean draft exits immediately; `Esc` from dirty draft enters `ConfirmingDetailSave`.
- [x] 8.5 Test `s`/`d`/`c` in `ConfirmingDetailSave` mode (save applies changes, discard reverts, cancel returns to editing).
- [x] 8.6 Test navigation interception: `j`/`k` with dirty draft enters confirmation, clean draft navigates normally.
