## 1. GroupBy enum — add DueDate and extend cycle

- [x] 1.1 Add `DueDate` variant to the `GroupBy` enum
- [x] 1.2 Add `"due-date"` to `to_config_str()` and `from_config()` for `GroupBy`
- [x] 1.3 Extend `next()` cycle: `Priority → DueDate → None`
- [x] 1.4 Add `View` to derive list: `#[derive(Hash, Eq)]` (needed for HashMap key)

## 2. App state — per-view/window grouping map

- [x] 2.1 Replace `group_by: GroupBy` field in `App` with `view_groupings: HashMap<String, GroupBy>` keyed by slot strings (e.g. `"due.day"`, `"no-due-date"`)
- [x] 2.2 Add `fn grouping_slot(&self) -> String` — returns `"due.<window>"` for Due view, `"<view>"` for others
- [x] 2.3 Add helper `fn active_group_by(&self) -> GroupBy` using `grouping_slot()`
- [x] 2.4 On `App::new`, load all 5 Due windows + 3 non-Due views from `group-by.<slot>` config keys
- [x] 2.5 Add `DueWindow::to_config_str()` returning `"day"`, `"week"`, `"month"`, `"year"`, `"all"`

## 3. G key — save per-slot grouping

- [x] 3.1 When `G` is pressed, update `view_groupings[slot]` and write `group-by.<slot>` to config

## 4. View + window switching — restore per-slot grouping

- [x] 4.1 Switching view or Due window restores that slot's saved grouping automatically via `active_group_by()`

## 5. Draw table — DueDate grouping rendering

- [x] 5.1 Key extraction for `DueDate`: `task.due_date.map(|d| format_due_group_label(d))`
- [x] 5.2 `format_due_group_label(date)` produces "Monday Feb 12th" format (full weekday, abbreviated month, ordinal day)
- [x] 5.3 `ordinal_day(u32)` helper for "st"/"nd"/"rd"/"th" suffix
- [x] 5.4 DueDate groups sort chronologically (by underlying date, not label string)
- [x] 5.5 "No Due Date" label for tasks with no due date (replaces generic "(none)")
- [x] 5.6 Same formatting and sort in both `visual_task_order()` and `draw_table()`

## 6. Tests

- [x] 6.1 Unit test: `GroupBy::Priority.next()` == `DueDate`, `GroupBy::DueDate.next()` == `None`
- [x] 6.2 Unit test: pressing `G` in Due/Day window saves `group-by.due.day` config key
- [x] 6.3 Unit test: switching view restores that view's saved grouping
- [x] 6.4 Unit test: `DueDate` grouping key extraction returns friendly format string
