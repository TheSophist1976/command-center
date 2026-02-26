## 1. View Enum and Filtering Logic

- [x] 1.1 Add `View` enum to `src/tui.rs` with variants: Today, All, Weekly, Monthly, Yearly, NoDueDate
- [x] 1.2 Implement `View::matches(&self, task: &Task, today: NaiveDate) -> bool` with filtering logic for each variant
- [x] 1.3 Implement `View::next()` and `View::prev()` methods for cycling through views
- [x] 1.4 Implement `View::display_name(&self) -> &str` returning human-readable names (Today, All Tasks, This Week, This Month, This Year, No Due Date)
- [x] 1.5 Implement `View::from_str()` to parse config values (today, all, weekly, monthly, yearly, no-due-date)

## 2. App Struct Integration

- [x] 2.1 Add `view: View` field to the `App` struct
- [x] 2.2 Read `default-view` from config in `App::new()` and initialize the view field, defaulting to `View::Today`
- [x] 2.3 Modify `filtered_indices()` to apply view filtering before user filter (two-stage chain)

## 3. Keybindings and Input Handling

- [x] 3.1 Add `v` keybinding in normal mode to cycle to the next view and reset selection to 0
- [x] 3.2 Add `V` (shift-v) keybinding in normal mode to cycle to the previous view and reset selection to 0

## 4. Header and Footer Updates

- [x] 4.1 Update `draw_header()` to display the active view name (e.g., "task-manager | Today" or "task-manager | Today | filter: status:open")
- [x] 4.2 Update `draw_footer()` to include `v:view` in the keybinding hints

## 5. Config Support

- [x] 5.1 Add `default-view` as a recognized config key with accepted values: today, all, weekly, monthly, yearly, no-due-date

## 6. Tests

- [x] 6.1 Add unit tests for `View::matches()` covering all 6 view variants with various due_date values
- [x] 6.2 Add unit tests for `View::next()`, `View::prev()` cycling including wrap-around
- [x] 6.3 Add unit tests for `View::from_str()` parsing valid values and falling back on invalid input
- [x] 6.4 Add integration test verifying the TUI launches with the default view
