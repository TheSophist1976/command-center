## 1. Dependencies and CLI Wiring

- [x] 1.1 Add `ratatui` and `crossterm` dependencies to `Cargo.toml`
- [x] 1.2 Add `Tui` variant to the `Command` enum in `src/cli.rs`
- [x] 1.3 Add `mod tui;` declaration and `Command::Tui` match arm in `src/main.rs` that calls `tui::run(path)`

## 2. App State and Core Types

- [x] 2.1 Create `src/tui.rs` with `Mode` enum (`Normal`, `Adding`, `Filtering`, `Confirming`), `Filter` struct, and `App` struct holding `TaskFile`, `file_path`, `selected`, `filter`, `mode`, and `input_buffer`
- [x] 2.2 Implement `App::new(path)` that loads the task file via `storage::load` and initializes default state
- [x] 2.3 Implement `App::filtered_tasks()` that returns task indices matching the current filter

## 3. Terminal Setup and Event Loop

- [x] 3.1 Implement `tui::run(path)` entry point that sets up alternate screen, raw mode, and installs a panic hook for terminal cleanup
- [x] 3.2 Implement the main event loop using `crossterm::event::poll` with 250ms timeout, dispatching key events and handling terminal resize

## 4. Rendering

- [x] 4.1 Implement the three-region layout: header bar (title + active filter), task table (scrollable with highlighted selection), and footer bar (context-sensitive keybinding hints)
- [x] 4.2 Implement task table rendering with columns for ID, status checkbox, priority, title, and tags — with "No tasks" empty-state message
- [x] 4.3 Implement footer rendering that changes based on current mode (Normal shows keybindings, Adding/Filtering shows input prompt, Confirming shows y/n prompt)

## 5. Navigation and Actions

- [x] 5.1 Implement Normal mode key handling: `j`/`Down` and `k`/`Up` for cursor movement with clamping at boundaries
- [x] 5.2 Implement toggle completion (`Enter`/`Space`) that flips task status and saves via `storage::save`
- [x] 5.3 Implement add mode: `a` enters Adding mode, text input builds in `input_buffer`, `Enter` creates task with default priority and saves, `Esc` cancels
- [x] 5.4 Implement delete with confirmation: `d` enters Confirming mode, `y` removes task and saves, any other key cancels
- [x] 5.5 Implement filter mode: `f`/`/` enters Filtering mode, parses `status:`, `priority:`, `tag:` expressions, `Esc` in Normal mode clears filter
- [x] 5.6 Implement `q` to quit and restore terminal state

## 6. Testing

- [x] 6.1 Add integration tests for `task tui --help` to verify the subcommand is registered
- [x] 6.2 Manually verify TUI launches, displays tasks, and all keybindings work correctly
