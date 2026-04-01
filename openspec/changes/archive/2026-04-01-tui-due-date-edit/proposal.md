## Why

The TUI supports inline editing for title, priority, tags, description, recurrence, and agent — but has no way to edit a task's due date without leaving the TUI and using the CLI. This creates friction for the most common task update: adjusting when something is due.

## What Changes

- Add `EditingDue` mode to the TUI
- Bind `d` key in Normal mode to open an inline due-date editor for the selected task
- Pre-fill the editor with the existing due date (`YYYY-MM-DD`) or empty string if none
- Validate input on confirm: accept `YYYY-MM-DD` or empty (clears the date)
- Save the updated due date to `tasks.md` and refresh the view
- **Remove** the delete task command (`d` key + `Mode::Confirming` flow) — tasks can be deleted via the CLI (`task delete <id>`)
- Update footer keybinding hints to include `d:due` and remove `d:delete`

## Capabilities

### New Capabilities
- `tui-due-date-edit`: Inline due date editing in the TUI via the `d` key

### Modified Capabilities
- `tui`: Footer hint line and Normal mode key table: `d` rebound from delete to due-date edit; delete command removed

## Impact

- `src/tui.rs`: New `Mode::EditingDue` variant; `d` key handler changed from delete to due-date edit; `Mode::Confirming` delete flow removed; footer hint updated
- `src/task.rs`: No changes needed
- `openspec/specs/tui/spec.md`: Delta spec for key rebinding and mode list
- `openspec/specs/tui-due-date-edit/spec.md`: New spec for this capability
