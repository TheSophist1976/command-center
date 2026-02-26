## ADDED Requirements

### Requirement: TUI entry point
The system SHALL provide a `task tui` subcommand that launches a full-screen terminal interface. The TUI SHALL take ownership of the terminal using crossterm's alternate screen and raw mode. On exit, the terminal SHALL be restored to its original state.

#### Scenario: Launch TUI
- **WHEN** the user runs `task tui`
- **THEN** the system SHALL enter alternate screen, enable raw mode, and display the TUI dashboard

#### Scenario: Quit TUI
- **WHEN** the user presses `q` in normal mode
- **THEN** the system SHALL restore the terminal to its original state and exit with code 0

#### Scenario: Panic recovery
- **WHEN** the TUI encounters a panic during execution
- **THEN** the system SHALL restore raw mode and alternate screen before printing the panic message

### Requirement: Three-region layout
The TUI SHALL render a three-region layout: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints.

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints like "j/k:nav  Enter:toggle  a:add  d:delete  f:filter  q:quit"

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

### Requirement: Task table display
The task table SHALL display columns for ID, status (checkbox), priority, title, and tags — matching the information shown by `task list`. The currently selected row SHALL be visually highlighted.

#### Scenario: Table with tasks
- **WHEN** the TUI loads a file containing tasks
- **THEN** the table SHALL display each task as a row with ID, a checkbox (checked for done), priority, title, and tags columns

#### Scenario: Empty task file
- **WHEN** the TUI loads a file with no tasks
- **THEN** the table area SHALL display a message like "No tasks. Press 'a' to add one."

### Requirement: Keyboard navigation
The user SHALL navigate the task list using `j` or `Down` to move the cursor down and `k` or `Up` to move the cursor up. The cursor SHALL wrap or clamp at list boundaries.

#### Scenario: Move cursor down
- **WHEN** the user presses `j` or `Down` with tasks below the cursor
- **THEN** the selected row SHALL move down by one

#### Scenario: Move cursor up
- **WHEN** the user presses `k` or `Up` with tasks above the cursor
- **THEN** the selected row SHALL move up by one

#### Scenario: Cursor at bottom of list
- **WHEN** the user presses `j` or `Down` on the last task
- **THEN** the cursor SHALL remain on the last task (clamp behavior)

#### Scenario: Cursor at top of list
- **WHEN** the user presses `k` or `Up` on the first task
- **THEN** the cursor SHALL remain on the first task (clamp behavior)

### Requirement: Toggle task completion
The user SHALL toggle a task between open and done by pressing `Enter` or `Space` on the selected task. The change SHALL be persisted to disk immediately.

#### Scenario: Mark task as done
- **WHEN** the user presses `Enter` on an open task
- **THEN** the task's status SHALL change to done, the display SHALL update, and the file SHALL be saved

#### Scenario: Reopen a done task
- **WHEN** the user presses `Enter` on a done task
- **THEN** the task's status SHALL change to open, the display SHALL update, and the file SHALL be saved

### Requirement: Add task
The user SHALL press `a` to enter add mode. The footer SHALL display a text input prompt. The user types a task title and presses `Enter` to confirm or `Esc` to cancel. The new task SHALL be added with default priority (medium) and no tags.

#### Scenario: Add a new task
- **WHEN** the user presses `a`, types "Buy groceries", and presses `Enter`
- **THEN** a new task with title "Buy groceries", status open, and priority medium SHALL be added and saved to disk

#### Scenario: Cancel adding a task
- **WHEN** the user presses `a` and then presses `Esc`
- **THEN** no task SHALL be added and the TUI SHALL return to normal mode

### Requirement: Delete task with confirmation
The user SHALL press `d` to initiate deletion of the selected task. The footer SHALL display a confirmation prompt ("Delete task N? y/n"). Pressing `y` confirms deletion; any other key cancels.

#### Scenario: Confirm delete
- **WHEN** the user presses `d` on task 3 and then presses `y`
- **THEN** task 3 SHALL be removed from the file and the display SHALL update

#### Scenario: Cancel delete
- **WHEN** the user presses `d` on task 3 and then presses `n` or `Esc`
- **THEN** task 3 SHALL remain and the TUI SHALL return to normal mode

### Requirement: Filter tasks
The user SHALL press `f` or `/` to enter filter mode. The footer SHALL display a text input for a filter expression. Supported filters: `status:open`, `status:done`, `priority:high`, `priority:medium`, `priority:low`, `tag:<name>`. Pressing `Esc` in normal mode SHALL clear any active filter and show all tasks.

#### Scenario: Filter by status
- **WHEN** the user presses `f`, types "status:open", and presses `Enter`
- **THEN** the task table SHALL show only tasks with open status

#### Scenario: Filter by tag
- **WHEN** the user presses `f`, types "tag:frontend", and presses `Enter`
- **THEN** the task table SHALL show only tasks tagged with "frontend"

#### Scenario: Clear filter
- **WHEN** a filter is active and the user presses `Esc`
- **THEN** the filter SHALL be cleared and all tasks SHALL be displayed

#### Scenario: No matching tasks
- **WHEN** the user applies a filter that matches no tasks
- **THEN** the table area SHALL display "No tasks match filter."

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, and Confirming. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys.

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard the input buffer

### Requirement: Shared storage layer
The TUI SHALL use the existing `storage::load` and `storage::save` functions for all file I/O. The TUI SHALL NOT implement its own file reading or writing logic.

#### Scenario: Load tasks at startup
- **WHEN** the TUI launches with `--file custom.md`
- **THEN** the TUI SHALL call `storage::load("custom.md")` to load tasks

#### Scenario: Save after mutation
- **WHEN** the user toggles, adds, or deletes a task in the TUI
- **THEN** the TUI SHALL call `storage::save` with the updated task file data
