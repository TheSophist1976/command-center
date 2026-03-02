## Why

Task descriptions are only visible by pressing `r` to edit them or through the CLI. Users have no way to see descriptions at a glance while browsing tasks in the TUI, and no way to view full task details without entering edit mode. This change adds both a truncated description column in the table and a toggleable detail panel for viewing all fields of the selected task.

## What Changes

- Add a conditional "Desc" column to the task table, shown when at least one visible task has a description (same pattern as the Due and Project columns)
- Descriptions are truncated to 30 characters in the table column
- Add a toggleable bottom detail panel that shows all fields of the currently selected task (id, title, status, priority, description, tags, due date, project, created, updated)
- A keybinding in Normal mode toggles the panel on/off
- When the panel is visible, it updates as the user navigates between tasks

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `tui`: Adding a conditional description column to the task table and a toggleable detail panel

## Impact

- `src/tui.rs` — Modify `draw_table` for description column, add detail panel rendering, add toggle state and keybinding
