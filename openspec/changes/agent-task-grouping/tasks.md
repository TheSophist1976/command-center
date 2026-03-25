## 1. Add View::ByAgent to the View enum

- [x] 1.1 Add `ByAgent` variant to the `View` enum in `src/tui.rs`
- [x] 1.2 Insert `ByAgent` into the view cycle (between `Recurring` and `Notes`) in `View::next()` and `View::prev()`
- [x] 1.3 Add `"by-agent"` to `View::from_config` and `"By Agent"` to `View::display_name`
- [x] 1.4 Add `View::ByAgent` match arm to `View::matches` — returns true for all open tasks (Done excluded by existing guard)

## 2. Implement By Agent grouped rendering

- [x] 2.1 Add a `draw_agent_grouped_table` function
- [x] 2.2 Build the group structure: collect unique agent values, sort alphabetically, put "Unassigned" last
- [x] 2.3 Within each group, tasks appear in the order produced by `filtered_indices()` (already sorted by due/priority)
- [x] 2.4 Render each section header as a visually distinct row (bold, dimmed BAR_BG-like background)
- [x] 2.5 Render task rows with ID, status, priority, title, due, tags columns
- [x] 2.6 Call `draw_agent_grouped_table` from `draw` when `app.view == View::ByAgent`

## 3. Fix selection navigation for header rows

- [x] 3.1 Compute rendered row index from `app.selected` by mapping task index to render position (accounting for injected headers)
- [x] 3.2 Navigation (`j`/`k`) operates on `filtered_indices()` which contains no headers — only task rows are selectable
- [x] 3.3 Task-level actions (toggle, priority, etc.) continue to use `app.selected` + `filtered_indices()` unchanged

## 4. Update footer and header display

- [x] 4.1 `View::display_name()` returns "By Agent" — shown in TUI header via existing `draw_header` code
- [x] 4.2 Footer hints unchanged — `v:view` already covers ByAgent view cycling

## 5. Verify and clean up

- [x] 5.1 `cargo build` — clean
- [x] 5.2 `cargo test` — 192 passing, 1 pre-existing auth failure
- [x] 5.3 View cycle tests updated to include `ByAgent`
