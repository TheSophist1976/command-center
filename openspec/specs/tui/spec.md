## ADDED Requirements

### Requirement: TUI entry point
The system SHALL provide a `task tui` subcommand that launches a full-screen terminal interface. The TUI SHALL take ownership of the terminal using crossterm's alternate screen and raw mode. On exit, the terminal SHALL be restored to its original state. When no `--file` flag or `TASK_FILE` env var is given, the TUI SHALL load tasks from the path resolved by `storage::resolve_file_path`, which includes the `default-dir` app config value.

#### Scenario: Launch TUI
- **WHEN** the user runs `task tui`
- **THEN** the system SHALL enter alternate screen, enable raw mode, and display the TUI dashboard

#### Scenario: Quit TUI
- **WHEN** the user presses `q` in normal mode
- **THEN** the system SHALL restore the terminal to its original state and exit with code 0

#### Scenario: Panic recovery
- **WHEN** the TUI encounters a panic during execution
- **THEN** the system SHALL restore raw mode and alternate screen before printing the panic message

#### Scenario: Launch with configured default directory

- **WHEN** `default-dir` is set in the app config and the user runs `task tui` with no `--file` flag
- **THEN** the TUI SHALL load tasks from `<default-dir>/tasks.md`

### Requirement: Three-region layout
The TUI SHALL render a three-region layout: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints.

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  D:set-dir  q:quit`

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

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
The user SHALL press `f` or `/` to enter filter mode. The footer SHALL display a text input for a filter expression. Supported filters: `status:open`, `status:done`, `priority:high`, `priority:medium`, `priority:low`, `priority:critical`, `tag:<name>`, `project:<name>`. Pressing `Esc` in normal mode SHALL clear any active filter and show all tasks.

#### Scenario: Filter by status
- **WHEN** the user presses `f`, types "status:open", and presses `Enter`
- **THEN** the task table SHALL show only tasks with open status

#### Scenario: Filter by tag
- **WHEN** the user presses `f`, types "tag:frontend", and presses `Enter`
- **THEN** the task table SHALL show only tasks tagged with "frontend"

#### Scenario: Filter by critical priority
- **WHEN** the user presses `f`, types "priority:critical", and presses `Enter`
- **THEN** the task table SHALL show only tasks with priority critical

#### Scenario: Filter by project
- **WHEN** the user presses `f`, types "project:Work", and presses `Enter`
- **THEN** the task table SHALL show only tasks with project "Work" (case-insensitive)

#### Scenario: Clear filter
- **WHEN** a filter is active and the user presses `Esc`
- **THEN** the filter SHALL be cleared and all tasks SHALL be displayed

#### Scenario: No matching tasks
- **WHEN** the user applies a filter that matches no tasks
- **THEN** the table area SHALL display "No tasks match filter."

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, and EditingDefaultDir. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. The `i` key in Normal mode SHALL trigger a Todoist import (handled outside the mode system via the status message pattern).

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import as specified in the tui-todoist-import capability

### Requirement: Edit task priority
The user SHALL press `p` in normal mode to enter priority-editing mode for the selected task. The footer SHALL display a picker prompt showing all four available priorities. The user SHALL press `c`, `h`, `m`, or `l` to set the priority to critical, high, medium, or low respectively. The change SHALL be persisted to disk immediately. Pressing `Esc` or any other key SHALL cancel without changing the task. The `p` key SHALL be a no-op when no task is selected.

#### Scenario: Set priority to critical
- **WHEN** the user presses `p` on a selected task and then presses `c`
- **THEN** the task's priority SHALL be set to critical, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to high
- **WHEN** the user presses `p` on a selected task and then presses `h`
- **THEN** the task's priority SHALL be set to high, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to medium
- **WHEN** the user presses `p` on a selected task and then presses `m`
- **THEN** the task's priority SHALL be set to medium, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to low
- **WHEN** the user presses `p` on a selected task and then presses `l`
- **THEN** the task's priority SHALL be set to low, the display SHALL update, and the file SHALL be saved

#### Scenario: Cancel priority edit
- **WHEN** the user presses `p` and then presses `Esc` or any key other than `c`, `h`, `m`, or `l`
- **THEN** the task's priority SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `p` and no task is selected (empty or fully filtered list)
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Edit task title
The user SHALL press `e` in normal mode to enter title-editing mode for the selected task. The footer SHALL display a text input pre-populated with the task's current title. The user SHALL edit the text and press `Enter` to confirm or `Esc` to cancel. A confirmed empty title SHALL be rejected and the TUI SHALL remain in editing mode. The change SHALL be persisted to disk immediately on confirmation. The `e` key SHALL be a no-op when no task is selected.

#### Scenario: Edit title to new value
- **WHEN** the user presses `e`, modifies the pre-populated title text, and presses `Enter`
- **THEN** the task's title SHALL be updated to the new value, the display SHALL update, and the file SHALL be saved

#### Scenario: Cancel title edit
- **WHEN** the user presses `e` and then presses `Esc`
- **THEN** the task's title SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: Reject empty title
- **WHEN** the user presses `e`, clears the input buffer, and presses `Enter`
- **THEN** the title SHALL NOT be updated and the TUI SHALL remain in title-editing mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `e` and no task is selected
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Edit task tags
The user SHALL press `t` in normal mode to enter tag-editing mode for the selected task. The footer SHALL display a text input pre-populated with the task's current tags as a space-separated string. The user SHALL edit the text and press `Enter` to confirm or `Esc` to cancel. An empty confirmed input SHALL clear all tags. The change SHALL be persisted to disk immediately on confirmation. The `t` key SHALL be a no-op when no task is selected.

#### Scenario: Edit tags to new values
- **WHEN** the user presses `t`, modifies the pre-populated tag string, and presses `Enter`
- **THEN** the task's tags SHALL be updated to the whitespace-split tokens of the new input, the display SHALL update, and the file SHALL be saved

#### Scenario: Clear all tags
- **WHEN** the user presses `t`, clears the input buffer, and presses `Enter`
- **THEN** the task's tags SHALL be set to an empty list and the file SHALL be saved

#### Scenario: Cancel tag edit
- **WHEN** the user presses `t` and then presses `Esc`
- **THEN** the task's tags SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `t` and no task is selected
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Edit task description
The user SHALL press `r` in normal mode to enter description-editing mode for the selected task. The footer SHALL display a text input pre-populated with the task's current description (empty string if none). The user SHALL edit the text and press `Enter` to confirm or `Esc` to cancel. A confirmed non-empty value SHALL set the description; a confirmed empty value SHALL clear it (set to none). The change SHALL be persisted to disk immediately on confirmation. The `r` key SHALL be a no-op when no task is selected.

#### Scenario: Set description
- **WHEN** the user presses `r`, types a description, and presses `Enter`
- **THEN** the task's description SHALL be set to the entered text, and the file SHALL be saved

#### Scenario: Clear description
- **WHEN** the user presses `r`, clears the input buffer, and presses `Enter`
- **THEN** the task's description SHALL be set to none and the file SHALL be saved

#### Scenario: Cancel description edit
- **WHEN** the user presses `r` and then presses `Esc`
- **THEN** the task's description SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `r` and no task is selected
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Shared storage layer
The TUI SHALL use the existing `storage::load` and `storage::save` functions for all file I/O. The TUI SHALL NOT implement its own file reading or writing logic.

#### Scenario: Load tasks at startup
- **WHEN** the TUI launches with `--file custom.md`
- **THEN** the TUI SHALL call `storage::load("custom.md")` to load tasks

#### Scenario: Save after mutation
- **WHEN** the user toggles, adds, or deletes a task in the TUI
- **THEN** the TUI SHALL call `storage::save` with the updated task file data

### Requirement: Set default directory from TUI

The user SHALL press `D` in normal mode to enter directory-setting mode. The footer SHALL display a text input prompt pre-populated with the current default directory (empty if not set). The user SHALL type a directory path and press `Enter` to confirm or `Esc` to cancel. On confirm, the system SHALL write the new value to the app config file and reload tasks from `<new-dir>/tasks.md`. Any unsaved in-memory state SHALL be saved before reloading. The `D` key SHALL be a no-op when the TUI is in any non-Normal mode.

#### Scenario: Set default directory

- **WHEN** the user presses `D`, types `/home/user/notes`, and presses `Enter`
- **THEN** the system SHALL save the current task state, write `default-dir: /home/user/notes` to the config file, and reload tasks from `/home/user/notes/tasks.md`

#### Scenario: Cancel setting default directory

- **WHEN** the user presses `D` and then presses `Esc`
- **THEN** the config SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: Footer hint updated

- **WHEN** the TUI is in normal mode
- **THEN** the footer SHALL include `D:set-dir` in its keybinding hints

### Requirement: Display due date and project in task table
The TUI task table SHALL display `due_date` and `project` columns when at least one visible task has those fields set. Tasks with `None` values SHALL display empty cells in those columns.

#### Scenario: Table shows due date column
- **WHEN** at least one task in the visible list has a `due_date`
- **THEN** the table SHALL include a `Due` column showing the date in `YYYY-MM-DD` format

#### Scenario: Table shows project column
- **WHEN** at least one task in the visible list has a `project`
- **THEN** the table SHALL include a `Project` column showing the project name

#### Scenario: No due dates or projects
- **WHEN** no visible tasks have `due_date` or `project` set
- **THEN** the `Due` and `Project` columns SHALL be omitted to preserve table width

### Requirement: Display due date and project in task show detail
The TUI SHALL display `due_date` and `project` in the full task detail view (or the equivalent details visible in the table or status line) when those fields are set.

#### Scenario: Due date visible in detail
- **WHEN** the selected task has a `due_date`
- **THEN** the due date SHALL be shown alongside other task metadata

#### Scenario: Project visible in detail
- **WHEN** the selected task has a `project`
- **THEN** the project name SHALL be shown alongside other task metadata

### Requirement: Filter tasks by project in TUI
The TUI filter mode SHALL support a `project:<name>` filter expression that shows only tasks whose `project` field matches the given name (case-insensitive).

#### Scenario: Filter by project
- **WHEN** the user presses `f`, types `project:Work`, and presses `Enter`
- **THEN** the task table SHALL show only tasks with `project` equal to `"Work"` (case-insensitive)

#### Scenario: No matching project
- **WHEN** the user applies a `project:` filter that matches no tasks
- **THEN** the table area SHALL display "No tasks match filter."

### Requirement: NLP message responses
When the NLP model determines that the user's query is unclear, conversational, does not map to a filter or update action, or is a question about the user's tasks, the system SHALL return a `Message(String)` action containing the model's plain-text response. The model SHALL use the task context (all fields: id, title, status, priority, tags, due_date, project) to answer task queries. The TUI SHALL display this message text in the status bar and return to Normal mode.

#### Scenario: Ambiguous query returns message
- **WHEN** the user enters NLP mode and types an ambiguous query like "hello"
- **THEN** the NLP module SHALL return `NlpAction::Message` with a helpful text response, and the TUI SHALL display that text in the status bar

#### Scenario: Task count query
- **WHEN** the user enters NLP mode and types "how many high-priority tasks do I have?"
- **THEN** the NLP module SHALL return `NlpAction::Message` with the count derived from the task context

#### Scenario: Task field query
- **WHEN** the user enters NLP mode and types "what projects do I have tasks in?"
- **THEN** the NLP module SHALL return `NlpAction::Message` listing the distinct project names from the task data

#### Scenario: Task summary query
- **WHEN** the user enters NLP mode and types "what's my oldest open task?"
- **THEN** the NLP module SHALL return `NlpAction::Message` with the answer based on task creation dates and status

#### Scenario: Unsupported action returns message
- **WHEN** the user enters NLP mode and requests an action the system cannot perform (e.g., "email my tasks to Alice")
- **THEN** the NLP module SHALL return `NlpAction::Message` explaining that the action is not supported

#### Scenario: Message display returns to normal mode
- **WHEN** the TUI receives an `NlpAction::Message` response
- **THEN** the TUI SHALL set the status message to the message text, return to Normal mode, and NOT modify any filters or tasks
