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
The TUI SHALL render a three-region layout in Normal mode: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints. In NlpChat mode, the TUI SHALL render a four-region layout: header (1 line), task table (top ~60%), chat panel (bottom ~40%), and input prompt (1 line).

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL use `theme::BAR_FG` foreground and `theme::BAR_BG` background, and the footer SHALL show keybinding hints with the same theme colors. The header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  T/W/M/Q:due  X:clr-due  R:recur  Tab:details  q:quit`

#### Scenario: Footer hints with detail panel visible
- **WHEN** the detail panel is visible in Normal mode
- **THEN** the footer SHALL show `Enter:edit` instead of `Enter:toggle` to indicate that Enter enters detail editing mode

#### Scenario: Footer hints in detail editing mode
- **WHEN** the TUI is in `EditingDetailPanel` mode
- **THEN** the footer SHALL show hints for editing: `j/k:field  Enter:save  Esc:cancel` (or equivalent context-sensitive hints)

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

#### Scenario: NlpChat split layout
- **WHEN** the TUI is in NlpChat mode
- **THEN** the layout SHALL split into header, task table (top portion), chat panel (bottom portion), and an input prompt line at the bottom

### Requirement: Task table display
The task table SHALL display columns for ID, status (checkbox), priority, title, and tags — matching the information shown by `task list`. The currently selected row SHALL be visually highlighted. When at least one visible task has a recurrence set, a recurrence indicator column SHALL be displayed showing `↻` for recurring tasks and blank for non-recurring tasks.

#### Scenario: Table with tasks
- **WHEN** the TUI loads a file containing tasks
- **THEN** the table SHALL display each task as a row with ID, a checkbox (checked for done), priority, title, and tags columns. Priority cells SHALL use colors from the `theme` module.

#### Scenario: Done task row styling
- **WHEN** a task with status Done is rendered in the table
- **THEN** all cells in that row SHALL use `theme::DONE_TEXT` foreground color, overriding priority coloring

#### Scenario: Overdue row styling
- **WHEN** an open task's due date is before today's date
- **THEN** the entire row SHALL be rendered in `theme::OVERDUE` foreground color and the status column SHALL display `[!]` instead of `[ ]`

#### Scenario: Empty task file
- **WHEN** the TUI loads a file with no tasks
- **THEN** the table area SHALL display a message like "No tasks. Press 'a' to add one."

#### Scenario: Description column shown conditionally
- **WHEN** at least one visible task has a non-empty description
- **THEN** the table SHALL include a "Desc" column after the Title column, displaying each task's description truncated to 30 characters with "…" appended if truncated

#### Scenario: Description column hidden
- **WHEN** no visible tasks have a description
- **THEN** the "Desc" column SHALL be omitted to preserve table width

#### Scenario: Description truncation
- **WHEN** a task's description exceeds 30 characters
- **THEN** the Desc cell SHALL display the first 29 characters followed by "…"

#### Scenario: Short description displayed in full
- **WHEN** a task's description is 30 characters or fewer
- **THEN** the Desc cell SHALL display the full description without truncation

#### Scenario: Recurrence indicator column shown
- **WHEN** at least one visible task has a recurrence set
- **THEN** the table SHALL include a recurrence column showing `↻` for recurring tasks and blank for non-recurring tasks

#### Scenario: Recurrence indicator column hidden
- **WHEN** no visible tasks have a recurrence set
- **THEN** the recurrence column SHALL be omitted to preserve table width

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
The user SHALL toggle a task between open and done by pressing `Enter` or `Space` on the selected task. The change SHALL be persisted to disk immediately. When the detail panel is visible, `Enter` SHALL enter detail editing mode instead of toggling completion; `Space` SHALL continue to toggle completion.

#### Scenario: Mark task as done
- **WHEN** the user presses `Enter` on an open task
- **THEN** the task's status SHALL change to done, the display SHALL update, and the file SHALL be saved

#### Scenario: Reopen a done task
- **WHEN** the user presses `Enter` on a done task
- **THEN** the task's status SHALL change to open, the display SHALL update, and the file SHALL be saved

#### Scenario: Enter with detail panel visible
- **WHEN** the detail panel is visible and the user presses `Enter` on a selected task
- **THEN** the TUI SHALL enter `EditingDetailPanel` mode instead of toggling the task's completion status

#### Scenario: Space with detail panel visible
- **WHEN** the detail panel is visible and the user presses `Space` on a selected task
- **THEN** the task's status SHALL toggle between open and done as usual (not entering edit mode)

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
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, EditingDetailPanel, ConfirmingDetailSave, EditingRecurrence, NlpChat, and ConfirmingNlp. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. The `i` key in Normal mode SHALL trigger a Todoist import (handled outside the mode system via the status message pattern).

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import as specified in the tui-todoist-import capability

#### Scenario: Colon key enters NlpChat mode
- **WHEN** the TUI is in Normal mode and the user presses `:`
- **THEN** the TUI SHALL enter NlpChat mode with an empty conversation history and display the split layout

#### Scenario: R key enters EditingRecurrence mode
- **WHEN** the TUI is in Normal mode and the user presses `R` with a task selected
- **THEN** the TUI SHALL enter EditingRecurrence mode and the footer SHALL display a text input prompt for recurrence (e.g., "Recurrence: _")

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

### Requirement: Task detail panel
The TUI SHALL provide a toggleable bottom panel that displays all fields of the currently selected task. The panel SHALL be toggled by pressing `Tab` in Normal mode. When visible, the layout SHALL split into a task table (top ~70%) and detail panel (bottom ~30%). The panel SHALL display: ID, title, status, priority, description (full text, wrapped), tags, due date, project, created timestamp, and updated timestamp. The panel content SHALL update as the user navigates between tasks.

#### Scenario: Toggle detail panel on
- **WHEN** the user presses `Tab` in Normal mode with the detail panel hidden
- **THEN** the layout SHALL split to show the task table above and the detail panel below, displaying all fields of the currently selected task

#### Scenario: Toggle detail panel off
- **WHEN** the user presses `Tab` in Normal mode with the detail panel visible
- **THEN** the detail panel SHALL be hidden and the table SHALL expand to fill the available space

#### Scenario: Panel updates on navigation
- **WHEN** the detail panel is visible and the user presses `j` or `k` to navigate
- **THEN** the panel SHALL update to show the details of the newly selected task

#### Scenario: Panel with no task selected
- **WHEN** the detail panel is visible and no task is selected (empty or fully filtered list)
- **THEN** the panel SHALL display "No task selected."

#### Scenario: Panel shows full description
- **WHEN** the detail panel is visible and the selected task has a description
- **THEN** the panel SHALL display the full description text (not truncated)

### Requirement: Detail panel inline editing
The TUI SHALL support inline editing of task fields within the detail panel. When the detail panel is visible and the user presses `Enter` on the selected task, the TUI SHALL enter `EditingDetailPanel` mode. The editable fields SHALL be: Title, Description, Priority, Status, Due Date, Project, and Tags. The user SHALL navigate between fields using `j`/`k` or `Tab`/`Shift-Tab`. Pressing `Esc` SHALL exit editing mode (with a dirty check if changes were made).

#### Scenario: Enter detail editing mode
- **WHEN** the detail panel is visible and the user presses `Enter` on a selected task
- **THEN** the TUI SHALL enter `EditingDetailPanel` mode, populate a draft from the current task, focus the first field (Title), and display the panel in edit layout with the focused field highlighted using `theme::HIGHLIGHT_BG`

#### Scenario: Navigate between fields
- **WHEN** the user is in `EditingDetailPanel` mode and presses `j`, `Down`, or `Tab`
- **THEN** the focus SHALL move to the next editable field (wrapping from Tags back to Title), saving the current input buffer value to the draft before moving

#### Scenario: Navigate fields backward
- **WHEN** the user is in `EditingDetailPanel` mode and presses `k`, `Up`, or `Shift-Tab`
- **THEN** the focus SHALL move to the previous editable field (wrapping from Title to Tags), saving the current input buffer value to the draft before moving

#### Scenario: Edit a text field
- **WHEN** a text field (Title, Description, Due Date, Project, Tags) is focused in `EditingDetailPanel` mode
- **THEN** the input buffer SHALL be loaded with the field's draft value, typing SHALL modify the buffer, and the panel SHALL render the current buffer with a cursor indicator

#### Scenario: Edit priority field
- **WHEN** the Priority field is focused and the user presses `c`, `h`, `m`, or `l`
- **THEN** the draft priority SHALL be set to critical, high, medium, or low respectively, and the display SHALL update immediately

#### Scenario: Toggle status field
- **WHEN** the Status field is focused and the user presses `Enter` or `Space`
- **THEN** the draft status SHALL toggle between open and done, and the display SHALL update immediately

#### Scenario: Exit editing with no changes
- **WHEN** the user presses `Esc` in `EditingDetailPanel` mode and the draft has no changes
- **THEN** the TUI SHALL exit to Normal mode immediately with no prompt

#### Scenario: Edit panel rendering
- **WHEN** the TUI is in `EditingDetailPanel` mode
- **THEN** the detail panel SHALL render each editable field on its own line with a label, the focused field SHALL be visually highlighted, and the footer SHALL show editing hints

### Requirement: Save-on-navigate confirmation
When the user attempts to leave the detail editing context with unsaved changes, the TUI SHALL prompt the user to save, discard, or cancel. This applies when pressing `Esc` to exit editing or when navigating to a different task while the draft is dirty.

#### Scenario: Dirty exit triggers confirmation
- **WHEN** the user presses `Esc` in `EditingDetailPanel` mode and the draft differs from the original task
- **THEN** the TUI SHALL enter `ConfirmingDetailSave` mode and display a footer prompt: "Unsaved changes. [s]ave  [d]iscard  [c]ancel"

#### Scenario: Save and exit
- **WHEN** the user presses `s` in `ConfirmingDetailSave` mode
- **THEN** the draft SHALL be applied to the task, the `updated` timestamp SHALL be set, the file SHALL be saved to disk, and the TUI SHALL exit to Normal mode

#### Scenario: Discard and exit
- **WHEN** the user presses `d` in `ConfirmingDetailSave` mode
- **THEN** the draft SHALL be discarded and the TUI SHALL exit to Normal mode with no changes persisted

#### Scenario: Cancel confirmation
- **WHEN** the user presses `c` or `Esc` in `ConfirmingDetailSave` mode
- **THEN** the TUI SHALL return to `EditingDetailPanel` mode with the draft intact

#### Scenario: Navigate away with dirty draft
- **WHEN** the detail panel is open with a dirty draft and the user presses `j` or `k` in Normal mode to navigate to a different task
- **THEN** the TUI SHALL enter `ConfirmingDetailSave` mode, storing the intended navigation direction. After save or discard, the navigation SHALL proceed. After cancel, the selection SHALL remain unchanged.

#### Scenario: Invalid due date on save
- **WHEN** the user saves and the due date field contains an invalid date string (not YYYY-MM-DD format and not empty)
- **THEN** the TUI SHALL display a status message indicating the invalid date, focus the due date field, and remain in `EditingDetailPanel` mode

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
When the NLP model determines that the user's query is unclear, conversational, does not map to a filter, update, or show_tasks action, or is a question about the user's tasks, the system SHALL return a `Message(String)` action containing the model's plain-text response. The model SHALL use the task context (all fields: id, title, status, priority, tags, due_date, project) to answer task queries. The TUI SHALL display this message text in the chat panel and remain in NlpChat mode.

#### Scenario: Ambiguous query returns message
- **WHEN** the user enters NLP mode and types an ambiguous query like "hello"
- **THEN** the NLP module SHALL return `NlpAction::Message` with a helpful text response, and the TUI SHALL display that text in the chat panel

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

#### Scenario: Message display stays in NlpChat mode
- **WHEN** the TUI receives an `NlpAction::Message` response while in NlpChat mode
- **THEN** the TUI SHALL append the message to the chat panel, clear the input buffer, and remain in NlpChat mode for follow-up input

### Requirement: Chat panel display
The TUI SHALL render a chat panel in NlpChat mode that displays the conversation history. User messages SHALL be prefixed with `> ` and visually distinguished from assistant messages. The chat panel SHALL auto-scroll to the most recent message when new messages are added.

#### Scenario: User message displayed
- **WHEN** the user submits a query in NlpChat mode
- **THEN** the chat panel SHALL display the user's message prefixed with `> `

#### Scenario: Assistant message displayed
- **WHEN** the model returns a Message or ShowTasks response
- **THEN** the chat panel SHALL display the assistant's text response below the user's message

#### Scenario: Auto-scroll on new message
- **WHEN** a new message is added to the conversation and the chat panel content exceeds the visible area
- **THEN** the chat panel SHALL scroll to show the most recent message

### Requirement: ShowTasks display in chat panel
When the model returns a `ShowTasks` action, the TUI SHALL display the accompanying text message followed by a compact task list in the chat panel. Each task SHALL be rendered with its ID, title, status, and priority. Task IDs that do not exist in the current task list SHALL be silently skipped.

#### Scenario: ShowTasks renders task list
- **WHEN** the model returns `ShowTasks` with task IDs `[1, 3, 7]` and text "Here are your overdue tasks:"
- **THEN** the chat panel SHALL display the text followed by a list showing each task's ID, title, status, and priority

#### Scenario: ShowTasks with invalid task IDs
- **WHEN** the model returns `ShowTasks` with task IDs `[1, 999]` and task 999 does not exist
- **THEN** the chat panel SHALL display task 1's details and silently skip task 999

#### Scenario: ShowTasks does not modify table filter
- **WHEN** the model returns a `ShowTasks` action
- **THEN** the main task table filter SHALL NOT be modified and the TUI SHALL remain in NlpChat mode

### Requirement: NlpChat conversation lifecycle
The conversation state SHALL be initialized when entering NlpChat mode and cleared when exiting. Pressing `Esc` in NlpChat mode SHALL clear the conversation history, restore the standard three-region layout, and return to Normal mode.

#### Scenario: Enter NlpChat mode
- **WHEN** the user presses `:` in Normal mode
- **THEN** the TUI SHALL enter NlpChat mode with empty conversation history and display the split layout with input prompt

#### Scenario: Stay in NlpChat after response
- **WHEN** the model returns any NlpAction (Filter, Update confirmation, Message, or ShowTasks)
- **THEN** the TUI SHALL remain in NlpChat mode with the input buffer cleared, ready for follow-up input

#### Scenario: Exit NlpChat mode
- **WHEN** the user presses `Esc` in NlpChat mode
- **THEN** the TUI SHALL clear conversation history, restore the three-region layout, and return to Normal mode

#### Scenario: NlpChat update confirmation
- **WHEN** the model returns an Update action while in NlpChat mode
- **THEN** the TUI SHALL enter ConfirmingNlp mode. After confirmation or cancellation, the TUI SHALL return to NlpChat mode (not Normal mode)

### Requirement: Quick due date assignment
The user SHALL set a task's due date directly from Normal mode using Shift-letter keybindings. Pressing `T` SHALL set the due date to today, `W` to one week from today (+7 days), `M` to one month from today, `Q` to three months from today (next quarter), and `X` SHALL clear the due date (set to None). The change SHALL be persisted to disk immediately. A status message SHALL confirm the new date or clearance. All due date keys SHALL be no-ops when no task is selected.

#### Scenario: Set due date to today
- **WHEN** the user presses `T` on a selected task
- **THEN** the task's `due_date` SHALL be set to today's date, the `updated` timestamp SHALL be set, the file SHALL be saved, and a status message "Due: YYYY-MM-DD" SHALL be displayed

#### Scenario: Set due date to next week
- **WHEN** the user presses `W` on a selected task
- **THEN** the task's `due_date` SHALL be set to 7 days from today, the file SHALL be saved, and a status message SHALL confirm the date

#### Scenario: Set due date to next month
- **WHEN** the user presses `M` on a selected task
- **THEN** the task's `due_date` SHALL be set to one calendar month from today (clamped to month-end if needed), the file SHALL be saved, and a status message SHALL confirm the date

#### Scenario: Set due date to next quarter
- **WHEN** the user presses `Q` on a selected task
- **THEN** the task's `due_date` SHALL be set to three calendar months from today, the file SHALL be saved, and a status message SHALL confirm the date

#### Scenario: Clear due date
- **WHEN** the user presses `X` on a selected task that has a due date
- **THEN** the task's `due_date` SHALL be set to None, the file SHALL be saved, and a status message "Due date cleared" SHALL be displayed

#### Scenario: No-op when list is empty
- **WHEN** the user presses `T`, `W`, `M`, `Q`, or `X` and no task is selected (empty or fully filtered list)
- **THEN** the TUI SHALL remain in Normal mode with no change

### Requirement: Quick recurrence setting via R keybinding
The user SHALL press `R` (shift-R) in Normal mode to enter `EditingRecurrence` mode for the selected task. The footer SHALL display a text input prompt. The user types a natural language recurrence pattern (e.g., "weekly", "every third thursday", "none") and presses `Enter` to submit. The input SHALL be sent to the NLP with a focused recurrence-parsing prompt. On success, the recurrence SHALL be set (or cleared if "none") on the task and saved to disk. Pressing `Esc` SHALL cancel. The `R` key SHALL be a no-op when no task is selected.

#### Scenario: Set weekly recurrence via R
- **WHEN** the user presses `R`, types "weekly", and presses `Enter`
- **THEN** the selected task's recurrence SHALL be set to `Interval(Weekly)`, the file SHALL be saved, and a status message "Recurrence: weekly" SHALL be displayed

#### Scenario: Set nth weekday recurrence via R
- **WHEN** the user presses `R`, types "every third thursday", and presses `Enter`
- **THEN** the selected task's recurrence SHALL be set to `NthWeekday { n: 3, weekday: Thu }`, the file SHALL be saved, and a status message "Recurrence: monthly:3:thu" SHALL be displayed

#### Scenario: Clear recurrence via R
- **WHEN** the user presses `R`, types "none", and presses `Enter`
- **THEN** the selected task's recurrence SHALL be set to `None`, the file SHALL be saved, and a status message "Recurrence cleared" SHALL be displayed

#### Scenario: Cancel recurrence edit
- **WHEN** the user presses `R` and then presses `Esc`
- **THEN** the task's recurrence SHALL remain unchanged and the TUI SHALL return to Normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `R` and no task is selected
- **THEN** the TUI SHALL remain in Normal mode with no change

#### Scenario: NLP parse error
- **WHEN** the user types an unrecognizable recurrence pattern and presses `Enter`
- **THEN** the TUI SHALL display a status message with the error and return to Normal mode

### Requirement: Task detail panel recurrence display
The detail panel SHALL display the task's recurrence field. For recurring tasks, it SHALL show a human-readable format (e.g., "Weekly", "Monthly (3rd Thu)"). For non-recurring tasks, it SHALL show "-".

#### Scenario: Detail panel shows recurrence
- **WHEN** the detail panel is visible and the selected task has `recurrence: Some(Interval(Weekly))`
- **THEN** the detail panel SHALL display "Recurrence: Weekly"

#### Scenario: Detail panel shows nth weekday recurrence
- **WHEN** the detail panel is visible and the selected task has `recurrence: Some(NthWeekday { n: 3, weekday: Thu })`
- **THEN** the detail panel SHALL display "Recurrence: Monthly (3rd Thu)"

#### Scenario: Detail panel shows no recurrence
- **WHEN** the detail panel is visible and the selected task has `recurrence: None`
- **THEN** the detail panel SHALL display "Recurrence: -"
