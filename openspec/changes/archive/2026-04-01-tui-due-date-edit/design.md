## Context

The TUI already supports inline editing for title, tags, description, recurrence, and agent via a shared `handle_input` function and `InputAction` enum. Each editable field follows the same pattern:

1. A key in Normal mode sets `app.input_buffer` (pre-filled with current value) and switches `app.mode` to a dedicated `Mode::Editing*` variant
2. The event loop routes to `handle_input(app, key, InputAction::Edit*)` while in that mode
3. On `Enter`, the input is validated, the task is mutated, and the file is saved
4. On `Esc`, the mode reverts to Normal without saving

Due date editing fits this pattern exactly. The only additional concern is date-string validation.

## Goals / Non-Goals

**Goals:**
- Add `u` key in Normal mode to open an inline due date editor
- Pre-fill with the task's current due date (`YYYY-MM-DD`) or empty string if none
- Accept `YYYY-MM-DD` as a valid input — save the parsed date to the task
- Accept empty string — clears the due date
- Reject invalid input silently (stay in edit mode with a status message)
- Update footer hint line to include `u:due`

**Non-Goals:**
- A date picker or calendar widget — plain text input only
- Partial date input (e.g., `04-15` without year)
- Relative dates (e.g., `tomorrow`, `+3d`)

## Decisions

**Reuse `handle_input` + `InputAction` pattern**
Adding `InputAction::EditDue` and `Mode::EditingDue` keeps the implementation consistent with existing editing modes. No new handler function needed — `handle_input` already handles all keyboard logic; we only add a new `InputAction` variant with its confirm logic.

**Key choice: `u`**
All obvious single-letter keys near `d` (delete) are taken. `u` is unused in Normal mode and is mnemonic for "update due".

**Validation: reject and stay in edit mode on bad input**
Rather than silently accepting garbage or crashing, invalid date strings will leave the mode as `EditingDue` and set `app.status_message` to `"Invalid date — use YYYY-MM-DD"`. This is consistent with how recurrence handles invalid patterns.

**Empty input clears the date**
Consistent with how `EditDescription` treats empty input as `None`.

## Risks / Trade-offs

- [Risk] User types a valid-looking but semantically odd date (e.g., `1900-01-01`) → Accepted as-is; no range validation. Low risk, user intent is clear.
- [Risk] `u` conflicts with a future feature → Low risk; key assignment can be changed when needed.

## Migration Plan

No migration needed. Due dates are already stored in `tasks.md`; this only adds a new write path via the TUI.
