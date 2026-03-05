## Why

All TUI views currently display tasks in storage/insertion order, making it difficult to see what's most urgent. Tasks should be sorted by due date (earliest first) then by priority (highest first) so the most actionable items always appear at the top.

## What Changes

- All views will sort filtered tasks by due date ascending (earliest first), with `None` due dates sorted last
- Within the same due date, tasks will be sorted by priority descending (Critical > High > Medium > Low)
- This applies uniformly to every view (Today, All, Weekly, Monthly, Yearly, NoDueDate, Recurring)
- The `cli list` command output will also respect this sort order

## Capabilities

### New Capabilities

- `task-sort-order`: Defines the default sort order for task lists — due date ascending then priority descending, applied after view and filter predicates

### Modified Capabilities

- `tui-views`: Views will apply the sort order to filtered results before display

## Impact

- `src/tui.rs`: `filtered_indices()` method will sort results before returning
- `src/cli.rs`: Any list/display commands will apply the same sort
- Existing tests that assume insertion order may need updating
