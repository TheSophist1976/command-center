## Context

The TUI currently shows all tasks in a flat list, filterable via manual filter expressions (`f` key). Users must type filter expressions each time to narrow by due date. The `App` struct holds a `Filter` and uses `filtered_indices()` to determine which tasks to display. The config system supports simple `key: value` pairs in a markdown file.

## Goals / Non-Goals

**Goals:**
- Add a `View` enum that pre-filters tasks by due-date ranges before user filters apply
- Let users cycle views with `v`/`V` keybindings
- Show the active view name in the header bar
- Support a `default-view` config key to control the startup view
- Keep the existing filter system working unchanged on top of views

**Non-Goals:**
- Custom/user-defined views (only the 6 predefined views)
- Persisting the last-used view across sessions (config sets the startup default, not a session memory)
- Calendar or date-picker UI
- Changing how overdue tasks are displayed (no special styling)

## Decisions

### 1. View enum in `tui.rs`

Add a `View` enum alongside the existing `Mode` and `Filter` types:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Today,
    All,
    Weekly,
    Monthly,
    Yearly,
    NoDueDate,
}
```

Each variant implements a `matches(&self, task: &Task, today: NaiveDate) -> bool` method (or a standalone function). This keeps view logic self-contained.

**Rationale**: An enum with a match method mirrors the existing `Filter::matches` pattern. Adding it as a method on `View` keeps the filtering pipeline clean: view filter → user filter → display.

**Alternative considered**: Reusing the `Filter` struct with a new date-range field. Rejected because views are conceptual presets, not user-composed expressions — mixing them into `Filter` would complicate the filter summary display and `Esc`-to-clear behavior.

### 2. View field on `App` struct

Add `view: View` to the `App` struct. Initialize it from config at startup:

```rust
struct App {
    // ... existing fields ...
    view: View,
}
```

In `App::new()`, read `default-view` from config and parse it into a `View`. Fall back to `View::Today` if unset or invalid.

### 3. Two-stage filtering in `filtered_indices()`

Modify `filtered_indices()` to apply view filtering first, then user filter:

```rust
fn filtered_indices(&self) -> Vec<usize> {
    let today = chrono::Local::now().date_naive();
    self.task_file.tasks.iter().enumerate()
        .filter(|(_, t)| self.view.matches(t, today))
        .filter(|(_, t)| self.filter.matches(t))
        .map(|(i, _)| i)
        .collect()
}
```

**Rationale**: Chaining two `.filter()` calls is simple and maintains the existing code structure. The `today` date is computed once per call to `filtered_indices()` (called on each frame render), which is fine since it changes at most once per day.

### 4. View cycling with `v` / `V`

Add `View::next()` and `View::prev()` methods that return the next/previous variant in the cycle. Handle in the normal-mode key handler:

```rust
KeyCode::Char('v') => app.view = app.view.next(),
KeyCode::Char('V') => app.view = app.view.prev(),
```

Reset `app.selected` to 0 when switching views to avoid index-out-of-bounds on a shorter filtered list.

### 5. Header display

Modify `draw_header()` to always show the view name. Format:

- No filter: `" task-manager  |  Today "`
- With filter: `" task-manager  |  Today  |  filter: status:open "`

Add a `View::display_name(&self) -> &str` method returning human-readable names.

### 6. `default-view` config key

Add `default-view` to the config system. Valid values: `today`, `all`, `weekly`, `monthly`, `yearly`, `no-due-date`. The value is a plain string parsed via `View::from_str()`.

Read at TUI startup in `App::new()`. No validation at `config set` time — invalid values fall back to `Today` at TUI launch (consistent with how `default-dir` doesn't validate the path exists).

### 7. Date calculations using `chrono`

- **Today**: `chrono::Local::now().date_naive()`
- **Weekly**: Use `chrono::Datelike::weekday()` to find Monday of current week, add 6 days for Sunday. Compare `due_date` is within `[monday, sunday]`.
- **Monthly**: Compare `due_date.year() == today.year() && due_date.month() == today.month()`
- **Yearly**: Compare `due_date.year() == today.year()`

All using `NaiveDate` which matches the existing `Task.due_date` type. No timezone complexity.

## Risks / Trade-offs

- **[Default view change]** → Users who upgrade will see the Today view instead of all tasks on first launch. Mitigation: This is the desired UX improvement. Users can set `default-view: all` to restore old behavior.
- **[View resets selection]** → Switching views resets cursor to index 0. Mitigation: Acceptable since the task list changes entirely. Preserving position across different filtered sets would be confusing.
- **[No overdue handling]** → The Today view excludes overdue tasks (past due dates). Users must use All or a filter to find them. Mitigation: This matches the spec as written. Overdue handling can be added as a separate change if needed.
