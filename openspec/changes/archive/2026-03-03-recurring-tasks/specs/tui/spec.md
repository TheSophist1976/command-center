## MODIFIED Requirements

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
