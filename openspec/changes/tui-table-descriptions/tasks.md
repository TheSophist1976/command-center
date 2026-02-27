## 1. Description column in task table

- [x] 1.1 Add `show_desc` conditional check in `draw_table` (true when any filtered task has a non-None, non-empty description)
- [x] 1.2 Add "Desc" header cell after "Title" when `show_desc` is true, add truncated description cell (29 chars + "…" if over 30, full text otherwise), and add `Constraint::Length(30)` width entry

## 2. Detail panel toggle

- [x] 2.1 Add `show_detail_panel: bool` field to `App` struct (default false), handle `Tab` keypress in `handle_normal` to toggle it
- [x] 2.2 Update Normal mode layout in `draw()` to split into table (~70%) and detail panel (~30%) when `show_detail_panel` is true
- [x] 2.3 Implement `draw_detail_panel` function that renders a bordered block titled "Task Details" with all fields of the selected task: ID, Title, Status, Priority, Description (full, wrapped), Tags, Due Date, Project, Created, Updated. Show "No task selected." when no task is selected.

## 3. Footer hints

- [x] 3.1 Add `Tab:details` to the Normal mode footer hint string

## 4. Tests

- [x] 4.1 Add unit test that `Tab` toggles `show_detail_panel` on and off
- [x] 4.2 Add unit test for description truncation logic (over 30 chars gets "…", 30 or under stays full, None/empty shows empty string)
