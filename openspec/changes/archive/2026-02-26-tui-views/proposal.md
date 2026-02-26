## Why

The TUI currently shows all tasks in a flat list with manual filtering. Users need quick access to time-based views of their tasks — what's due today, this week, this month, or this year — without typing filter expressions each time. A configurable default view lets users land on the most relevant slice of tasks immediately on launch.

## What Changes

- Add a `View` concept to the TUI with predefined views: Today (due today + no due date), All, Weekly, Monthly, Yearly, and No Due Date
- Add keybinding to cycle through or select views in normal mode
- Display the active view name in the TUI header
- Add a `default-view` config key so users can set which view loads on TUI launch
- The existing filter system continues to work and stacks on top of the active view

## Capabilities

### New Capabilities
- `tui-views`: Predefined time-based views (Today, All, Weekly, Monthly, Yearly, No Due Date) with keybinding to switch between them and header display of the active view
- `default-view-config`: A `default-view` config key that controls which view the TUI opens with, defaulting to "today" if unset

### Modified Capabilities
- `tui`: Add view awareness to the TUI layout (header shows active view, footer shows view-switch keybinding). Filters stack on top of the active view.
- `app-config`: Add the `default-view` key to the config system with accepted values: today, all, weekly, monthly, yearly, no-due-date

## Impact

- **Code**: `src/tui.rs` (new View enum, view filtering logic, keybinding, header/footer updates), `src/config.rs` (new config key validation)
- **Existing behavior**: The TUI currently shows all tasks by default. After this change, the default view becomes "today" (due today + no due date) unless configured otherwise. Users can switch to "all" to restore the previous behavior.
- **Dependencies**: Uses `chrono` (already a dependency) for date comparisons
